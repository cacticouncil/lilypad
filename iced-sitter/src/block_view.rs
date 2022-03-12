use iced::{
    canvas::{Frame, Path, Program, Text},
    Color, Font, HorizontalAlignment, Point, Rectangle, Size, VerticalAlignment,
};
use tree_sitter::{Node, Tree};

use crate::Message;

pub struct BlockView {
    source: String, // TODO: optimize to a &str
    tree: Tree,
}

impl BlockView {
    pub fn new(source: String, tree: Tree) -> Self {
        Self { source, tree }
    }
}

// These are temp values that seemed *close enough*
// TODO: figure out exact values (as a ratio to the font size)
const FONT_WIDTH: f32 = 9.09;
const FONT_HEIGHT: f32 = 20.0;

impl BlockView {
    fn color(node: &Node) -> Color {
        match node.kind() {
            "module" => Color::from_rgb(0.0, 0.4, 0.4),
            "import_statement" => Color::from_rgb(0.4, 0.1, 0.2),
            "expression_statement" => Color::from_rgb(0.5, 0.1, 0.5),
            "while_statement" => Color::from_rgb(0.7, 0.4, 0.4),
            "if_statement" => Color::from_rgb(0.2, 0.3, 0.2),
            _ => Color::from_rgb(1.0, 1.0, 1.0),
        }
    }

    fn draw_node(&self, node: Node, frame: &mut Frame) {
        // don't draw boxes for nodes that are just string literals
        // without this every space would get it's own color
        if !node.is_named() {
            return;
        }

        // don't draw boxes for nodes that aren't high level
        // TODO: maybe a hash table?? depends on scale
        let nodes_to_draw: [&'static str; 5] = [
            "module",
            "import_statement",
            "expression_statement",
            "while_statement",
            "if_statement",
        ];
        if !nodes_to_draw.contains(&node.kind()) {
            return;
        }

        let start = node.start_position();
        let end = node.end_position();

        let start_pt = Point::new(
            (start.column as f32) * FONT_WIDTH,
            (start.row as f32) * FONT_HEIGHT,
        );

        let size = {
            if start.row == end.row {
                // if block is all on one row, then
                let width = ((end.column - start.column) as f32) * FONT_WIDTH;
                Size::new(width, FONT_HEIGHT)
            } else {
                // if block is across rows,
                // then the end column won't necessarily be the furthest point to the left
                // this will also fix an out of bounds if start > end col
                let height = ((end.row - start.row + 1) as f32) * FONT_HEIGHT;
                // find the longest line of the block
                let columns = self.source[node.byte_range()]
                    .lines()
                    .map(|l| l.len())
                    .max()
                    .unwrap_or(0);
                Size::new(columns as f32 * FONT_WIDTH, height)
            }
        };

        // draw the block in
        let block = Path::rectangle(start_pt, size);
        frame.fill(&block, Self::color(&node));

        // draw text that the block contains
        // this is currently replaced by drawing all the text at once
        // this is handy to uncomment to test aligning blocks with text
        
        // let string = &self.source[node.byte_range()];
        // let text = Text {
        //     content: string.to_string(),
        //     position: start_pt,
        //     color: Color::BLACK,
        //     size: 20.0,
        //     font: Font::Default,
        //     horizontal_alignment: HorizontalAlignment::Left,
        //     vertical_alignment: VerticalAlignment::Top,
        // };
        // frame.fill_text(text);
    }
}

impl Program<Message> for BlockView {
    fn draw(
        &self,
        bounds: Rectangle,
        _cursor: iced::canvas::Cursor, // TODO: look into this redrawing every time the cursor moves (which is way too much)
    ) -> Vec<iced::canvas::Geometry> {
        let mut frame = Frame::new(bounds.size());

        // pre-order traversal because we want to draw the parent under their children
        let mut cursor = self.tree.root_node().walk();
        'outer: loop {
            // first time encountering the node, so draw it
            self.draw_node(cursor.node(), &mut frame);

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

        let text = Text {
            content: self.source.clone(),
            position: Point::ORIGIN,
            color: Color::WHITE,
            size: 20.0,
            font: Font::Default,
            horizontal_alignment: HorizontalAlignment::Left,
            vertical_alignment: VerticalAlignment::Top,
        };
        frame.fill_text(text);

        vec![frame.into_geometry()]
    }
}
