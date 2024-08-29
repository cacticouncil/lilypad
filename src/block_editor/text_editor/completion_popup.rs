use std::borrow::Cow;

use egui::{Pos2, Rect, Response, Sense, Ui, Vec2, Widget};
use ropey::Rope;

use super::TextEdit;
use crate::{
    block_editor::{
        rope_ext::RopeSliceExt,
        text_range::{TextPoint, TextRange},
        MonospaceFont, TOTAL_TEXT_X_OFFSET,
    },
    lsp::completion::VSCodeCompletionItem,
    theme,
    util_widgets::SelectableRow,
    vscode,
};

pub struct CompletionPopup {
    completions: Vec<VSCodeCompletionItem>,
    selection: usize,
    text_cursor: TextPoint,
    completion_triggered: bool,
}

impl CompletionPopup {
    pub fn new() -> Self {
        Self {
            completions: vec![],
            selection: 0,
            text_cursor: TextPoint::ZERO,
            completion_triggered: false,
        }
    }

    pub fn widget<'a>(
        &'a mut self,
        edit: &'a mut Option<TextEdit<'static>>,
        source: &'a Rope,
        font: &'a MonospaceFont,
    ) -> impl Widget + 'a {
        move |ui: &mut Ui| -> Response {
            let (id, rect) = ui.allocate_space(ui.available_size());
            let offset = rect.min.to_vec2();
            let response = ui.interact(rect, id, Sense::hover());

            if self.completion_triggered {
                *edit = self.edit_for_selected(source);
                self.completion_triggered = false;
                return response;
            }

            let painter = ui.painter();

            // draw background
            painter.rect_filled(rect, 0.0, theme::POPUP_BACKGROUND);

            // draw completions
            for (line, completion) in self.completions.iter().enumerate() {
                let rect = Rect::from_min_size(
                    Pos2::new(0.0, line as f32 * font.size.y) + offset,
                    Vec2::new(rect.width(), font.size.y),
                );
                let row_response = ui.put(
                    rect,
                    SelectableRow::new(
                        &completion.name(),
                        completion.color(),
                        line == self.selection,
                        font.id.clone(),
                    ),
                );
                if row_response.clicked() {
                    self.selection = line;
                    *edit = self.edit_for_selected(source);
                }
            }

            if response.contains_pointer() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            response
        }
    }

    pub fn clear(&mut self) {
        self.completions.clear();
    }

    pub fn request_completions(&mut self, source: &Rope, selection: TextRange) {
        // clear existing completions
        self.completions.clear();

        // only request completions when selection is just a cursor
        if !selection.is_cursor() {
            return;
        }
        let cursor = selection.start;

        // if we are at the start of the line, do not do anything
        // return does not trigger, but backspace does
        if source.line(cursor.line).whitespace_at_start() == cursor.col {
            return;
        }

        // set the cursor so we can filter results using it
        self.text_cursor = cursor;

        // request
        vscode::request_completions(cursor.line, cursor.col)
    }

    pub fn has_completions(&self) -> bool {
        !self.completions.is_empty()
    }

    pub fn select_next(&mut self) {
        self.selection = (self.selection + 1) % self.completions.len();
    }

    pub fn select_prev(&mut self) {
        self.selection = if self.selection > 0 {
            self.selection - 1
        } else {
            self.completions.len() - 1
        };
    }

    // on the next run of widget, immediately return the current selection
    pub fn trigger_completion(&mut self) {
        self.completion_triggered = true;
    }

    pub fn calc_size(&self, font: &MonospaceFont) -> Vec2 {
        if self.completions.is_empty() {
            return Vec2::ZERO;
        };

        let height = font.size.y * self.completions.len() as f32;

        let max_label_len: usize = self
            .completions
            .iter()
            .map(|fix| fix.name().chars().count())
            .max()
            .unwrap_or(0);
        let width = max_label_len as f32 * font.size.x;

        Vec2::new(width, height)
    }

    pub fn calc_origin(&self, cursor: TextPoint, padding: &[f32], font: &MonospaceFont) -> Pos2 {
        // find the bottom of the current selection
        let total_padding: f32 = padding.iter().take(cursor.line + 1).sum();
        let y = (cursor.line as f32 + 2.0) * font.size.y + total_padding;
        let x = (cursor.col as f32) * font.size.x + TOTAL_TEXT_X_OFFSET;
        Pos2::new(x, y)
    }

    pub fn set_completions(&mut self, completions: &[VSCodeCompletionItem], source: &Rope) {
        // clear existing completions
        self.completions.clear();

        // reset the selection because there are new completions
        self.selection = 0;

        // if there are too many completions, it is unfocused so shouldn't show at all
        if completions.len() > 100 {
            return;
        }

        // move completions that start with what has been typed so far to the top of the list
        // split into two lists and then combine them to maintain ordering between elements within the two groups
        let prefix_range = self.range_of_word_before_cursor(source);
        let prefix = source
            .line(prefix_range.start.line)
            .slice(prefix_range.start.col..prefix_range.end.col)
            .to_string()
            .to_lowercase();

        let mut has_prefix = vec![];
        let mut no_prefix = vec![];
        for completion in completions {
            if completion.name().to_lowercase().starts_with(&prefix) {
                has_prefix.push(completion.clone());
            } else {
                no_prefix.push(completion.clone());
            }
        }
        self.completions.extend(has_prefix);
        self.completions.extend(no_prefix);

        // only show the top 10 completions
        self.completions.truncate(10);
    }

    pub fn edit_for_selected(&self, source: &Rope) -> Option<TextEdit<'static>> {
        if let Some(completion) = self.completions.get(self.selection) {
            // select the word before the cursor
            // (so what was typed so far is replaced by the completion)
            let range = self.range_of_word_before_cursor(source);

            // indent newlines with the current indentation level
            let mut text = completion.text_to_insert();
            if text.contains('\n') {
                let indent_count = source.line(self.text_cursor.line).whitespace_at_start();
                let newline_with_indent = &format!("\n{}", " ".repeat(indent_count));
                text = text.replace('\n', newline_with_indent);
            }

            Some(TextEdit::new(Cow::Owned(text), range))
        } else {
            None
        }
    }

    fn range_of_word_before_cursor(&self, source: &Rope) -> TextRange {
        let mut start = self.text_cursor;
        let curr_line = source.line(start.line);
        start.col = start.col.min(curr_line.len_chars());
        while start.col > 0 {
            let c = curr_line.char(start.col - 1);
            if !(c.is_alphanumeric() || c == '_') {
                break;
            }
            start.col -= 1;
        }
        TextRange::new(start, self.text_cursor)
    }
}
