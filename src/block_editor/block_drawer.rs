use druid::kurbo::RoundedRect;
use druid::{Color, PaintCtx, Point, RenderContext, Size};
use tree_sitter_c2rust::{Node, TreeCursor};

use crate::block_editor::{FONT_HEIGHT, FONT_WIDTH};
use crate::lang::LanguageConfig;

use super::{GUTTER_WIDTH, OUTER_PAD, SHOW_ERROR_BLOCK_OUTLINES};

/* ------------------------------ tree handling ----------------------------- */

#[derive(PartialEq, Clone, Copy)]
pub enum BlockType {
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
            Error => Some(ERROR),
            Divider => None,
        }
    }

    fn from_node(node: &Node, lang: &LanguageConfig) -> Option<Self> {
        use BlockType::*;

        if SHOW_ERROR_BLOCK_OUTLINES && node.is_error() {
            return Some(Error);
        }

        lang.categorize_node(node)
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
    fn from_node(node: &Node, lang: &LanguageConfig) -> Option<Self> {
        let Some(syntax_type) = BlockType::from_node(node, lang) else { return None };
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

pub fn blocks_for_tree(cursor: &mut TreeCursor, lang: &LanguageConfig) -> Vec<Block> {
    // generate
    let mut root: Vec<Block> = vec![];
    let curr_node = cursor.node();

    // get all lower blocks
    let mut children: Vec<Block> = if cursor.goto_first_child() {
        let mut blocks = blocks_for_tree(cursor, lang);

        while cursor.goto_next_sibling() {
            blocks.append(&mut blocks_for_tree(cursor, lang));
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
    if let Some(mut block) = Block::from_node(&curr_node, lang) {
        block.children = children;
        root.push(block);
    } else {
        root.append(&mut children);
    }

    root
}

/* --------------------------------- drawing -------------------------------- */

const OUTER_CORNER_RAD: f64 = 6.0;
const MIN_CORNER_RAD: f64 = 1.5;

const BLOCK_STROKE_WIDTH: f64 = 2.0;
const BLOCK_INNER_PAD: f64 = 2.0;
const BLOCK_TOP_PAD: f64 = 1.0;

pub fn draw_blocks(blocks: &Vec<Block>, ctx: &mut PaintCtx) {
    draw_blocks_helper(blocks, 0, 0.0, ctx);
}

fn draw_blocks_helper(
    blocks: &Vec<Block>,
    level: usize,
    mut total_padding: f64,
    ctx: &mut PaintCtx,
) -> f64 {
    for block in blocks {
        if block.syntax_type == BlockType::Divider {
            // do not draw this block
            total_padding = draw_blocks_helper(&block.children, level, total_padding, ctx);
        } else {
            total_padding += BLOCK_STROKE_WIDTH + BLOCK_INNER_PAD + BLOCK_TOP_PAD;

            // draw children first to get total size
            let inside_padding =
                draw_blocks_helper(&block.children, level + 1, total_padding, ctx) - total_padding;

            draw_block(block, level, total_padding, inside_padding, ctx);
            total_padding += inside_padding;
            total_padding += BLOCK_STROKE_WIDTH + BLOCK_INNER_PAD;
        }
    }

    total_padding
}

fn draw_block(
    block: &Block,
    level: usize,
    padding_above: f64,
    padding_inside: f64,
    ctx: &mut PaintCtx,
) {
    // No color for invisible nodes
    let color = match block.color() {
        Some(color) => color,
        None => return,
    };

    let font_width = *FONT_WIDTH.get().unwrap();
    let font_height = *FONT_HEIGHT.get().unwrap();

    let start_pt = Point::new(
        (block.col as f64) * font_width + OUTER_PAD + GUTTER_WIDTH - (BLOCK_STROKE_WIDTH / 2.0),
        (block.line as f64) * font_height + OUTER_PAD - (BLOCK_STROKE_WIDTH / 2.0) + padding_above,
    );

    // determine the margin based on level
    let margin =
        (start_pt.x) + ((level as f64) * (BLOCK_INNER_PAD + BLOCK_STROKE_WIDTH)) + OUTER_PAD;

    // get the size of the rectangle to draw
    let size = Size::new(
        (ctx.size().width) - margin,
        ((block.height as f64) * font_height) + (BLOCK_INNER_PAD * 2.0) + padding_inside,
    );

    // nested corner radii should be r_inner = r_outer - distance
    let rounding = f64::max(
        OUTER_CORNER_RAD - (level as f64 * BLOCK_INNER_PAD),
        MIN_CORNER_RAD,
    );

    // draw it
    let rect = RoundedRect::from_origin_size(start_pt, size, rounding);
    ctx.stroke(rect, &color, BLOCK_STROKE_WIDTH);
}

/* ---------------------------- padding for text ---------------------------- */

pub fn make_padding(blocks: &Vec<Block>, line_count: usize) -> Vec<f64> {
    let mut padding = vec![0.0; line_count];
    padding_helper(blocks, &mut padding);
    padding
}

fn padding_helper(blocks: &Vec<Block>, padding: &mut Vec<f64>) {
    // do not calculate padding for empty file
    // (there will still be one block for an empty file)
    if padding.is_empty() {
        return;
    }

    for block in blocks {
        if block.syntax_type != BlockType::Divider {
            padding[block.line] += BLOCK_STROKE_WIDTH + BLOCK_INNER_PAD + BLOCK_TOP_PAD;

            let end_row = block.line + block.height;
            if end_row < padding.len() {
                padding[end_row] += BLOCK_STROKE_WIDTH + BLOCK_INNER_PAD;
            }
        }
        padding_helper(&block.children, padding);
    }
}
