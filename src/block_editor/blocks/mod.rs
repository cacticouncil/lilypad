mod creation;
mod drawing;
mod padding;

pub use padding::Padding;

pub struct BlockTrees {
    // the individual trees of blocks
    trees: Vec<Block>,

    /// the padding at each line caused by the blocks
    padding: Padding,
}

impl BlockTrees {
    pub fn default() -> Self {
        BlockTrees {
            trees: vec![],
            padding: Padding::default(),
        }
    }

    pub fn trees(&self) -> &[Block] {
        &self.trees
    }

    pub fn padding(&self) -> &Padding {
        &self.padding
    }

    #[allow(dead_code)]
    pub fn print_debug(&self) {
        fn print_blocks_debug_helper(blocks: &[Block], indent: &str, last: bool) {
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

        print_blocks_debug_helper(&self.trees, "", true);
    }
}

pub struct Block {
    pub line: usize,
    pub col: usize,
    pub height: usize,
    pub syntax_type: BlockType,
    pub children: Vec<Block>,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum BlockType {
    Object,
    FunctionDef,
    While,
    If,
    For,
    Try,
    Switch,
    Generic,
    Comment,
    Error,
    Divider,
}

struct BlockConfig {
    pub outer_corner_rad: f32,
    pub min_corner_rad: f32,
    pub stroke_width: f32,
    pub inner_pad: f32,
    pub top_outer_pad: f32,
}

impl BlockConfig {
    pub const fn total_top_pad(&self) -> f32 {
        self.stroke_width + self.inner_pad + self.top_outer_pad
    }

    pub const fn total_inner_pad(&self) -> f32 {
        self.stroke_width + self.inner_pad
    }
}

const BLOCK_CONFIG: BlockConfig = BlockConfig {
    outer_corner_rad: 6.0,
    min_corner_rad: 1.5,
    stroke_width: 1.5,
    inner_pad: 3.0,
    top_outer_pad: 1.0,
};
