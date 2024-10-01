use egui::{Painter, Pos2, Rect, Response, Stroke, Ui, Vec2, Widget};
use ropey::Rope;

use crate::{
    block_editor::{MonospaceFont, OUTER_PAD, TOTAL_TEXT_X_OFFSET},
    lsp::documentation::Documentation,
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

    pub fn set_hover(&mut self, message: String) {
        self.message = message;
    }

    pub fn request_hover(&self, line: usize, col: usize) {
        vscode::request_hover(line, col);
    }
}

impl Documentation {}
