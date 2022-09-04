use crate::BackgroundRect;
use slint::Color;
use tree_sitter::{Node, Tree};

pub fn rects_for_tree(tree: &Tree, src: &str) -> Vec<BackgroundRect> {
    let mut rects = vec![];

    // pre-order traversal because we want to draw the parent under their children
    let mut cursor = tree.root_node().walk();
    'outer: loop {
        // first time encountering the node, so draw it
        // draw_node(cursor.node(), &data.source, ctx);
        if let Some(rect) = rect_for_node(cursor.node(), src) {
            rects.push(rect);
        }

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

    rects
}

// these are just completely eyeballed currently
const FONT_WIDTH: f32 = 8.35;
const FONT_HEIGHT: f32 = 18.505;
const LINE_SPACING: f32 = 0.0;

fn rect_for_node(node: Node, src: &str) -> Option<BackgroundRect> {
    // don't draw boxes for nodes that are just string literals
    // without this every space would get it's own color
    if !node.is_named() {
        return None;
    }

    // get color/see if this node should be drawn
    // don't draw boxes for nodes that aren't high level
    let color = match color(&node) {
        Some(color) => color,
        None => return None,
    };

    let start = node.start_position();
    let end = node.end_position();

    let x = (start.column as f32) * FONT_WIDTH;
    let y = (start.row as f32) * (FONT_HEIGHT + LINE_SPACING);

    let (w, h) = {
        if start.row == end.row {
            // if block is all on one row, then
            let width = ((end.column - start.column) as f32) * FONT_WIDTH;
            (width, FONT_HEIGHT)
        } else {
            // if block is across rows,
            // then the end column won't necessarily be the furthest point to the left
            // this will also fix an out of bounds if start > end col
            let height = ((end.row - start.row + 1) as f32) * (FONT_HEIGHT + LINE_SPACING);
            // find the longest line of the block
            let columns = src[node.byte_range()]
                .lines()
                .map(|l| l.len())
                .max()
                .unwrap_or(0);
            (columns as f32 * FONT_WIDTH, height)
        }
    };

    Some(BackgroundRect { x, y, w, h, color })
}

fn color(node: &Node) -> Option<Color> {
    match node.kind() {
        "module" => Some(Color::from_rgb_f32(0.0, 0.4, 0.4)),
        "class_definition" => Some(Color::from_rgb_f32(0.9, 0.43, 0.212)),
        "function_definition" => Some(Color::from_rgb_f32(0.0, 0.47, 0.47)),
        "import_statement" => Some(Color::from_rgb_f32(0.77, 0.176, 0.188)),
        "expression_statement" => Some(Color::from_rgb_f32(0.5, 0.2, 0.5)),
        "while_statement" => Some(Color::from_rgb_f32(0.305, 0.0, 0.305)),
        "if_statement" => Some(Color::from_rgb_f32(0.502, 0.086, 0.22)),
        _ => None,
    }
}
