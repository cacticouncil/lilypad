use druid::{Color, PaintCtx, Point, Rect, RenderContext, Size};
use ropey::Rope;
use std::cmp::{max, Ordering};

use crate::theme;

use super::{rope_ext::RopeExt, text_range::TextRange, BlockEditor, FONT_HEIGHT, FONT_WIDTH};

impl BlockEditor {
    pub fn draw_cursor(&self, ctx: &mut PaintCtx) {
        if self.cursor_visible {
            // we want to draw the cursor where the mouse has last been (selection end)
            let total_pad: f64 = self.padding.iter().take(self.selection.end.row + 1).sum();
            let block = Rect::from_origin_size(
                Point::new(
                    super::TOTAL_TEXT_X_OFFSET + (self.selection.end.col as f64) * FONT_WIDTH,
                    super::OUTER_PAD + (self.selection.end.row as f64) * FONT_HEIGHT + total_pad,
                ),
                Size::new(2.0, FONT_HEIGHT),
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
        let start_y = selection.start.row;
        let end_y = selection.end.row;

        match end_y.cmp(&start_y) {
            Ordering::Greater => {
                // Forward selection, multiple lines
                // Fill first line from cursor to end
                self.draw_selection_block(
                    selection.start.col,
                    selection.start.row,
                    source.len_char_for_line(selection.start.row) - selection.start.col,
                    false,
                    color,
                    ctx,
                );

                // fill in any in between lines
                for line in (start_y + 1)..end_y {
                    self.draw_selection_block(
                        0,
                        line,
                        max(source.len_char_for_line(line), 1),
                        true,
                        color,
                        ctx,
                    );
                }

                // Fill last line from the left until cursor
                self.draw_selection_block(
                    0,
                    selection.end.row,
                    selection.end.col,
                    true,
                    color,
                    ctx,
                );
            }
            Ordering::Less => {
                // Backwards selection, multiple lines

                // Fill first line from cursor to beginning
                self.draw_selection_block(
                    0,
                    selection.start.row,
                    selection.start.col,
                    true,
                    color,
                    ctx,
                );

                // fill in between lines
                for line in (end_y + 1)..start_y {
                    self.draw_selection_block(
                        0,
                        line,
                        max(source.len_char_for_line(line), 1),
                        true,
                        color,
                        ctx,
                    );
                }

                // Fill last line from the right until cursor
                self.draw_selection_block(
                    selection.end.col,
                    selection.end.row,
                    source.len_char_for_line(selection.end.row) - selection.end.col,
                    false,
                    color,
                    ctx,
                );
            }
            Ordering::Equal => {
                // Just one line
                let ord_sel = selection.ordered();
                self.draw_selection_block(
                    ord_sel.start.col,
                    ord_sel.start.row,
                    ord_sel.end.col - ord_sel.start.col,
                    false,
                    color,
                    ctx,
                );
            }
        }
    }

    fn draw_selection_block(
        &self,
        x: usize,
        y: usize,
        width: usize,
        chained_below: bool,
        color: &Color,
        ctx: &mut PaintCtx,
    ) {
        // TODO: don't calculate every time
        let total_pad: f64 = self
            .padding
            .iter()
            .take(if chained_below { y } else { y + 1 })
            .sum();

        let block = Rect::from_origin_size(
            Point::new(
                (x as f64 * FONT_WIDTH) + super::TOTAL_TEXT_X_OFFSET,
                (y as f64 * FONT_HEIGHT) + super::OUTER_PAD + total_pad,
            ),
            Size::new(
                width as f64 * FONT_WIDTH,
                FONT_HEIGHT + if chained_below { self.padding[y] } else { 0.0 },
            ),
        );
        ctx.fill(block, color);
    }
}
