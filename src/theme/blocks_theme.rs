use egui::Color32;

use super::one_dark;
use crate::block_editor::BlockType;

#[derive(Clone, Copy, PartialEq)]
pub struct BlocksTheme {
    // Given a block type and a depth, return the color to use
    pub color_for: fn(BlockType, usize) -> Option<Color32>,
}

impl BlocksTheme {
    const fn new(color_for: fn(BlockType, usize) -> Option<Color32>) -> Self {
        Self { color_for }
    }

    pub fn for_str(string: &str) -> Self {
        match string {
            "syntax_colored" => SYNTAX_COLORED_BLOCKS,
            "depth_grayscale" => GRAYSCALE_DEPTH_BLOCKS,
            "alternating_colored" => ALTERNATING_COLORED_BLOCKS,
            _ => SYNTAX_COLORED_BLOCKS,
        }
    }
}

static SYNTAX_COLORED_BLOCKS: BlocksTheme = BlocksTheme::new(|block_type, _| {
    use crate::block_editor::BlockType::*;

    match block_type {
        Object => Some(Color32::from_rgb(255, 71, 72)),
        FunctionDef => Some(Color32::from_rgb(163, 93, 213)),
        While => Some(Color32::from_rgb(245, 163, 0)),
        If => Some(Color32::from_rgb(103, 199, 40)),
        For => Some(Color32::from_rgb(255, 131, 193)),
        Try => Some(Color32::from_rgb(84, 129, 230)),
        Switch => Some(Color32::from_rgb(255, 192, 203)),
        Generic => Some(Color32::from_rgb(42, 189, 218)),
        Error => Some(Color32::from_rgb(255, 0, 0)),
        Comment => None,
        Divider => None,
    }
});

static GRAYSCALE_DEPTH_BLOCKS: BlocksTheme = BlocksTheme::new(|block_type, depth| {
    if block_type == BlockType::Divider || block_type == BlockType::Comment {
        return None;
    }

    // get darker as depth increases
    match depth {
        0 => Some(Color32::from_rgb(0x8D, 0x98, 0xAB)),
        1 => Some(Color32::from_rgb(0x77, 0x82, 0x95)),
        2 => Some(Color32::from_rgb(0x61, 0x6C, 0x7F)),
        3 => Some(Color32::from_rgb(0x4C, 0x56, 0x6A)),
        _ => Some(Color32::from_rgb(0x3B, 0x42, 0x55)),
    }
});

static ALTERNATING_COLORED_BLOCKS: BlocksTheme = BlocksTheme::new(|block_type, depth| {
    if block_type == BlockType::Divider || block_type == BlockType::Comment {
        return None;
    }

    match depth % 2 {
        0 => Some(one_dark::BLUE),
        1 => Some(one_dark::GREEN),
        _ => unreachable!("mod 2"),
    }
});
