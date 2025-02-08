use log::error;

use super::{Block, BlockType, BLOCK_CONFIG};

pub struct Padding {
    cumulative: Vec<f32>,
}

impl Padding {
    pub fn default() -> Self {
        Self {
            cumulative: vec![0.0],
        }
    }

    pub fn cumulative(&self, through_line: usize) -> f32 {
        if let Some(pad) = self.cumulative.get(through_line).copied() {
            pad
        } else {
            error!("Padding::cumulative: line out of bounds");
            0.0
        }
    }

    pub fn cumulative_iter<'a>(&'a self) -> impl Iterator<Item = f32> + 'a {
        self.cumulative.iter().copied()
    }

    pub fn individual(&self, line: usize) -> f32 {
        let line_pad = self.cumulative(line);

        if line == 0 {
            line_pad
        } else {
            let before_pad = self.cumulative(line - 1);
            line_pad - before_pad
        }
    }

    pub fn total(&self) -> f32 {
        self.cumulative.last().copied().unwrap_or(0.0)
    }

    pub fn count(&self) -> usize {
        self.cumulative.len()
    }

    pub fn for_blocks(blocks: &Vec<Block>, line_count: usize) -> Self {
        // empty file still gets one line in the editor
        if blocks.is_empty() {
            return Padding::default();
        }

        // find the individual padding for each line
        let mut padding = vec![0.0; line_count];
        Self::padding_helper(blocks, &mut padding);

        // convert to cumulative padding
        for i in 1..padding.len() {
            padding[i] += padding[i - 1];
        }

        Padding {
            cumulative: padding,
        }
    }

    fn padding_helper(blocks: &Vec<Block>, padding: &mut Vec<f32>) {
        // do not calculate padding for empty file
        // (there will still be one block for an empty file)
        if padding.is_empty() {
            return;
        }

        for block in blocks {
            if block.syntax_type != BlockType::Divider {
                padding[block.line] += BLOCK_CONFIG.total_top_pad();

                let end_line = block.line + block.height;
                if end_line < padding.len() {
                    padding[end_line] += BLOCK_CONFIG.total_inner_pad();
                }
            }
            Self::padding_helper(&block.children, padding);
        }
    }
}
