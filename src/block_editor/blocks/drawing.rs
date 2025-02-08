use std::ops::RangeInclusive;

use egui::{Painter, Pos2, Rect, Stroke, Vec2};

use super::{Block, BlockTrees, BlockType, BLOCK_CONFIG};
use crate::{block_editor::MonospaceFont, theme::blocks_theme::BlocksTheme};

impl BlockTrees {
    pub fn draw(
        &self,
        offset: Vec2,
        width: f32,
        visible_lines: Option<RangeInclusive<usize>>,
        blocks_theme: BlocksTheme,
        font: &MonospaceFont,
        painter: &Painter,
    ) {
        draw_blocks_helper(
            &self.trees,
            0,
            0.0,
            offset,
            width,
            visible_lines,
            blocks_theme,
            font,
            painter,
        );
    }
}

fn draw_blocks_helper(
    blocks: &Vec<Block>,
    level: usize,
    mut total_padding: f32,
    offset: Vec2,
    width: f32,
    visible_lines: Option<RangeInclusive<usize>>,
    blocks_theme: BlocksTheme,
    font: &MonospaceFont,
    painter: &Painter,
) -> f32 {
    // TODO: this probably could be changed to reuse the already calculated padding
    for block in blocks {
        if block.syntax_type == BlockType::Divider {
            // do not draw this block
            total_padding = draw_blocks_helper(
                &block.children,
                level,
                total_padding,
                offset,
                width,
                visible_lines.clone(),
                blocks_theme,
                font,
                painter,
            );
        } else {
            total_padding += BLOCK_CONFIG.total_top_pad();

            // draw children first to get total size
            let inside_padding = draw_blocks_helper(
                &block.children,
                level + 1,
                total_padding,
                offset,
                width,
                visible_lines.clone(),
                blocks_theme,
                font,
                painter,
            ) - total_padding;

            let block_visible = match visible_lines.clone() {
                Some(visible) => {
                    block.line <= *visible.end() && *visible.start() <= block.line + block.height
                }
                None => true,
            };

            if block_visible {
                draw_block(
                    block,
                    level,
                    total_padding,
                    inside_padding,
                    offset,
                    width,
                    blocks_theme,
                    font,
                    painter,
                );
            }

            total_padding += inside_padding;
            total_padding += BLOCK_CONFIG.total_inner_pad();
        }
    }

    total_padding
}

fn draw_block(
    block: &Block,
    level: usize,
    padding_above: f32,
    padding_inside: f32,
    offset: Vec2,
    width: f32,
    blocks_theme: BlocksTheme,
    font: &MonospaceFont,
    painter: &Painter,
) {
    // No color for invisible nodes
    let color = match (blocks_theme.color_for)(block.syntax_type, level) {
        Some(color) => color,
        None => return,
    };

    let start_pt = Pos2::new(
        (block.col as f32) * font.size.x - (BLOCK_CONFIG.stroke_width / 2.0),
        (block.line as f32) * font.size.y
            - (BLOCK_CONFIG.stroke_width / 2.0)
            - (BLOCK_CONFIG.inner_pad / 2.0)
            + padding_above,
    );

    // determine the margin based on level
    let right_margin = (level as f32) * (BLOCK_CONFIG.inner_pad + BLOCK_CONFIG.stroke_width);

    // get the size of the rectangle to draw
    let size = Vec2::new(
        width - start_pt.x - right_margin,
        ((block.height as f32) * font.size.y) + (BLOCK_CONFIG.inner_pad * 2.0) + padding_inside,
    );

    // nested corner radii should be r_inner = r_outer - distance
    let rounding = f32::max(
        BLOCK_CONFIG.outer_corner_rad - (level as f32 * BLOCK_CONFIG.inner_pad),
        BLOCK_CONFIG.min_corner_rad,
    );

    // draw it
    let rect = Rect::from_min_size(start_pt + offset, size);
    let stroke = Stroke::new(BLOCK_CONFIG.stroke_width, color);
    painter.rect_stroke(rect, rounding, stroke);
}
