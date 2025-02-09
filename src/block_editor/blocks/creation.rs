use tree_sitter::{Node, TreeCursor};

use super::{Block, BlockTrees, BlockType, Padding};
use crate::block_editor::{
    rope_ext::RopeSliceExt,
    text_range::{TextPoint, TextRange},
    SHOW_ERROR_BLOCK_OUTLINES,
};
use crate::lang::config::{LanguageConfig, NewScopeChar};

impl BlockType {
    fn from_node(node: &Node, lang: &LanguageConfig) -> Option<Self> {
        use BlockType::*;

        if SHOW_ERROR_BLOCK_OUTLINES && node.is_error() {
            return Some(Error);
        }

        lang.categorize_node(node)
    }

    pub const fn as_str(&self) -> &'static str {
        use BlockType::*;
        match self {
            Object => "Object",
            FunctionDef => "FunctionDef",
            While => "While",
            If => "If",
            For => "For",
            Try => "Try",
            Switch => "Switch",
            Generic => "Generic",
            Comment => "Comment",
            Error => "Error",
            Divider => "Divider",
        }
    }
}

impl Block {
    fn from_node(node: &Node, lang: &LanguageConfig) -> Option<Self> {
        let syntax_type = BlockType::from_node(node, lang)?;
        let start_pos = node.start_position();
        let end_pos = node.end_position();
        Some(Block {
            line: start_pos.row,
            col: std::cmp::min(start_pos.column, end_pos.column.saturating_sub(1)),
            height: end_pos.row - start_pos.row + 1,
            syntax_type,
            children: vec![],
        })
    }

    pub fn text_range(&self) -> TextRange {
        TextRange::new(
            TextPoint::new(self.line, self.col),
            TextPoint::new(self.line + self.height, 0),
        )
    }
}

impl BlockTrees {
    pub fn for_ts_tree(
        cursor: &mut TreeCursor,
        source: &ropey::Rope,
        lang: &LanguageConfig,
    ) -> Self {
        let mut trees = tree_to_blocks(cursor, lang);

        merge_comments(&mut trees, source);

        // insert divider blocks for 2+ lines of whitespace
        let newline_chunks = find_whitespace_chunks(source, 2);
        for chunk_start_line in newline_chunks {
            insert_divider(&mut trees, chunk_start_line);
        }

        merge_adjacent_generic_blocks(&mut trees);

        // if languages uses braces for new scopes,
        // adjust the block starts so that they contain their children
        // (since it would be possible for a block to start further in than its children)
        if lang.new_scope_char == NewScopeChar::Brace {
            adjust_block_starts(&mut trees);
        }

        let padding = Padding::for_blocks(&trees, source.len_lines());
        BlockTrees { trees, padding }
    }
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

/// Merges comments above a block into the block below.
/// Also discards comment blocks that are on the same line as code.
fn merge_comments(blocks: &mut Vec<Block>, source: &ropey::Rope) {
    let mut i = 0;
    while !blocks.is_empty() && i < blocks.len() {
        let curr = &blocks[i];

        if curr.syntax_type == BlockType::Comment && i < blocks.len() - 1 {
            let next = &blocks[i + 1];

            // touches the next block
            if curr.line + curr.height == next.line
                // don't merge with dividers
                && next.syntax_type != BlockType::Divider
                // not sharing a line with code
                && source.line(curr.line).whitespace_at_start() == curr.col
            {
                blocks[i + 1].line = blocks[i].line;
                blocks[i + 1].height += blocks[i].height;
            }

            blocks.remove(i);
        } else {
            merge_comments(&mut blocks[i].children, source);
            i += 1;
        }
    }
}

/// Combines adjacent generic blocks into one generic block
fn merge_adjacent_generic_blocks(blocks: &mut Vec<Block>) {
    // this makes the assumption that generic blocks won't have any children.
    // would need to be adjusted if that changes.
    let mut i = 0;
    while !blocks.is_empty() && i < blocks.len() {
        let curr = &blocks[i];

        if curr.syntax_type == BlockType::Generic && i < blocks.len() - 1 {
            // have current generic absorb following generic
            let next = &blocks[i + 1];
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

/// Adjust the column and width of blocks so that all blocks contain their descendants
fn adjust_block_starts(blocks: &mut Vec<Block>) -> usize {
    let mut max_col = usize::MAX;
    for block in blocks {
        // adjust children first and find the max column that still contains them
        let child_max = adjust_block_starts(&mut block.children);

        // divider columns are nonsense
        // so skip adjusting them
        if block.syntax_type == BlockType::Divider {
            max_col = child_max;
            continue;
        }

        // adjust this block if it does not contain its children
        if child_max < block.col {
            block.col = child_max;
        }

        // min the max column for this layer with this block
        if block.col < max_col {
            max_col = block.col;
        }
    }
    max_col
}
