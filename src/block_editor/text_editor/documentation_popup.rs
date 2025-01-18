use crate::block_editor::text_range::{TextPoint, TextRange};
use egui::{Align2, Painter, Pos2, Rect, Response, Stroke, Ui, Vec2, Widget};
use ropey::Rope;

use crate::{
    block_editor::{MonospaceFont, OUTER_PAD, TOTAL_TEXT_X_OFFSET},
    lsp::documentation::{self, Documentation},
    theme, vscode,
};

pub struct DocumentationPopup {
    message: String,
    range: TextRange,
}

impl DocumentationPopup {
    pub fn new() -> Self {
        DocumentationPopup {
            message: String::from(" "),
            range: TextRange::new(TextPoint::new(0, 0), TextPoint::new(0, 0)),
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

    pub fn set_hover(&mut self, message: String, range: TextRange) {
        self.message = message;
        self.range = range;
    }

    pub fn request_hover(&mut self, line: usize, col: usize) {
        vscode::request_hover(line, col);
    }

    pub fn calc_origin(
        &self,
        documentation: &Documentation,
        offset: Vec2,
        padding: &[f32],
        font: &MonospaceFont,
    ) -> Pos2 {
        let mut height = font.size.y;
        height += (documentation.message.len()) as f32 * font.size.y;

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

        Pos2::new(x, y)
    }

    pub fn calc_size(&self, documentation: &Documentation, font: &MonospaceFont) -> Vec2 {
        // find dimensions
        let num_newlines = documentation.message.chars().filter(|&c| c == '\n').count();
        let height = (num_newlines + 1) as f32 * font.size.y;
        let longest_line_len = documentation
            .message
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        let width = (longest_line_len + 1) as f32 * font.size.x;

        Vec2::new(width, height)
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
        }
    }
}
