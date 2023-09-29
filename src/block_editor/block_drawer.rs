use druid::kurbo::RoundedRect;
use druid::{Color, PaintCtx, Point, RenderContext, Size};
use tree_sitter_c2rust::{Node, TreeCursor};

use crate::block_editor::{FONT_HEIGHT, FONT_WIDTH};
use crate::lang::LanguageConfig;

use super::rope_ext::RopeSliceExt;
use super::text_range::{TextPoint, TextRange};
use super::{OUTER_PAD, SHOW_ERROR_BLOCK_OUTLINES};

/* ------------------------------ tree handling ----------------------------- */

#[derive(PartialEq, Clone, Copy, Debug)]
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
    pub line: usize,
    pub col: usize,
    pub height: usize,
    pub syntax_type: BlockType,
    pub children: Vec<Block>,
}

impl Block {
    fn from_node(node: &Node, lang: &LanguageConfig) -> Option<Self> {
        let Some(syntax_type) = BlockType::from_node(node, lang) else {
            return None;
        };
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

    pub fn text_range(&self) -> TextRange {
        TextRange {
            start: TextPoint {
                col: self.col,
                row: self.line,
            },
            end: TextPoint {
                col: 0,
                row: self.line + self.height,
            },
        }
    }
}

/* ----------------------- tree sitter tree to blocks ----------------------- */

pub fn blocks_for_tree(
    cursor: &mut TreeCursor,
    source: &ropey::Rope,
    lang: &LanguageConfig,
) -> Vec<Block> {
    let mut blocks = tree_to_blocks(cursor, lang);

    // insert divider blocks for 2+ lines of whitespace
    let newline_chunks = find_whitespace_chunks(source, 2);
    for chunk_start_line in newline_chunks {
        insert_divider(&mut blocks, chunk_start_line);
    }

    merge_adjacent_generic_blocks(&mut blocks);

    blocks
}

/// Converts a tree sitter tree to a tree of blocks (with no additional processing)
fn tree_to_blocks(cursor: &mut TreeCursor, lang: &LanguageConfig) -> Vec<Block> {
    // get the current node before moving the cursor
    let curr_node = cursor.node();

    // get all lower blocks
    let mut children: Vec<Block> = if cursor.goto_first_child() {
        let mut blocks = tree_to_blocks(cursor, lang);

        while cursor.goto_next_sibling() {
            blocks.append(&mut tree_to_blocks(cursor, lang));
        }

        cursor.goto_parent();

        blocks
    } else {
        vec![]
    };

    // get block for current level
    let mut root: Vec<Block> = vec![];
    if let Some(mut block) = Block::from_node(&curr_node, lang) {
        // if the current node gets a block, add it to the root
        block.children = children;
        root.push(block);
    } else {
        // otherwise, add the children to the top level
        root.append(&mut children);
    }

    root
}

fn merge_adjacent_generic_blocks(blocks: &mut Vec<Block>) {
    // this makes the assumption that generic blocks won't have any children.
    // would need to be adjusted if that changes.

    let mut i = 0;
    while !blocks.is_empty() && i < blocks.len() - 1 {
        let curr = &blocks[i];
        let next = &blocks[i + 1];

        if curr.syntax_type == BlockType::Generic {
            // have current generic absorb following generic
            if next.syntax_type == BlockType::Generic {
                if curr.line + curr.height <= next.line {
                    let gap = next.line - (curr.line + curr.height);
                    blocks[i].height += gap + next.height;
                }

                blocks.remove(i + 1);
            } else {
                i += 1;
            }
        } else {
            merge_adjacent_generic_blocks(&mut blocks[i].children);
            i += 1;
        }
    }
}

// Inserts a divider at the given line
fn insert_divider(blocks: &mut Vec<Block>, line: usize) {
    let divider = Block {
        line,
        col: 0,
        height: 0,
        syntax_type: BlockType::Divider,
        children: vec![],
    };

    let mut curr_level = blocks;
    'outer: while !curr_level.is_empty() {
        for idx in 0..curr_level.len() {
            let block = &curr_level[idx];

            // if block contains the line, insert the divider inside the block
            // otherwise, insert before the first block past that line
            if block.line <= line && line < block.line + block.height {
                curr_level = &mut curr_level[idx].children;
                continue 'outer;
            }

            if block.line > line {
                curr_level.insert(idx, divider);
                return;
            }
        }
        break;
    }
}

