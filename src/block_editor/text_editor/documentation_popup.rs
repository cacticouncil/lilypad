use std::num;

use crate::{
    block_editor::{blocks::Padding, MonospaceFont, OUTER_PAD, TOTAL_TEXT_X_OFFSET},
    lsp::documentation::BlockType,
    lsp::documentation::Documentation,
    theme,
};
use egui::{Align2, Color32, Painter, Pos2, Response, RichText, Ui, Vec2, Widget};
use ropey::Rope;

pub struct DocumentationPopup {
    pub is_hovered: bool,
    pub is_above: bool,
    pub popup_size: Vec2,
}

impl DocumentationPopup {
    pub fn new() -> Self {
        Self {
            is_hovered: false,
            popup_size: Vec2::new(0.0, 0.0),
            is_above: false,
        }
    }

    pub fn make_rect(&self, pos: Pos2) -> egui::Rect {
        egui::Rect::from_min_size(pos, self.popup_size)
    }

    pub fn check_if_hovered(&mut self, pos: Pos2, size: Vec2) -> bool {
        self.is_hovered = pos.x >= size.x
            && ((pos.y <= size.y && !self.is_above) || (pos.y >= size.y && self.is_above));
        log::info!("pos: {:?}", pos);
        log::info!("size: {:?}", size);
        log::info!("is_hovered: {}", self.is_hovered);
        self.is_hovered
    }

    pub fn widget<'a>(
        &'a mut self,
        documentation: &'a Documentation,
        font: &'a MonospaceFont,
    ) -> impl Widget + 'a {
        move |ui: &mut Ui| -> Response {
            let (id, rect) = ui.allocate_space(ui.available_size());
            let response = ui.interact(rect, id, egui::Sense::click_and_drag());

            // Set background color within the allocated rectangle
            let frame = egui::Frame::none()
                .fill(theme::POPUP_BACKGROUND) // Matches previous rect_filled color
                .inner_margin(1.0) // Adds some padding inside the box
                .rounding(5.0); // Optional rounded corners

            frame.show(&mut ui.child_ui(rect, *ui.layout(), None), |ui| {
                let scroll_response = egui::ScrollArea::vertical()
                    .id_source(format!("hover_scroll_{}", self.popup_size.length()))
                    .auto_shrink([true; 2]) // Prevents unwanted shrinking
                    .show(ui, |ui| {
                        ui.set_width(rect.width()); // Ensures content width matches the allocated rect
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            // Draw the message
                            for i in &documentation.hover_info.all_blocks {
                                let text = &i.0;
                                match i.1 {
                                    BlockType::CodeBlock => {
                                        ui.monospace(
                                            RichText::new(text)
                                                .color(Color32::from_rgb(170, 215, 255)),
                                        );
                                    }
                                    BlockType::RegularBlock => {
                                        ui.monospace(
                                            RichText::new(text).color(theme::syntax::DEFAULT),
                                        );
                                    }
                                }
                            }
                        });
                    });
            });

            response
        }
    }

    pub fn calc_origin(
        &mut self,
        documentation: &Documentation,
        offset: Vec2,
        padding: &Padding,
        font: &MonospaceFont,
    ) -> Pos2 {
        let mut height = font.size.y;
        let num_newlines = &documentation
            .hover_info
            .all_blocks
            .iter()
            .fold(0, |acc, block| {
                acc + block.0.chars().filter(|&c| c == '\n').count()
            })
            + &documentation.hover_info.all_blocks.len()
            - 1;
        height += num_newlines as f32 * font.size.y;
        log::info!("num_newlines: {}", num_newlines);
        // find the vertical start by finding top of line and then subtracting box size
        let total_padding: f32 = padding.cumulative(documentation.range.start.line + 1);

        let documentation_start = OUTER_PAD
            + total_padding
            + (documentation.range.start.line as f32 * font.size.y)
            + offset.y;
        let y = if height > documentation_start {
            // put it below the line if there isn't enough room above
            self.is_above = false;
            documentation_start + font.size.y
        } else {
            self.is_above = true;
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
        let num_newlines = &documentation
            .hover_info
            .all_blocks
            .iter()
            .fold(0, |acc, block| {
                acc + block.0.chars().filter(|&c| c == '\n').count()
            })
            + &documentation.hover_info.all_blocks.len();
        let height = (num_newlines) as f32 * font.size.y;
        let longest_line_len = documentation
            .message
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        let width = (longest_line_len) as f32 * font.size.x;
        let width = width.min(max_hover_width);
        let height = height.min(max_hover_height);
        Vec2::new(width, height)
    }
}

impl Documentation {
    #[warn(dead_code)]
    pub fn draw(
        &self,
        padding: &Padding,
        source: &Rope,
        _offset: Vec2,
        _font: &MonospaceFont,
        _painter: &Painter,
    ) {
        let range = self.range.ordered();
        let line_ranges = range.individual_lines(source);
    }
}
