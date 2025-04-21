use std::num;

use crate::{
    block_editor::{blocks::Padding, MonospaceFont, OUTER_PAD, TOTAL_TEXT_X_OFFSET},
    lsp::documentation::Documentation,
    theme,
};
use egui::{Align2, Color32, Painter, Pos2, Response, RichText, Ui, Vec2, Widget};
use egui_commonmark::CommonMarkCache;
use ropey::Rope;

pub struct DocumentationPopup {
    pub markdown_cache: CommonMarkCache,
    pub is_hovered: bool,
    pub is_above: bool,
    pub popup_size: Vec2,
}

impl DocumentationPopup {
    pub fn new() -> Self {
        Self {
            markdown_cache: CommonMarkCache::default(),
            is_hovered: false,
            popup_size: Vec2::new(0.0, 0.0),
            is_above: false,
        }
    }

    pub fn make_rect(&self, pos: Pos2) -> egui::Rect {
        egui::Rect::from_min_size(pos, self.popup_size)
    }

    pub fn widget<'a>(
        &'a mut self,
        ui: &mut egui::Ui,
        documentation: &'a Documentation,
        font: &'a MonospaceFont,
    ) {
        ui.style_mut().url_in_tooltip = true;
        //todo: clicking on a link in the rust hover info blows up evertyihng
        egui_commonmark::CommonMarkViewer::new().show(
            ui,
            &mut self.markdown_cache,
            &documentation.message,
        );
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

impl Documentation {
    #[warn(dead_code)]
    pub fn draw(
        &self,
        padding: &Padding,
        source: &Rope,
        _offset: Vec2,
        _font: &MonospaceFont,
        _painter: &Painter,
    ) {
        let range = self.range.ordered();
        let line_ranges = range.individual_lines(source);
    }
}
