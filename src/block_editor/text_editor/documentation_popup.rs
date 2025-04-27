use crate::{
    block_editor::{blocks::Padding, MonospaceFont, OUTER_PAD, TOTAL_TEXT_X_OFFSET},
    lsp::documentation::Documentation,
};
use egui::{Pos2, Vec2};
use egui_commonmark::CommonMarkCache;
pub struct DocumentationPopup {
    pub markdown_cache: CommonMarkCache,
}

impl DocumentationPopup {
    pub fn new() -> Self {
        let mut cache = CommonMarkCache::default();
        let theme_bytes = include_bytes!("../../theme/hover_theme.tmTheme");
        let _ = cache.add_syntax_theme_from_bytes("hover_theme", theme_bytes);
        Self {
            markdown_cache: cache,
        }
    }

    pub fn widget<'a>(&'a mut self, ui: &mut egui::Ui, documentation: &'a Documentation) {
        ui.style_mut().url_in_tooltip = true;
        //todo: clicking on a link in the rust hover info blows up evertyihng
        let _viewer = egui_commonmark::CommonMarkViewer::new()
            .syntax_theme_dark("hover_theme")
            .show(ui, &mut self.markdown_cache, &documentation.message);
    }

    pub fn calc_origin(
        &mut self,
        documentation: &Documentation,
        offset: Vec2,
        padding: &Padding,
        font: &MonospaceFont,
    ) -> Pos2 {
        //Largely based obn diagnostic_popup.rs version
        let total_padding = padding.cumulative(documentation.range.start.line + 1);
        let x =
            TOTAL_TEXT_X_OFFSET + (documentation.range.start.col as f32 * font.size.x) + offset.x;
        let y = OUTER_PAD
            + total_padding
            + (documentation.range.start.line as f32 * font.size.y)
            + offset.y
            + 8.8; //This puts it at about midpoint of the line, 10 looks better but it goes away before the mouse moves on to it, maybe 9.5 is better.
                   //FIX: LITTLE GAP BETWEEN TEXT AND POPUP GOES AWAY, COULDN'T FIGURE OUT.
        Pos2::new(x, y)
    }
}

impl Documentation {}
