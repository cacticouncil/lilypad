use druid::{piet::PietText, PaintCtx, Point, Size};
use ropey::Rope;

use crate::{
    block_editor::{
        block_drawer::{self, Block},
        rope_ext::RopeExt,
        text_drawer::TextDrawer,
        FONT_HEIGHT, FONT_WIDTH,
    },
    lang::LanguageConfig,
    parse::TreeManager,
    theme::blocks_theme::BlocksTheme,
};

pub struct LooseBlock {
    text: String,
    blocks: Vec<Block>,
    padding: Vec<f64>,
    size: Size,
    lang: &'static LanguageConfig,
    tree_manager: TreeManager,
    text_drawer: TextDrawer,
    interior_padding: f64,
}

impl LooseBlock {
    pub fn new(lang: &'static LanguageConfig, interior_padding: f64) -> Self {
        Self {
            text: String::new(),
            blocks: vec![],
            padding: vec![],
            size: Size::ZERO,
            lang,
            tree_manager: TreeManager::new(lang),
            text_drawer: TextDrawer::new(lang),
            interior_padding,
        }
    }

    pub fn make_from_text(
        text: &str,
        lang: &'static LanguageConfig,
        interior_padding: f64,
        piet_text: &mut PietText,
    ) -> Self {
        let mut block = Self::new(lang, interior_padding);
        block.set_text(text, piet_text);
        block
    }

    pub fn set_text(&mut self, text: &str, piet_text: &mut PietText) {
        self.text = text.to_string();
        let rope = Rope::from_str(text);

        self.tree_manager.replace(&rope);
        self.text_drawer
            .layout(self.tree_manager.get_cursor().node(), &rope, piet_text);

        // find blocks
        self.blocks =
            block_drawer::blocks_for_tree(&mut self.tree_manager.get_cursor(), &rope, self.lang);
        self.padding = block_drawer::make_padding(&self.blocks, rope.len_lines());

        // find dimensions
        let max_chars = rope.lines().map(|l| l.len_chars()).max().unwrap_or(0);
        let width = max_chars as f64 * FONT_WIDTH.get().unwrap() + self.interior_padding;
        let line_count = rope.len_lines() - if rope.ends_with('\n') { 1 } else { 0 };
        let height =
            (FONT_HEIGHT.get().unwrap() * line_count as f64) + self.padding.iter().sum::<f64>();
        self.size = Size::new(width, height);
    }

    pub fn change_language(&mut self, lang: &'static LanguageConfig) {
        self.tree_manager.change_language(lang);
        self.text_drawer.change_language(lang);
        self.blocks.clear();
        self.padding.clear();
    }

    pub fn draw(&self, offset: Point, width: f64, block_theme: BlocksTheme, ctx: &mut PaintCtx) {
        block_drawer::draw_blocks(&self.blocks, offset, width, block_theme, ctx);
        self.text_drawer.draw(&self.padding, offset, ctx);
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}
