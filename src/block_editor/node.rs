use druid::piet::{Text, TextLayoutBuilder};
use druid::{Color, FontFamily, PaintCtx, Point, Rect, RenderContext, Size};
use tree_sitter::Node;

use crate::block_editor::{FONT_HEIGHT, FONT_WIDTH};

pub fn draw(node: Node, source: &str, ctx: &mut PaintCtx) {
    // don't draw boxes for nodes that are just string literals
    // without this every space would get it's own color
    if !node.is_named() {
        return;
    }

    // get color/see if this node should be drawn
    // don't draw boxes for nodes that aren't high level
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

    let mut margin: f64 = 0.0;

    // Check that node is high level and then determine the margin based on tabbing
    if check_node_name(&node) {
        margin = (start_pt.x)
            + (((node.start_position().column as f64 / 4.0) + 1.0) * (1.0 * FONT_WIDTH));
    }

    let size = {
        if start.row == end.row {
            // if block is all on one row, then
            Size::new(ctx.size().width - margin, FONT_HEIGHT)
        } else {
            // if block is across rows
            let height = ((end.row - start.row + 1) as f64) * FONT_HEIGHT;
            // Fill entire screen width with block for module node
            if node.kind() == "module" {
                Size::new(ctx.size().width, height)
            } else {
                Size::new((ctx.size().width) - margin, height)
            }
        }
    };

    // draw the block in
    let block = Rect::from_origin_size(start_pt, size);
    ctx.fill(block, &color);
    // draw text in
    draw_text(node, source, ctx, start_pt.x, start_pt.y);
}

fn color(node: &Node) -> Option<Color> {
    if node.is_error() {
        return Some(Color::rgb(1.0, 0.0, 0.0));
    }

    match node.kind() {
        "class_definition" => Some(Color::rgb(0.9, 0.43, 0.212)),
        "function_definition" => Some(Color::rgb(0.0, 0.47, 0.47)),
        "import_statement" => Some(Color::rgb(0.77, 0.176, 0.188)),
        "expression_statement" => Some(Color::rgb(0.5, 0.2, 0.5)),
        "while_statement" => Some(Color::rgb(0.305, 0.0, 0.305)),
        "if_statement" => Some(Color::rgb(0.502, 0.086, 0.22)),
        "else_clause" => Some(Color::rgb(0.502, 0.086, 0.22)),
        "break_statement" => Some(Color::rgb(0.5, 0.2, 0.5)),
        "for_statement" => Some(Color::rgb(0.305, 0.0, 0.305)),
        "try_statement" => Some(Color::rgb(0.502, 0.086, 0.22)),
        "except_clause" => Some(Color::rgb(0.502, 0.086, 0.22)),
        "finally_clause" => Some(Color::rgb(0.502, 0.086, 0.22)),
        "elif_clause" => Some(Color::rgb(0.502, 0.086, 0.22)),
        "comment" => Some(Color::rgb(0.0, 0.4, 0.4)),
        "continue_statement" => Some(Color::rgb(0.5, 0.2, 0.5)),
        //"pair" => Some(Color::rgb(0.502, 0.086, 0.22)),
        _ => None,
    }
}

fn draw_text(node: Node, source: &str, ctx: &mut PaintCtx, start_x: f64, start_y: f64) {
    let source_line = get_first_node_line(node, source);

    let text = ctx.text();
    let layout = text
        .new_text_layout(source_line)
        .font(FontFamily::new_unchecked("Roboto Mono"), 15.0)
        .text_color(Color::WHITE)
        .build()
        .unwrap();
    ctx.draw_text(&layout, (start_x, start_y));
}

fn get_first_node_line(node: Node, source: &str) -> String {
    source[node.byte_range()]
        .lines()
        .next()
        .unwrap()
        .to_string()
}

fn check_node_name(node: &Node) -> bool {
    let node_names = [
        "class_definition",
        "function_definition",
        "import_statement",
        "expression_statement",
        "while_statement",
        "if_statement",
        "else_clause",
        "break_statement",
        "for_statement",
        "try_statement",
        "except_clause",
        "finally_clause",
        "elif_clause",
        "comment",
        "continue_statement",
    ];

    node_names.contains(&node.kind())
}
