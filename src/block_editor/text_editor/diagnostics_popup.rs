use std::collections::HashMap;

use egui::{Align2, Painter, Pos2, Rect, Response, Stroke, Ui, Vec2, Widget};
use ropey::Rope;

use crate::{
    block_editor::{blocks::Padding, MonospaceFont, OUTER_PAD, TOTAL_TEXT_X_OFFSET},
    lsp::diagnostics::{Diagnostic, VSCodeCodeAction},
    theme,
    util_widgets::SelectableRow,
};

pub struct DiagnosticPopup {
    fixes: HashMap<usize, Vec<VSCodeCodeAction>>,
}

impl DiagnosticPopup {
    pub fn new() -> Self {
        DiagnosticPopup {
            fixes: HashMap::new(),
        }
    }

    pub fn widget<'a>(
        &'a mut self,
        diagnostic: &'a Diagnostic,
        font: &'a MonospaceFont,
    ) -> impl Widget + 'a {
        move |ui: &mut Ui| -> Response {
            let (id, rect) = ui.allocate_space(ui.available_size());
            let response = ui.interact(rect, id, egui::Sense::click_and_drag());

            // set background color
            ui.painter().rect_filled(rect, 0.0, theme::POPUP_BACKGROUND);

            // draw message
            ui.painter().text(
                rect.min,
                Align2::LEFT_TOP,
                &diagnostic.message,
                font.id.clone(),
                theme::syntax::DEFAULT,
            );

            // draw fixes
            if let Some(fixes) = &self.fixes.get(&diagnostic.id) {
                for (line, fix) in fixes.iter().enumerate() {
                    let fix_response = ui.put(
                        Rect::from_min_size(
                            rect.min + Vec2::new(0.0, (line + 1) as f32 * font.size.y),
                            Vec2::new(rect.width(), font.size.y),
                        ),
                        SelectableRow::new(
                            &fix.title,
                            theme::syntax::FUNCTION,
                            false,
                            font.id.clone(),
                        ),
                    );
                    if fix_response.clicked() {
                        fix.run();
                    }
                }
            } else {
                // if no fixes, request them
                diagnostic.request_fixes();
            }

            response
        }
    }

    pub fn set_fixes(&mut self, id: usize, fixes: Vec<VSCodeCodeAction>) {
        self.fixes.insert(id, fixes);
    }

    pub fn clear_fixes(&mut self) {
        self.fixes.clear();
    }

    pub fn calc_origin(
        &self,
        diagnostic: &Diagnostic,
        offset: Vec2,
        padding: &Padding,
        font: &MonospaceFont,
    ) -> Pos2 {
        // find height
        let mut height = font.size.y;
        if let Some(fixes) = &self.fixes.get(&diagnostic.id) {
            height += fixes.len() as f32 * font.size.y;
        }

        // find the vertical start by finding top of line and then subtracting box size
        let total_padding: f32 = padding.cumulative(diagnostic.range.start.line + 1);
        let diagnostic_start = OUTER_PAD
            + total_padding
            + (diagnostic.range.start.line as f32 * font.size.y)
            + offset.y;
        let y = if height > diagnostic_start {
            // put it below the line if there isn't enough room above
            diagnostic_start + font.size.y
        } else {
            diagnostic_start - height
        };

        // find the horizontal start
        let x = TOTAL_TEXT_X_OFFSET + (diagnostic.range.start.col as f32 * font.size.x) + offset.x;

        Pos2::new(x, y)
    }

    pub fn calc_size(&self, diagnostic: &Diagnostic, font: &MonospaceFont) -> Vec2 {
        // find dimensions
        let mut height = font.size.y;
        if let Some(fixes) = &self.fixes.get(&diagnostic.id) {
            height += fixes.len() as f32 * font.size.y;
        }

        let text_len = diagnostic.message.chars().count();
        let max_fix_len: usize = if let Some(fixes) = &self.fixes.get(&diagnostic.id) {
            fixes
                .iter()
                .map(|fix| fix.title.chars().count())
                .max()
                .unwrap_or(0)
        } else {
            0
        };
        let width = usize::max(text_len, max_fix_len) as f32 * font.size.x;

        Vec2::new(width, height)
    }
}

impl Diagnostic {
    pub fn draw(
        &self,
        padding: &Padding,
        source: &Rope,
        offset: Vec2,
        font: &MonospaceFont,
        painter: &Painter,
    ) {
        let range = self.range.ordered();
        let line_ranges = range.individual_lines(source);

        for line_range in line_ranges {
            let line_num = line_range.start.line;

            // find bottom of current line
            let y =
                padding.cumulative(line_num) + ((line_num + 1) as f32 * font.size.y) + OUTER_PAD;

            // find the start and end of the line
            let x = TOTAL_TEXT_X_OFFSET + (line_range.start.col as f32 * font.size.x);
            let width = (line_range.end.col - line_range.start.col) as f32 * font.size.x;

            // draw line
            painter.line_segment(
                [Pos2::new(x, y) + offset, Pos2::new(x + width, y) + offset],
                Stroke::new(2.0, self.severity.color()),
            );
        }
    }
}
