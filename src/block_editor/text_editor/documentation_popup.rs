use egui::{Align2, Painter, Pos2, Rect, Response, Stroke, Ui, Vec2, Widget};
use ropey::Rope;

use crate::{
    block_editor::{MonospaceFont, OUTER_PAD, TOTAL_TEXT_X_OFFSET},
    lsp::documentation::{Documentation, VSCodeHoverItem},
    theme,
    util_widgets::SelectableRow,
    vscode,
};

pub struct DocumentationPopup {
    message: String,
    cursor: Pos2,
}

impl DocumentationPopup {
    pub fn new() -> Self {
        DocumentationPopup {
            message: String::from(" "),
            cursor: Pos2::new(0.0, 0.0),
        }
    }

    pub fn set_hover(&mut self, message: Vec<VSCodeHoverItem>) {}

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

            // draw line
        }
    }
}
