use druid::{Color, PaintCtx, Point, Rect, RenderContext, Size};
use std::cmp::Ordering;

use super::{block_drawer, line_len, BlockEditor, FONT_HEIGHT, FONT_WIDTH};

impl BlockEditor {
    pub fn draw_blocks(&self, ctx: &mut PaintCtx) {
        let tree_manager = self.tree_manager.borrow();
        let mut cursor = tree_manager.get_cursor();
        let blocks = block_drawer::blocks_for_tree(&mut cursor);
        block_drawer::draw_blocks(blocks, ctx);
    }

    pub fn draw_cursor(&self, ctx: &mut PaintCtx) {
        // we want to draw the cursor where the mouse has last been (selection end)
        let block = Rect::from_origin_size(
            Point::new(
                (self.selection.end.x as f64) * FONT_WIDTH,
                (self.selection.end.y as f64) * FONT_HEIGHT,
            ),
            Size::new(2.0, FONT_HEIGHT),
        );
        if self.cursor_visible {
            //colors cursor
            ctx.fill(block, &Color::WHITE);
        }
    }

    pub fn draw_selection(&self, source: &str, ctx: &mut PaintCtx) {
        let start_y = self.selection.start.y;
        let end_y = self.selection.end.y;

        match end_y.cmp(&start_y) {
            Ordering::Greater => {
                // Forward selection, multiple lines
                // Fill first line from cursor to end
                selection_block(
                    self.selection.start.x,
                    self.selection.start.y,
                    line_len(self.selection.start.y, source) - self.selection.start.x,
                    ctx,
                );

                // fill in any in between lines
                for line in (start_y + 1)..end_y {
                    selection_block(0, line, line_len(line, source), ctx);
                }

                // Fill last line from the left until cursor
                selection_block(0, self.selection.end.y, self.selection.end.x, ctx);
            }
            Ordering::Less => {
                // Backwards selection, multiple lines

                // Fill first line from cursor to beginning
                selection_block(0, self.selection.start.y, self.selection.start.x, ctx);

                // fill in between lines
                for line in (end_y + 1)..start_y {
                    selection_block(0, line, line_len(line, source), ctx);
                }

                // Fill last line from the right until cursor
                selection_block(
                    self.selection.end.x,
                    self.selection.end.y,
                    line_len(self.selection.end.y, source) - self.selection.end.x,
                    ctx,
                );
            }
            Ordering::Equal => {
                // Just one line
                let ord_sel = self.selection.ordered();
                selection_block(
                    ord_sel.start.x,
                    ord_sel.start.y,
                    ord_sel.end.x - ord_sel.start.x,
                    ctx,
                );
            }
        }
    }
}

fn selection_block(x: usize, y: usize, width: usize, ctx: &mut PaintCtx) {
    let block = Rect::from_origin_size(
        Point::new(x as f64 * FONT_WIDTH, y as f64 * FONT_HEIGHT),
        Size::new(width as f64 * FONT_WIDTH, FONT_HEIGHT),
    );
    ctx.fill(block, &Color::rgba(0.255, 0.255, 0.255, 0.5));
}
