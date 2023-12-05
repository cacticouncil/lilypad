use druid::{
    piet::{PietTextLayout, Text, TextLayoutBuilder},
    Color, Event, MouseButton, PaintCtx, Point, Rect, RenderContext, Size, Widget,
};
use ropey::Rope;

use crate::{
    block_editor::{
        commands,
        rope_ext::RopeSliceExt,
        text_range::{TextEdit, TextPoint, TextRange},
        EditorModel, FONT_FAMILY, FONT_HEIGHT, FONT_SIZE, FONT_WIDTH, TOTAL_TEXT_X_OFFSET,
    },
    lsp::completion::VSCodeCompletionItem,
    theme, vscode,
};

pub struct CompletionPopup {
    completions: Vec<VSCodeCompletionItem>,
    selection: usize,
    text_cursor: TextPoint,
}

impl CompletionPopup {
    pub fn new() -> Self {
        Self {
            completions: vec![],
            selection: 0,
            text_cursor: TextPoint::ZERO,
        }
    }

    pub fn calc_origin(&self, padding: &[f64], cursor: TextPoint) -> Point {
        // find the bottom of the current selection
        let total_padding: f64 = padding.iter().take(cursor.row + 1).sum();
        let y = (cursor.row as f64 + 2.0) * *FONT_HEIGHT.get().unwrap() + total_padding;
        let x = (cursor.col as f64) * *FONT_WIDTH.get().unwrap() + TOTAL_TEXT_X_OFFSET;
        Point::new(x, y)
    }

    pub fn clear(&mut self) {
        self.completions.clear();
    }

    pub fn request_completions(&mut self, source: &Rope, selection: TextRange) {
        // only request completions when selection is just a cursor
        if !selection.is_cursor() {
            return;
        }
        let cursor = selection.start;

        // if we are at the start of the line, do not do anything
        // return does not trigger, but backspace does
        if source.line(cursor.row).whitespace_at_start() == cursor.col {
            return;
        }

        // set the cursor so we can filter results using it
        self.text_cursor = cursor;

        // request
        vscode::request_completions(cursor.row, cursor.col)
    }

    fn calc_size(&self) -> Size {
        if self.completions.is_empty() {
            return Size::ZERO;
        };

        let height = *FONT_HEIGHT.get().unwrap() * self.completions.len() as f64;

        let max_label_len: usize = self
            .completions
            .iter()
            .map(|fix| fix.name().chars().count())
            .max()
            .unwrap_or(0);
        let width = max_label_len as f64 * FONT_WIDTH.get().unwrap();

        Size::new(width, height)
    }

    fn apply_selected_completion(&mut self, source: &Rope, ctx: &mut druid::EventCtx) {
        if let Some(completion) = self.completions.get(self.selection) {
            let text_edit = self.edit_for_completion(completion.text_to_insert(), source);
            ctx.submit_command(commands::APPLY_EDIT.with(text_edit));

            self.completions.clear();
            ctx.request_layout();
            ctx.request_paint();
        }
    }

    fn edit_for_completion(&self, completion: String, source: &Rope) -> TextEdit {
        // select the word before the cursor
        // (so what was typed so far is replaced by the completion)
        let range = self.range_of_word_before_cursor(source);

        // indent newlines with the current indentation level
        let mut text = completion.clone();
        if text.contains('\n') {
            let indent_count = source.line(self.text_cursor.row).whitespace_at_start();
            let newline_with_indent = &format!("\n{}", " ".repeat(indent_count));
            text = text.replace('\n', newline_with_indent);
        }

        TextEdit { text, range }
    }

