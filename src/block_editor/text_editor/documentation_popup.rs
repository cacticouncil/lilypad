use std::collections::HashMap;

use egui::{Align2, Painter, Pos2, Rect, Response, Stroke, Ui, Vec2, Widget};
use ropey::Rope;

use crate::{
    block_editor::{MonospaceFont, OUTER_PAD, TOTAL_TEXT_X_OFFSET},
    lsp::documentation::{self, Documentation},
    theme,
    util_widgets::SelectableRow,
    vscode,
};

pub struct DocumentationPopup {
    message: String,
    line: usize,
    col: usize,
}

impl DocumentationPopup {
    pub fn new() -> Self {
        DocumentationPopup {
            message: String::from(" "),
            line: 0,
            col: 0,
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
                rect.min * 2.0,
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

    pub fn get_line(&self) -> f32 {
        self.line as f32
    }

    pub fn get_col(&self) -> f32 {
        self.col as f32
    }

    pub fn request_hover(&mut self, line: usize, col: usize) {
        vscode::request_hover(line, col);
        self.line = line;
        self.col = col;
    }
    pub fn calc_origin(
        &self,
        documentation: &Documentation,
        offset: Vec2,
        padding: &[f32],
        font: &MonospaceFont,
    ) -> Vec2 {
        // find height
        let height = font.size.y;

        // find the vertical start by finding top of line and then subtracting box size
        let total_padding: f32 = padding
            .iter()
            .take(documentation.range.start.line + 1)
            .sum();
        let documentation_start = OUTER_PAD
            + total_padding
            + (documentation.range.start.line as f32 * font.size.y)
            + offset.y;
        let y = if height > documentation_start {
            // put it below the line if there isn't enough room above
            documentation_start + font.size.y
        } else {
            documentation_start - height
        };

        // find the horizontal start
        let x =
            TOTAL_TEXT_X_OFFSET + (documentation.range.start.col as f32 * font.size.x) + offset.x;

        Vec2::new(x * 2.0, y * 2.0)
    }

    pub fn calc_size(&self, documentation: &Documentation, font: &MonospaceFont) -> Pos2 {
        // find dimensions
        let num_newlines = documentation.message.chars().filter(|&c| c == '\n').count();
        let height = (num_newlines + 1) as f32 * font.size.y;
        let text_len = documentation.message.chars().filter(|&c| c != '`').count();
        let width = text_len as f32 * font.size.x;

        Pos2::new(width, height)
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
