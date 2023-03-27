use druid::{Color, PaintCtx, Point, Rect, RenderContext, Size};
use druid::kurbo::{RoundedRect};
use tree_sitter_c2rust::Node;

use crate::block_editor::{FONT_HEIGHT, FONT_WIDTH};

pub fn draw(node: Node, ctx: &mut PaintCtx) {
    // get color (and see if this node should be drawn)
    let color = match color(&node) {
        Some(color) => color,
        None => return,
    };

    let start = node.start_position();
    let end = node.end_position();

    let start_pt = Point::new(
        (start.column as f64) * FONT_WIDTH,
        (start.row as f64) * FONT_HEIGHT,
    );

    // determine the margin based on tabbing
    let margin =
        (start_pt.x) + (((node.start_position().column as f64 / 4.0) + 1.0) * (1.0 * FONT_WIDTH));

    // get the size of the rectangle to draw
    let size = Size::new(
        (ctx.size().width) - margin,
        ((end.row - start.row + 1) as f64) * FONT_HEIGHT,
    );

    // draw it
    let block = RoundedRect::from_origin_size(start_pt, size, 5.0);
    ctx.fill(block, &color);
}

fn color(node: &Node) -> Option<Color> {
    use crate::theme::blocks::*;

    if node.is_error() {
        return Some(Color::rgb(1.0, 0.0, 0.0));
    }

    match node.kind() {
        "class_definition" => Some(CLASS),
        "function_definition" => Some(FUNCTION),
        "import_statement" => Some(GENERIC),
        "expression_statement" => Some(GENERIC),
        "while_statement" => Some(WHILE),
        "if_statement" => Some(IF),
        "for_statement" => Some(FOR),
        "try_statement" => Some(TRY),
        // "comment" => Some(COMMENT),
        _ => None,
    }
}
