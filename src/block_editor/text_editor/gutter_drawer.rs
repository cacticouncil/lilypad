use egui::{Align2, Painter, Pos2, Rect, Vec2};
use std::collections::HashSet;

use crate::{
    block_editor::{MonospaceFont, GUTTER_WIDTH},
    theme,
};

use super::StackFrameLines;

pub fn draw_line_numbers(
    mut padding: &[f32],
    offset: Vec2,
    curr_line: usize,
    breakpoints: &HashSet<usize>,
    stack_frame: StackFrameLines,
    font: &MonospaceFont,
    painter: &Painter,
) {
    // if document is empty, still draw line number 1
    if padding.is_empty() {
        padding = &[0.0];
    }

    let mut y_pos = offset.y;
    for (num, padding) in padding.iter().enumerate() {
        y_pos += padding;

        // draw a background color for the stack trace lines
        // TODO: look better (maybe highlight the code instead of the gutter?)

        if let Some(selected) = stack_frame.selected {
            if selected == num + 1 {
                let rect = Rect::from_min_size(
                    Pos2::new(offset.x, y_pos),
                    Vec2::new(GUTTER_WIDTH, font.size.y),
                );
                painter.rect_filled(rect, 0.0, theme::STACK_FRAME_SELECTED);
            }
        }

        if let Some(deepest) = stack_frame.deepest {
            if deepest == num + 1 {
                let rect = Rect::from_min_size(
                    Pos2::new(offset.x, y_pos),
                    Vec2::new(GUTTER_WIDTH, font.size.y),
                );
                painter.rect_filled(rect, 0.0, theme::STACK_FRAME_DEEPEST);
            }
        }

        // draw a red dot before line numbers that have breakpoints
        if breakpoints.contains(&num) {
            let dot_pos = Pos2::new(offset.x + 10.0, y_pos + (font.size.y / 2.0));
            painter.circle_filled(dot_pos, 4.0, theme::BREAKPOINT);
        }

        // draw the line number
        let color = if curr_line == num {
            theme::INTERFACE_TEXT
        } else {
            theme::LINE_NUMBERS
        };
        let display_num = (num + 1).to_string();
        let pos = Pos2::new(offset.x + GUTTER_WIDTH, y_pos);
        painter.text(pos, Align2::RIGHT_TOP, display_num, font.id.clone(), color);

        y_pos += font.size.y;
    }
}
