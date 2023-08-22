use druid::{
    piet::{PietTextLayout, Text, TextLayout, TextLayoutBuilder},
    PaintCtx, Point, RenderContext,
};

use super::{FONT_FAMILY, FONT_SIZE};
use crate::theme;

pub fn draw_line_numbers(padding: &Vec<f64>, curr_line: usize, ctx: &mut PaintCtx) {
    // if document is empty
    if padding.is_empty() {
        let text = make_num_text(1, true, ctx);
        let pos = Point::new(super::GUTTER_WIDTH - text.size().width, super::OUTER_PAD);
        ctx.draw_text(&text, pos);
        return;
    }

    let mut y_pos = super::OUTER_PAD;
    for (num, padding) in padding.iter().enumerate() {
        y_pos += padding;
        let text = make_num_text(num + 1, curr_line == num, ctx);
        let pos = Point::new(
            super::GUTTER_WIDTH - text.size().width, // left align the text
            y_pos,
        );
        ctx.draw_text(&text, pos);
        y_pos += super::FONT_HEIGHT.get().unwrap();
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
