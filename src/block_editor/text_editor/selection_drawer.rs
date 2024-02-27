use druid::{Color, PaintCtx, Point, Rect, RenderContext, Size};
use ropey::Rope;

use super::TextEditor;
use crate::{
    block_editor::{
        text_range::TextRange, FONT_HEIGHT, FONT_WIDTH, OUTER_PAD, TOTAL_TEXT_X_OFFSET,
    },
    theme,
};

impl TextEditor {
    pub fn draw_cursor(&self, ctx: &mut PaintCtx) {
        if self.cursor_visible {
            // we want to draw the cursor where the mouse has last been (selection end)
            let total_pad: f64 = self.padding.iter().take(self.selection.end.line + 1).sum();
            let block = Rect::from_origin_size(
                Point::new(
                    TOTAL_TEXT_X_OFFSET
                        + (self.selection.end.col as f64) * FONT_WIDTH.get().unwrap(),
                    OUTER_PAD
                        + (self.selection.end.line as f64) * FONT_HEIGHT.get().unwrap()
                        + total_pad,
                ),
                Size::new(2.0, *FONT_HEIGHT.get().unwrap()),
            );

            ctx.fill(block, &theme::CURSOR);
        }
    }

    pub fn draw_selection(&self, source: &Rope, ctx: &mut PaintCtx) {
        if !self.selection.is_cursor() {
            self.draw_selection_blocks(self.selection, source, &theme::SELECTION, ctx);
        }
    }

    pub fn draw_pseudo_selection(&self, source: &Rope, ctx: &mut PaintCtx) {
        if let Some(selection) = self.pseudo_selection {
            self.draw_selection_blocks(selection, source, &theme::PSEUDO_SELECTION, ctx);
        }
    }

    fn draw_selection_blocks(
        &self,
        selection: TextRange,
        source: &Rope,
        color: &Color,
        ctx: &mut PaintCtx,
    ) {
        let selection = selection.ordered();
        let line_ranges = selection.individual_lines(source);

        // start the the total padding through the first line so the selection
        // block is placed on the text of the first line (instead of the padding above it)
        let mut total_padding: f64 = self.padding.iter().take(selection.start.line + 1).sum();

        for line_range in line_ranges {
            // one line per range so the line number is the start of the range
            let line_num = line_range.start.line;

            // find width of selection block in chars
            let width = line_range.end.col - line_range.start.col
                + if line_num != selection.end.line { 1 } else { 0 }; // 1 is added to the width to include the newline

            self.draw_selection_block(
                line_num,
                line_range.start.col,
                width,
                total_padding,
                line_num != selection.start.line,
                color,
                ctx,
            );

            // the padding for the first line was adding before the loop
            if line_num != selection.start.line {
                total_padding += self.padding[line_num];
            }
        }
    }

    fn draw_selection_block(
        &self,
        line: usize,
        col: usize,
        width: usize,
        padding_above: f64,
        has_block_above: bool,
        color: &Color,
        ctx: &mut PaintCtx,
    ) {
        let font_width = *FONT_WIDTH.get().unwrap();
        let font_height = *FONT_HEIGHT.get().unwrap();

        let line_padding = if has_block_above {
            self.padding[line]
        } else {
            0.0
        };

        let block = Rect::from_origin_size(
            Point::new(
                (col as f64 * font_width) + TOTAL_TEXT_X_OFFSET,
                (line as f64 * font_height) + OUTER_PAD + padding_above,
            ),
            Size::new(width as f64 * font_width, font_height + line_padding),
        );

        ctx.fill(block, color);
    }
}
