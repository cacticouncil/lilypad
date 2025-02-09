use egui::{Align2, Color32, Painter, Pos2, Rect, Vec2, Widget};
use ropey::Rope;
use std::collections::HashSet;

use crate::{
    block_editor::{blocks::Padding, MonospaceFont, GUTTER_WIDTH, OUTER_PAD},
    theme, vscode,
};

use super::{coord_conversions::pt_to_text_coord, StackFrameLines};

pub struct Gutter<'a> {
    curr_line: usize,
    breakpoints: &'a mut HashSet<usize>,
    stack_frame: StackFrameLines,
    padding: &'a Padding,
    source: &'a Rope,
    font: &'a MonospaceFont,
}

impl<'a> Gutter<'a> {
    pub fn new(
        curr_line: usize,
        breakpoints: &'a mut HashSet<usize>,
        stack_frame: StackFrameLines,
        padding: &'a Padding,
        source: &'a Rope,
        font: &'a MonospaceFont,
    ) -> Self {
        Self {
            curr_line,
            padding,
            breakpoints,
            stack_frame,
            source,
            font,
        }
    }
}

impl Widget for Gutter<'_> {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        let (id, rect) = ui.allocate_space(ui.available_size());
        let response = ui.interact(rect, id, egui::Sense::click());

        if response.clicked() {
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                self.handle_click(pointer_pos - rect.min.to_vec2());
            }
        }

        let mut preview_line: Option<usize> = None;
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Default);

            // draw breakpoint preview
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let loc = pt_to_text_coord(pointer_pos, self.padding, self.source, self.font);
                preview_line = Some(loc.line);
            }
        }

        self.draw(preview_line, rect.min.to_vec2(), ui.painter());

        response
    }
}

impl Gutter<'_> {
    fn draw(&self, preview_line: Option<usize>, offset: Vec2, painter: &Painter) {
        for (num, line_cumulative_padding) in self.padding.cumulative_iter().enumerate() {
            let y_pos =
                offset.y + line_cumulative_padding + (self.font.size.y * num as f32) + OUTER_PAD;

            // draw a background color for the stack trace lines
            // TODO: look better (maybe highlight the code instead of the gutter?)

            if let Some(selected) = self.stack_frame.selected {
                if selected == num + 1 {
                    let rect = Rect::from_min_size(
                        Pos2::new(offset.x, y_pos),
                        Vec2::new(GUTTER_WIDTH, self.font.size.y),
                    );
                    painter.rect_filled(rect, 0.0, theme::STACK_FRAME_SELECTED);
                }
            }

            if let Some(deepest) = self.stack_frame.deepest {
                if deepest == num + 1 {
                    let rect = Rect::from_min_size(
                        Pos2::new(offset.x, y_pos),
                        Vec2::new(GUTTER_WIDTH, self.font.size.y),
                    );
                    painter.rect_filled(rect, 0.0, theme::STACK_FRAME_DEEPEST);
                }
            }

            // draw a red dot before line numbers that have breakpoints
            let color: Option<Color32> = if self.breakpoints.contains(&num) {
                Some(theme::BREAKPOINT)
            } else if preview_line == Some(num) {
                Some(theme::PREVIEW_BREAKPOINT)
            } else {
                None
            };
            if let Some(color) = color {
                let dot_pos = Pos2::new(offset.x + 10.0, y_pos + (self.font.size.y / 2.0));
                painter.circle_filled(dot_pos, 4.0, color);
            }

            // draw the line number
            let color = if self.curr_line == num {
                theme::INTERFACE_TEXT
            } else {
                theme::LINE_NUMBERS
            };
            let display_num = (num + 1).to_string();
            let pos = Pos2::new(offset.x + GUTTER_WIDTH, y_pos);
            painter.text(
                pos,
                Align2::RIGHT_TOP,
                display_num,
                self.font.id.clone(),
                color,
            );
        }
    }

    fn handle_click(&mut self, pos: Pos2) {
        let loc = pt_to_text_coord(pos, self.padding, self.source, self.font);

        // TODO: clicking past the end currently adds a breakpoint to the last line, don't do that
        if self.breakpoints.contains(&loc.line) {
            self.breakpoints.remove(&loc.line);
        } else {
            self.breakpoints.insert(loc.line);
        }
        vscode::register_breakpoints(self.breakpoints.iter().cloned().collect());
    }
}
