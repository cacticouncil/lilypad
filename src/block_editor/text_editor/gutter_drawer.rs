use std::collections::HashSet;

use druid::{
    piet::{PietTextLayout, Text, TextLayout, TextLayoutBuilder},
    PaintCtx, Point, RenderContext, Size,
};

use crate::{
    block_editor::{FONT_FAMILY, FONT_HEIGHT, FONT_SIZE, GUTTER_WIDTH, OUTER_PAD},
    theme,
};

use super::StackFrameLines;

pub fn draw_line_numbers(
    padding: &[f64],
    curr_line: usize,
    breakpoints: &HashSet<usize>,
    stack_frame: StackFrameLines,
    ctx: &mut PaintCtx,
) {
    // if document is empty, still draw line number 1
    if padding.is_empty() {
        let text = make_num_text(1, true, ctx);
        let pos = Point::new(GUTTER_WIDTH - text.size().width, OUTER_PAD);
        ctx.draw_text(&text, pos);
        return;
    }

    let font_height = *FONT_HEIGHT.get().unwrap();
    let mut y_pos = OUTER_PAD;
    for (num, padding) in padding.iter().enumerate() {
        y_pos += padding;

        // draw a background color for the stack trace lines
        // TODO: look better (maybe highlight the code instead of the gutter?)

        if let Some(selected) = stack_frame.selected {
            if selected == num + 1 {
                let rect = druid::kurbo::Rect::from_origin_size(
                    Point::new(0.0, y_pos),
                    Size::new(GUTTER_WIDTH, font_height),
                );
                ctx.fill(rect, &theme::STACK_FRAME_SELECTED);
            }
        }

        if let Some(deepest) = stack_frame.deepest {
            if deepest == num + 1 {
                let rect = druid::kurbo::Rect::from_origin_size(
                    Point::new(0.0, y_pos),
                    Size::new(GUTTER_WIDTH, font_height),
                );
                ctx.fill(rect, &theme::STACK_FRAME_DEEPEST);
            }
        }

        // draw a red dot before line numbers that have breakpoints
        if breakpoints.contains(&num) {
            let dot_pos = Point::new(10.0, y_pos + (font_height / 2.0));
            let dot = druid::kurbo::Circle::new(dot_pos, 4.0);
            ctx.fill(dot, &theme::BREAKPOINT);
        }

        // draw the line number
        let text = make_num_text(num + 1, curr_line == num, ctx);
        let pos = Point::new(
            GUTTER_WIDTH - text.size().width, // left align the text
            y_pos,
        );
        ctx.draw_text(&text, pos);

        y_pos += font_height;
    }
}

fn make_num_text(num: usize, curr: bool, ctx: &mut PaintCtx) -> PietTextLayout {
    let color = if curr {
        theme::INTERFACE_TEXT
    } else {
        theme::LINE_NUMBERS
    };
    ctx.text()
        .new_text_layout(num.to_string())
        .font(
            FONT_FAMILY.get().unwrap().clone(),
            *FONT_SIZE.get().unwrap(),
        )
        .text_color(color)
        .build()
        .unwrap()
}
