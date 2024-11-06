use std::collections::HashMap;

use egui::{Align2, Painter, Pos2, Rect, Response, Stroke, Ui, Vec2, Widget};
use ropey::Rope;

use crate::{
    block_editor::{MonospaceFont, OUTER_PAD, TOTAL_TEXT_X_OFFSET},
    lsp::documentation::Documentation,
    theme,
    util_widgets::SelectableRow,
    vscode,
};

pub struct DocumentationPopup {
    message: String,
}

impl DocumentationPopup {
    pub fn new() -> Self {
        DocumentationPopup {
            message: String::from(" "),
        }
    }
    pub fn widget<'a>(
        &'a mut self,
        documentation: &'a Documentation,
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
                &documentation.message,
                font.id.clone(),
                theme::syntax::DEFAULT,
            );

            response
        }
    }

    pub fn set_hover(&mut self, message: String) {
        self.message = message;
    }

    pub fn get_hover(&self) -> &str {
        &self.message
    }

    pub fn request_hover(&self, line: usize, col: usize) {
        vscode::request_hover(line, col);
    }
}

impl Documentation {
    pub fn draw(
        &self,
        padding: &[f32],
        source: &Rope,
        offset: Vec2,
        font: &MonospaceFont,
        painter: &Painter,
    ) {
        let range = self.range.ordered();
        let line_ranges = range.individual_lines(source);

        let mut total_padding: f32 = padding.iter().take(range.start.line).sum();

        for line_range in line_ranges {
            let line_num = line_range.start.line;

            total_padding += padding[line_num];

            // find bottom of current line
            let y = total_padding + ((line_num + 1) as f32 * font.size.y) + OUTER_PAD;

            // find the start and end of the line
            let x = TOTAL_TEXT_X_OFFSET + (line_range.start.col as f32 * font.size.x);
            let width = (line_range.end.col - line_range.start.col) as f32 * font.size.x;
        }
    }
}