/// Finds the starting indexes of chunks consisting of chunk_size or more whitespace lines
fn find_whitespace_chunks(source: &ropey::Rope, chunk_size: usize) -> Vec<usize> {
    // find all lines that are whitespace
    let whitespace_lines: Vec<usize> = source
        .lines()
        .enumerate()
        .filter(|(_, line)| line.whitespace_at_start() == line.excluding_linebreak().len_chars())
        .map(|(idx, _)| idx)
        .collect();

    // filter to just chunks of two or more (and only keep the first in a chunk)
    let mut chunk_starts = vec![];
    let mut current_chunk = vec![];

    for line in whitespace_lines {
        if current_chunk.is_empty() || current_chunk.last().map(|x| x + 1) == Some(line) {
            current_chunk.push(line);
        } else {
            if current_chunk.len() >= chunk_size {
                chunk_starts.push(current_chunk[0]);
            }
            current_chunk.clear();
            current_chunk.push(line);
        }
    }

    if current_chunk.len() >= chunk_size {
        chunk_starts.push(current_chunk[0]);
    }

    chunk_starts
}

/* --------------------------------- drawing -------------------------------- */

const OUTER_CORNER_RAD: f64 = 6.0;
const MIN_CORNER_RAD: f64 = 1.5;

const BLOCK_STROKE_WIDTH: f64 = 2.0;
const BLOCK_INNER_PAD: f64 = 2.0;
const BLOCK_TOP_PAD: f64 = 1.0;

pub fn draw_blocks(blocks: &Vec<Block>, offset: Point, ctx: &mut PaintCtx) {
    draw_blocks_helper(blocks, 0, 0.0, offset, ctx);
}

fn draw_blocks_helper(
    blocks: &Vec<Block>,
    level: usize,
    mut total_padding: f64,
    offset: Point,
    ctx: &mut PaintCtx,
) -> f64 {
    for block in blocks {
        if block.syntax_type == BlockType::Divider {
            // do not draw this block
            total_padding = draw_blocks_helper(&block.children, level, total_padding, offset, ctx);
        } else {
            total_padding += BLOCK_STROKE_WIDTH + BLOCK_INNER_PAD + BLOCK_TOP_PAD;

            // draw children first to get total size
            let inside_padding =
                draw_blocks_helper(&block.children, level + 1, total_padding, offset, ctx)
                    - total_padding;

            draw_block(block, level, total_padding, inside_padding, offset, ctx);
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
    offset: Point,
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
        (block.col as f64) * font_width + offset.x - (BLOCK_STROKE_WIDTH / 2.0),
        (block.line as f64) * font_height + offset.y - (BLOCK_STROKE_WIDTH / 2.0) + padding_above,
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

/* -------------------------------- debugging ------------------------------- */

#[allow(dead_code)]
pub fn print_blocks_debug(blocks: &Vec<Block>) {
    print_blocks_debug_helper(blocks, "", true);
}

fn print_blocks_debug_helper(blocks: &Vec<Block>, indent: &str, last: bool) {
    let join_symbol = if last { "└─ " } else { "├─ " };

    let new_indent = format!("{}{}", indent, if last { "    " } else { "│  " });
    for (idx, block) in blocks.iter().enumerate() {
        let last_child = idx == blocks.len() - 1;
        println!(
            "{}{}{:?} ({:?})",
            indent,
            join_symbol,
            block.syntax_type,
            block.text_range()
        );
        print_blocks_debug_helper(&block.children, &new_indent, last_child);
    }
}
