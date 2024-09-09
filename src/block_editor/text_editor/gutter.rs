use egui::{Align2, Painter, Pos2, Rect, Vec2, Widget};
use ropey::Rope;
use std::collections::HashSet;

use crate::{
    block_editor::{MonospaceFont, GUTTER_WIDTH},
    theme, vscode,
};

use super::{coord_conversions::pt_to_text_coord, StackFrameLines};

pub struct Gutter<'a> {
    curr_line: usize,
    breakpoints: &'a mut HashSet<usize>,
    stack_frame: StackFrameLines,
    padding: &'a [f32],
    source: &'a Rope,
    font: &'a MonospaceFont,
}

impl<'a> Gutter<'a> {
    pub fn new(
        curr_line: usize,
        breakpoints: &'a mut HashSet<usize>,
        stack_frame: StackFrameLines,
        padding: &'a [f32],
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

impl<'a> Widget for Gutter<'a> {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        // if document is empty, still draw line number 1
        if self.padding.is_empty() {
            self.padding = &[0.0];
        }

        let (id, rect) = ui.allocate_space(ui.available_size());
        let response = ui.interact(rect, id, egui::Sense::click());

        if response.clicked() {
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                self.handle_click(pointer_pos);
            }
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
        }

        self.draw(rect.min.to_vec2(), ui.painter());

        response
    }
}

impl<'a> Gutter<'a> {
    fn draw(&self, offset: Vec2, painter: &Painter) {
        let mut y_pos = offset.y;
        for (num, padding) in self.padding.iter().enumerate() {
            y_pos += padding;

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
            if self.breakpoints.contains(&num) {
                let dot_pos = Pos2::new(offset.x + 10.0, y_pos + (self.font.size.y / 2.0));
                painter.circle_filled(dot_pos, 4.0, theme::BREAKPOINT);
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

            y_pos += self.font.size.y;
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
