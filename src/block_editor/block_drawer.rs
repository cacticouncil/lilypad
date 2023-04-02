use druid::kurbo::RoundedRect;
use druid::{Color, PaintCtx, Point, RenderContext, Size};
use tree_sitter_c2rust::{Node, TreeCursor};

use crate::block_editor::{FONT_HEIGHT, FONT_WIDTH};

#[derive(PartialEq)]
enum BlockType {
    Class,
    FunctionDef,
    While,
    If,
    For,
    Try,
    Generic,
    Error,
    Divider,
}

impl BlockType {
    const fn color(&self) -> Option<Color> {
        use crate::theme::blocks::*;
        use BlockType::*;

        match self {
            Class => Some(CLASS),
            FunctionDef => Some(FUNCTION),
            While => Some(WHILE),
            If => Some(IF),
            For => Some(FOR),
            Try => Some(TRY),
            Generic => Some(GENERIC),
            Error => Some(Color::rgb8(255, 0, 0)),
            Divider => None,
        }
    }

    fn from_node(node: &Node) -> Option<Self> {
        use BlockType::*;

        if node.is_error() {
            return Some(Error);
        }

        match node.kind() {
            // scopes
            "class_definition" => Some(Class),
            "function_definition" => Some(FunctionDef),
            "while_statement" => Some(While),
            "if_statement" => Some(If),
            "for_statement" => Some(For),
            "try_statement" => Some(Try),

            // normal expressions (incomplete)
            "import_statement" => Some(Generic),
            "expression_statement" => Some(Generic),
            "comment" => Some(Generic),

            // dividers to keep generics from merging
            "else_clause" => Some(Divider),
            "elif_clause" => Some(Divider),
            "except_clause" => Some(Divider),

            // do not handle the rest
            _ => None,
        }
    }
}

pub struct Block {
    line: usize,
    col: usize,
    height: usize,
    syntax_type: BlockType,
    children: Vec<Block>,
}

impl Block {
    fn from_node(node: &Node) -> Option<Self> {
        let Some(syntax_type) = BlockType::from_node(node) else { return None };
        let start_pos = node.start_position();
        let end_pos = node.end_position();
        Some(Block {
            line: start_pos.row,
            col: start_pos.column,
            height: end_pos.row - start_pos.row + 1,
            syntax_type,
            children: vec![],
        })
    }

    const fn color(&self) -> Option<Color> {
        self.syntax_type.color()
    }
}

pub fn blocks_for_tree(cursor: &mut TreeCursor) -> Vec<Block> {
    // generate
    let mut root: Vec<Block> = vec![];
    let curr_node = cursor.node();

    // get all lower blocks
    let mut children: Vec<Block> = if cursor.goto_first_child() {
        let mut blocks = blocks_for_tree(cursor);

        while cursor.goto_next_sibling() {
            blocks.append(&mut blocks_for_tree(cursor));
        }

        cursor.goto_parent();

        blocks
    } else {
        vec![]
    };

    // merge adjacent generics
    let mut i = 0;
    while !children.is_empty() && i < children.len() - 1 {
        let curr = &children[i];
        let next = &children[i + 1];

        if curr.syntax_type == BlockType::Generic && next.syntax_type == BlockType::Generic {
            if curr.line + curr.height <= next.line {
                let gap = next.line - (curr.line + curr.height);
                children[i].height += gap + next.height;
            }

            // does not merge children because currently generic blocks won't have any.
            // if that changes, that will need to be added here

            children.remove(i + 1);
        } else {
            i += 1;
        }
    }

    // get block for current level
    if let Some(mut block) = Block::from_node(&curr_node) {
        block.children = children;
        root.push(block);
    } else {
        root.append(&mut children);
    }

    root
}

pub fn draw_blocks(blocks: Vec<Block>, ctx: &mut PaintCtx) {
    draw_blocks_helper(&blocks, 0, ctx);
}

fn draw_blocks_helper(blocks: &Vec<Block>, level: usize, ctx: &mut PaintCtx) {
    for block in blocks {
        let drawn = draw_block(block, level, ctx);
        draw_blocks_helper(&block.children, level + if drawn { 1 } else { 0 }, ctx);
    }
}

fn draw_block(block: &Block, level: usize, ctx: &mut PaintCtx) -> bool {
    // No color for invisible nodes
    let color = match block.color() {
        Some(color) => color,
        None => return false,
    };

    let start_pt = Point::new(
        (block.col as f64) * FONT_WIDTH,
        (block.line as f64) * FONT_HEIGHT,
    );

    // determine the margin based on level
    let margin = (start_pt.x) + (((level as f64) + 1.0) * FONT_WIDTH);

    // get the size of the rectangle to draw
    let size = Size::new(
        (ctx.size().width) - margin,
        (block.height as f64) * FONT_HEIGHT,
    );

    // draw it
    let rect = RoundedRect::from_origin_size(start_pt, size, 5.0);
    ctx.stroke(rect, &color, 3.0);
    // ctx.fill(rect, &block.color());

    true
}
