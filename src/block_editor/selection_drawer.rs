use druid::{PaintCtx, Point, Rect, RenderContext, Size};
use std::cmp::{max, Ordering};

use crate::theme;

use super::{line_len, BlockEditor, FONT_HEIGHT, FONT_WIDTH};

impl BlockEditor {
    pub fn draw_cursor(&self, ctx: &mut PaintCtx) {
        if self.cursor_visible {
            // we want to draw the cursor where the mouse has last been (selection end)
            let total_pad: f64 = self.padding.iter().take(self.selection.end.y + 1).sum();
            let block = Rect::from_origin_size(
                Point::new(
                    super::TOTAL_TEXT_X_OFFSET + (self.selection.end.x as f64) * FONT_WIDTH,
                    super::OUTER_PAD + (self.selection.end.y as f64) * FONT_HEIGHT + total_pad,
                ),
                Size::new(2.0, FONT_HEIGHT),
            );

            ctx.fill(block, &theme::CURSOR);
        }
    }

    pub fn draw_selection(&self, source: &str, ctx: &mut PaintCtx) {
        let start_y = self.selection.start.y;
        let end_y = self.selection.end.y;

        match end_y.cmp(&start_y) {
            Ordering::Greater => {
                // Forward selection, multiple lines
                // Fill first line from cursor to end
                self.selection_block(
                    self.selection.start.x,
                    self.selection.start.y,
                    line_len(self.selection.start.y, source) - self.selection.start.x,
                    false,
                    ctx,
                );

                // fill in any in between lines
                for line in (start_y + 1)..end_y {
                    self.selection_block(0, line, max(line_len(line, source), 1), true, ctx);
                }

                // Fill last line from the left until cursor
                self.selection_block(0, self.selection.end.y, self.selection.end.x, true, ctx);
            }
            Ordering::Less => {
                // Backwards selection, multiple lines

                // Fill first line from cursor to beginning
                self.selection_block(0, self.selection.start.y, self.selection.start.x, true, ctx);

                // fill in between lines
                for line in (end_y + 1)..start_y {
                    self.selection_block(0, line, max(line_len(line, source), 1), true, ctx);
                }

                // Fill last line from the right until cursor
                self.selection_block(
                    self.selection.end.x,
                    self.selection.end.y,
                    line_len(self.selection.end.y, source) - self.selection.end.x,
                    false,
                    ctx,
                );
            }
            Ordering::Equal => {
                // Just one line
                let ord_sel = self.selection.ordered();
                self.selection_block(
                    ord_sel.start.x,
                    ord_sel.start.y,
                    ord_sel.end.x - ord_sel.start.x,
                    false,
                    ctx,
                );
            }
        }
    }

    fn selection_block(
        &self,
        x: usize,
        y: usize,
        width: usize,
        chained_below: bool,
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
        ctx.fill(block, &theme::SELECTION);
    }
}
