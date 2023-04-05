use druid::{EventCtx, MouseEvent};

use super::{line_len, BlockEditor, IntPoint, Selection, FONT_HEIGHT, FONT_WIDTH};

impl BlockEditor {
    /* ----------------------------- Cursor Movement ---------------------------- */
    pub fn cursor_up(&mut self, source: &str) {
        // when moving up, use top of selection
        let cursor_pos = self.selection.ordered().start;

        self.selection = if cursor_pos.y == 0 {
            Selection::new_cursor(0, 0)
        } else {
            // TODO: the normal text editor experience has a "memory" of how far right
            // the cursor started during a chain for arrow up/down (and then it snaps back there).
            // if that memory is implemented, it can replace self.cursor_pos.x
            Selection::new_cursor(
                clamp_col(cursor_pos.y - 1, cursor_pos.x, source),
                cursor_pos.y - 1,
            )
        }
    }

    pub fn cursor_down(&mut self, source: &str) {
        // when moving down use bottom of selection
        let cursor_pos = self.selection.ordered().end;

        let last_line = source.lines().count() - 1;
        let next_line = std::cmp::min(cursor_pos.y + 1, last_line);

        self.selection = if cursor_pos.y == last_line {
            // if on last line, just move to end of line
            Selection::new_cursor(
                source.lines().last().unwrap_or("").chars().count(),
                last_line,
            )
        } else {
            // same memory thing as above applies here
            Selection::new_cursor(clamp_col(next_line, cursor_pos.x, source), next_line)
        }
    }

    pub fn cursor_left(&mut self, source: &str) {
        if self.selection.is_cursor() {
            // actually move if cursor
            let cursor_pos = self.selection.start;
            if cursor_pos.x == 0 {
                // if at start of line, move to end of line above
                if cursor_pos.y != 0 {
                    self.selection =
                        Selection::new_cursor(line_len(cursor_pos.y - 1, source), cursor_pos.y - 1);
                }
            } else {
                self.selection = Selection::new_cursor(cursor_pos.x - 1, cursor_pos.y);
            }
        } else {
            // just move cursor to start of selection
            let start = self.selection.ordered().start;
            self.selection = Selection::new_cursor(start.x, start.y);
        }
    }

    pub fn cursor_right(&mut self, source: &str) {
        if self.selection.is_cursor() {
            // actually move if cursor
            let cursor_pos = self.selection.start;

            let curr_line_len = line_len(cursor_pos.y, source);
            if cursor_pos.x == curr_line_len {
                // if at end of current line, go to next line
                let last_line = source.lines().count() - 1;
                if cursor_pos.y != last_line {
                    self.selection = Selection::new_cursor(0, cursor_pos.y + 1);
                }
            } else {
                self.selection = Selection::new_cursor(cursor_pos.x + 1, cursor_pos.y);
            }
        } else {
            // just move cursor to end of selection
            let end = self.selection.ordered().end;
            self.selection = Selection::new_cursor(end.x, end.y);
        }
    }

    pub fn cursor_to_line_start(&mut self, source: &str) {
        // go with whatever line the mouse was last on
        let cursor_pos = self.selection.end;

        let line = source.lines().nth(cursor_pos.y).unwrap_or("");
        let start_idx = line.len() - line.trim_start().len();
        self.selection = Selection::new_cursor(start_idx, cursor_pos.y);
    }

    pub fn cursor_to_line_end(&mut self, source: &str) {
        // go with whatever line the mouse was last on
        let cursor_pos = self.selection.end;

        self.selection = Selection::new_cursor(line_len(cursor_pos.y, source), cursor_pos.y);
    }

    /* ------------------------------ Mouse Clicks ------------------------------ */
    pub fn mouse_clicked(&mut self, mouse: &MouseEvent, source: &str, ctx: &mut EventCtx) {
        // move the cursor and get selection start position
        let loc = self.mouse_to_coord(mouse, source);
        self.selection.start = loc;
        self.selection.end = loc;

        // request keyboard focus if not already focused
        if !ctx.is_focused() {
            ctx.request_focus();
        }
    }

    pub fn mouse_dragged(&mut self, mouse: &MouseEvent, source: &str, _ctx: &mut EventCtx) {
        // set selection end position to dragged position
        self.selection.end = self.mouse_to_coord(mouse, source);
    }

    fn mouse_to_coord(&self, mouse: &MouseEvent, source: &str) -> IntPoint {
        // find the line clicked on by finding the next one and then going back one
        let mut y: usize = 0;
        let mut total_pad = 0.0;
        // do check the last padding since there is no line there
        for row_pad in &self.padding[..(self.padding.len() - 1)] {
            total_pad += row_pad;
            let curr_line_start = total_pad + (y as f64 * FONT_HEIGHT);
            let raw_y = mouse.pos.y - super::OUTER_PAD;
            if raw_y <= curr_line_start {
                break;
            }
            y += 1;
        }
        y = y.saturating_sub(1);

        let x_raw =
            ((mouse.pos.x - super::OUTER_PAD - super::TEXT_L_PAD) / FONT_WIDTH).round() as usize;
        let x_bound = clamp_col(y, x_raw, source);

        IntPoint::new(x_bound, y)
    }
}

fn clamp_col(row: usize, col: usize, source: &str) -> usize {
    std::cmp::min(col, line_len(row, source))
}
