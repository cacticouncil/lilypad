use druid::{EventCtx, MouseEvent};
use ropey::Rope;
use tree_sitter_c2rust::{Node, Point, TreeCursor};

use super::{
    rope_ext::{RopeExt, RopeSliceExt},
    BlockEditor, TextPoint, TextRange, FONT_HEIGHT, FONT_WIDTH,
};

impl BlockEditor {
    fn set_selection(&mut self, selection: TextRange, source: &Rope) {
        self.selection = selection;
        self.find_pseudo_selection(source);

        // make cursor visible whenver moved
        self.cursor_visible = true;

        // clear input ignore stack
        self.input_ignore_stack.clear();
        self.paired_delete_stack.clear();
    }

    pub fn find_pseudo_selection(&mut self, source: &Rope) {
        self.pseudo_selection = None;
        if self.selection.is_cursor() {
            // find if the cursor is after a quote
            let cursor = self.selection.start;
            let cursor_offset = cursor.char_idx_in(source);
            let (prev_char, _) = source.surrounding_chars(cursor_offset);

            if prev_char == '"' || prev_char == '\'' {
                // find the node for the string
                // string will always be the lowest level
                // don't set if error (bc that would make things go wonky when unpaired)
                let Some(node) = lowest_non_err_named_node_for_point(
                    self.tree_manager.get_cursor(),
                    cursor.into(),
                ) else { return };

                // set pseudo selection to node
                self.pseudo_selection = Some(TextRange::new(
                    node.start_position().into(),
                    node.end_position().into(),
                ));
            }
        }
    }

    /* ----------------------------- Cursor Movement ---------------------------- */
    pub fn cursor_up(&mut self, source: &Rope) {
        // when moving up, use top of selection
        let cursor_pos = self.selection.ordered().start;

        let selection = if cursor_pos.row == 0 {
            TextRange::ZERO
        } else {
            // TODO: the normal text editor experience has a "memory" of how far right
            // the cursor started during a chain for arrow up/down (and then it snaps back there).
            // if that memory is implemented, it can replace self.cursor_pos.x
            TextRange::new_cursor(
                clamp_col(cursor_pos.row - 1, cursor_pos.col, source),
                cursor_pos.row - 1,
            )
        };
        self.set_selection(selection, source);
    }

    pub fn cursor_down(&mut self, source: &Rope) {
        // when moving down use bottom of selection
        let cursor_pos = self.selection.ordered().end;

        let last_line = source.lines().count() - 1;
        let next_line = std::cmp::min(cursor_pos.row + 1, last_line);

        let selection = if cursor_pos.row == last_line {
            // if on last line, just move to end of line
            TextRange::new_cursor(
                source
                    .get_line(source.len_lines() - 1)
                    .map_or(0, |line| line.len_chars()),
                last_line,
            )
        } else {
            // same memory thing as above applies here
            TextRange::new_cursor(clamp_col(next_line, cursor_pos.col, source), next_line)
        };
        self.set_selection(selection, source);
    }

    pub fn cursor_left(&mut self, source: &Rope) {
        let selection = if self.selection.is_cursor() {
            // actually move if cursor
            let cursor_pos = self.selection.start;
            if cursor_pos.col == 0 {
                // if at start of line, move to end of line above
                if cursor_pos.row != 0 {
                    TextRange::new_cursor(
                        source.len_char_for_line(cursor_pos.row - 1),
                        cursor_pos.row - 1,
                    )
                } else {
                    // already at top left
                    return;
                }
            } else {
                TextRange::new_cursor(cursor_pos.col - 1, cursor_pos.row)
            }
        } else {
            // just move cursor to start of selection
            let start = self.selection.ordered().start;
            TextRange::new_cursor(start.col, start.row)
        };
        self.set_selection(selection, source);
    }

    pub fn cursor_right(&mut self, source: &Rope) {
        let selection = if self.selection.is_cursor() {
            // actually move if cursor
            let cursor_pos = self.selection.start;

            let curr_line_len = source.len_char_for_line(cursor_pos.row);
            if cursor_pos.col == curr_line_len {
                // if at end of current line, go to next line
                let last_line = source.len_lines() - 1;
                if cursor_pos.row != last_line {
                    TextRange::new_cursor(0, cursor_pos.row + 1)
                } else {
                    // already at end
                    return;
                }
            } else {
                TextRange::new_cursor(cursor_pos.col + 1, cursor_pos.row)
            }
        } else {
            // just move cursor to end of selection
            let end = self.selection.ordered().end;
            TextRange::new_cursor(end.col, end.row)
        };
        self.set_selection(selection, source);
    }

    pub fn cursor_to_line_start(&mut self, source: &Rope) {
        // go with whatever line the mouse was last on
        let cursor_pos = self.selection.end;

        let start_idx = source.line(cursor_pos.row).whitespace_at_start();
        let selection = TextRange::new_cursor(start_idx, cursor_pos.row);
        self.set_selection(selection, source);
    }

    pub fn cursor_to_line_end(&mut self, source: &Rope) {
        // go with whatever line the mouse was last on
        let cursor_pos = self.selection.end;

        let selection =
            TextRange::new_cursor(source.len_char_for_line(cursor_pos.row), cursor_pos.row);
        self.set_selection(selection, source);
    }

    /* ------------------------------ Mouse Clicks ------------------------------ */
    pub fn mouse_clicked(&mut self, mouse: &MouseEvent, source: &Rope, ctx: &mut EventCtx) {
        // move the cursor and get selection start position
        let loc = self.mouse_to_coord(mouse, source);
        let selection = TextRange::new(loc, loc);
        self.set_selection(selection, source);

        // request keyboard focus if not already focused
        if !ctx.is_focused() {
            ctx.request_focus();
        }
    }

    pub fn mouse_dragged(&mut self, mouse: &MouseEvent, source: &Rope, _ctx: &mut EventCtx) {
        // set selection end position to dragged position
        self.selection.end = self.mouse_to_coord(mouse, source);

        // clear pseudo selection
        self.pseudo_selection = None;

        // show cursor
        self.cursor_visible = true;
    }

    pub fn mouse_to_coord(&self, mouse: &MouseEvent, source: &Rope) -> TextPoint {
        // find the line clicked on by finding the next one and then going back one
        let mut y: usize = 0;
        let mut total_pad = 0.0;
        for row_pad in &self.padding {
            total_pad += row_pad;
            let curr_line_start = total_pad + (y as f64 * FONT_HEIGHT);
            let raw_y = mouse.pos.y - super::OUTER_PAD;
            if raw_y <= curr_line_start {
                break;
            }
            y += 1;
        }
        y = y.saturating_sub(1);

        // TODO: if past last line, move to end of last line

        let x_raw = ((mouse.pos.x - super::OUTER_PAD - super::GUTTER_WIDTH - super::TEXT_L_PAD)
            / FONT_WIDTH)
            .round() as usize;
        let x_bound = clamp_col(y, x_raw, source);

        TextPoint::new(x_bound, y)
    }
}

fn clamp_col(row: usize, col: usize, source: &Rope) -> usize {
    std::cmp::min(col, source.len_char_for_line(row))
}

fn lowest_non_err_named_node_for_point(mut cursor: TreeCursor, point: Point) -> Option<Node> {
    // go to lowest node for point
    while cursor.goto_first_child_for_point(point).is_some() {
        if cursor.node().is_error() {
            return None;
        }
    }

    // if landed on a unnamed node, find first named parent
    while !cursor.node().is_named() && cursor.goto_parent() {}

    Some(cursor.node())
}
