use druid::kurbo::RoundedRect;
use druid::{Color, PaintCtx, Point, RenderContext, Size};
use tree_sitter_c2rust::{Node, TreeCursor};

use crate::block_editor::{FONT_HEIGHT, FONT_WIDTH};

pub fn draw_for_tree(mut cursor: TreeCursor, ctx: &mut PaintCtx) {
    // pre-order traversal because we want to draw the parent under their children
    'outer: loop {
        // first time encountering the node, so draw it
        draw_node(cursor.node(), ctx);

        // keep traveling down the tree as far as we can
        if cursor.goto_first_child() {
            continue;
        }

        // if we can't travel any further down, try the next sibling
        if cursor.goto_next_sibling() {
            continue;
        }

        // travel back up
        // loop until we reach the root or can go to the next sibling of a node again
        'inner: loop {
            // break outer if we reached the root
            if !cursor.goto_parent() {
                break 'outer;
            }

            // if there is a sibling at this level, visit the sibling's subtree
            if cursor.goto_next_sibling() {
                break 'inner;
            }
        }
    }
}

fn draw_node(node: Node, ctx: &mut PaintCtx) {
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
