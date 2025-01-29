use crate::{
    block_editor::{MonospaceFont, OUTER_PAD, TOTAL_TEXT_X_OFFSET},
    lsp::documentation::BlockType,
    lsp::documentation::Documentation,
    theme,
};
use egui::{Align2, Color32, Painter, Pos2, Response, Ui, Vec2, Widget};
use ropey::Rope;

pub struct DocumentationPopup {}

impl DocumentationPopup {
    pub fn new() -> Self {
        Self {}
    }
    pub fn widget<'a>(
        &'a mut self,
        documentation: &'a Documentation,
        font: &'a MonospaceFont,
    ) -> impl Widget + 'a {
        move |ui: &mut Ui| -> Response {
            let (id, rect) = ui.allocate_space(ui.available_size());
            let response = ui.interact(rect, id, egui::Sense::hover());
            // set background color
            ui.painter().rect_filled(rect, 0.0, theme::POPUP_BACKGROUND);
            let mut current_y = rect.min.y; // Start at the top of the rect
                                            //let painter = ui.painter();
            for i in &documentation.hover_info.all_blocks {
                let text = &i.0;
                //  let language: Language = Language::for_file(&self.hover_info.language);
                // Measure the text size to get its height
                let text_size = ui.painter().layout(
                    (text).to_string(),
                    font.id.clone(),
                    theme::syntax::DEFAULT,
                    rect.max.x - rect.min.x, // Use the available width for wrapping
                );

                // Render the text block
                match i.1 {
                    BlockType::CodeBlock => {
                        /* let mut colored_text = Source::new(Rope::from(text as &str), language);
                        let mut text_drawer = TextDrawer::new();
                        text_drawer.highlight_source(&mut colored_text);
                        text_drawer.draw(
                            &[1.0],
                            Vec2::new(rect.min.x, rect.min.y),
                            None,
                            font,
                            painter,
                        );*/
                        ui.painter().text(
                            egui::pos2(rect.min.x, current_y),
                            Align2::LEFT_TOP,
                            text,
                            font.id.clone(),
                            Color32::from_rgb(170, 215, 255),
                        );
                    }
                    BlockType::RegularBlock => {
                        ui.painter().text(
                            egui::pos2(rect.min.x, current_y),
                            Align2::LEFT_TOP,
                            text,
                            font.id.clone(),
                            theme::syntax::DEFAULT,
                        );
                    }
                }

                // Move down by the height of the block
                current_y += text_size.size().y; // Add spacing between blocks
            }

            response
        }
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
        let max_hover_width = 500.0;
        let max_hover_height = 400.0;
        // find dimensions
        let num_newlines = documentation.message.chars().filter(|&c| c == '\n').count()
            + &documentation.hover_info.all_blocks.len();
        let height = (num_newlines + 1) as f32 * font.size.y;
        let longest_line_len = documentation
            .message
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        let width = (longest_line_len + 1) as f32 * font.size.x;
        let width = width.min(max_hover_width);
        let height = height.min(max_hover_height);
        Vec2::new(width, height)
    }
}

impl Documentation {
    pub fn draw(
        &self,
        padding: &[f32],
        source: &Rope,
        _offset: Vec2,
        _font: &MonospaceFont,
        _painter: &Painter,
    ) {
        let range = self.range.ordered();
        let line_ranges = range.individual_lines(source);

        let mut _total_padding: f32 = padding.iter().take(range.start.line).sum();

        for line_range in line_ranges {
            let line_num = line_range.start.line;

            _total_padding += padding[line_num];

            // find bottom of current line
        }
    }
}