    fn range_of_word_before_cursor(&self, source: &Rope) -> TextRange {
        let mut start = self.text_cursor;
        let curr_line = source.line(start.row);
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

impl Widget<EditorModel> for CompletionPopup {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &Event,
        data: &mut EditorModel,
        _env: &druid::Env,
    ) {
        let font_height = *FONT_HEIGHT.get().unwrap();

        match event {
            Event::MouseDown(mouse) if mouse.button == MouseButton::Left => {
                if !self.completions.is_empty() {
                    // find fix for row clicked on
                    let row_clicked = (mouse.pos.y / font_height) as usize;

                    // catch clicking out of bounds
                    if row_clicked >= self.completions.len() {
                        return;
                    }

                    // highlight the row clicked on
                    self.selection = row_clicked;

                    ctx.request_paint();
                }
                ctx.set_handled();
            }
            Event::MouseUp(mouse) if mouse.button == MouseButton::Left => {
                // trigger the completion set by mouse down
                if !self.completions.is_empty() {
                    self.apply_selected_completion(&data.source.lock().unwrap(), ctx);
                }

                ctx.set_handled();
            }
            Event::KeyDown(key) => {
                if !self.completions.is_empty() {
                    match key.key {
                        druid::keyboard_types::Key::ArrowUp => {
                            self.selection = self.selection.saturating_sub(1);
                            ctx.request_paint();
                            ctx.set_handled();
                        }
                        druid::keyboard_types::Key::ArrowDown => {
                            self.selection =
                                std::cmp::min(self.selection + 1, self.completions.len() - 1);
                            ctx.request_paint();
                            ctx.set_handled();
                        }
                        druid::keyboard_types::Key::Enter => {
                            self.apply_selected_completion(&data.source.lock().unwrap(), ctx);
                            ctx.set_handled();
                        }
                        druid::keyboard_types::Key::Escape => {
                            self.completions.clear();
                            ctx.request_layout();
                            ctx.request_paint();
                            ctx.set_handled();
                        }
                        _ => {}
                    }
                }
            }
            Event::Command(command) => {
                if let Some(completions) = command.get(commands::SET_COMPLETIONS) {
                    // clear existing completions
                    self.completions.clear();

                    // reset the selection because there are new completions
                    self.selection = 0;

                    // if there are too many completions, it is unfocused so shouldn't show at all
                    if completions.len() > 100 {
                        ctx.set_handled();
                        return;
                    }

                    // move completions that start with what has been typed so far to the top of the list
                    // split into two lists and then combine them to maintain ordering between elements within the two groups
                    let source = &data.source.lock().unwrap();
                    let prefix_range = self.range_of_word_before_cursor(source);
                    let prefix = source
                        .line(prefix_range.start.row)
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

                    ctx.request_layout();
                    ctx.request_paint();
                    ctx.set_handled();
                }
            }

            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut druid::LifeCycleCtx,
        _event: &druid::LifeCycle,
        _data: &EditorModel,
        _env: &druid::Env,
    ) {
    }

    fn update(
        &mut self,
        _ctx: &mut druid::UpdateCtx,
        _old_data: &EditorModel,
        _data: &EditorModel,
        _env: &druid::Env,
    ) {
    }

    fn layout(
        &mut self,
        _ctx: &mut druid::LayoutCtx,
        _bc: &druid::BoxConstraints,
        _data: &EditorModel,
        _env: &druid::Env,
    ) -> Size {
        self.calc_size()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &EditorModel, _env: &druid::Env) {
        // TODO: look good

        if !self.completions.is_empty() {
            // set background color
            let rect = ctx.size().to_rect();
            ctx.fill(rect, &theme::POPUP_BACKGROUND);

            // draw completions
            let font_height = *FONT_HEIGHT.get().unwrap();
            for (line, completion) in self.completions.iter().enumerate() {
                // highlight background if selected
                if line == self.selection {
                    let rect = Rect::from_origin_size(
                        Point::new(0.0, line as f64 * font_height),
                        Size::new(ctx.size().width, font_height),
                    );
                    ctx.fill(rect, &theme::SELECTION);
                }

                let pos = Point::new(0.0, line as f64 * font_height);
                let color = completion.color();
                let layout = make_text_layout(&completion.name(), color, ctx);
                ctx.draw_text(&layout, pos);
            }
        }
    }
}

fn make_text_layout(text: &str, color: Color, ctx: &mut PaintCtx) -> PietTextLayout {
    ctx.text()
        .new_text_layout(text.to_string())
        .font(
            FONT_FAMILY.get().unwrap().clone(),
            *FONT_SIZE.get().unwrap(),
        )
        .text_color(color)
        .build()
        .unwrap()
}
