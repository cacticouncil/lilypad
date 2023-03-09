use druid::{EventCtx, MouseEvent};
use tree_sitter_c2rust::InputEdit;

use super::{line_len, os_linebreak, BlockEditor, IntPoint, Selection, FONT_HEIGHT, FONT_WIDTH};
use crate::vscode;

impl BlockEditor {
    /* ------------------------------ Editing Text ------------------------------ */
    pub fn insert_str(&mut self, source: &mut String, add: &str) {
        // update source
        let old_selection = self.selection.ordered();
        let offsets = old_selection.offset_in(source);
        source.replace_range(offsets.as_range(), add);

        // move cursor
        self.selection = Selection::new_cursor(
            old_selection.start.x + add.chars().count(),
            old_selection.start.y,
        );

        // update tree
        let edits = InputEdit {
            start_byte: offsets.start,
            old_end_byte: offsets.end,
            new_end_byte: offsets.start + add.len(),
            start_position: old_selection.start.as_tree_sitter(),
            old_end_position: old_selection.end.as_tree_sitter(),
            new_end_position: self.selection.end.as_tree_sitter(),
        };
        self.tree_manager.borrow_mut().update(source, edits);

        // update vscode
        Self::send_vscode_edit(add, old_selection);
    }

    pub fn insert_newline(&mut self, source: &mut String) {
        // TODO: maintain indent level
        let old_selection = self.selection.ordered();

        // update source
        let offsets = old_selection.ordered().offset_in(source);
        source.replace_range(offsets.as_range(), os_linebreak());

        // move cursor
        self.selection = Selection::new_cursor(0, old_selection.start.y + 1);

        // update tree
        let edits = InputEdit {
            start_byte: offsets.start,
            old_end_byte: offsets.end,
            new_end_byte: offsets.start + os_linebreak().len(),
            start_position: old_selection.start.as_tree_sitter(),
            old_end_position: old_selection.end.as_tree_sitter(),
            new_end_position: self.selection.end.as_tree_sitter(),
        };
        self.tree_manager.borrow_mut().update(source, edits);

        // update vscode
        Self::send_vscode_edit(os_linebreak(), old_selection);
    }

    pub fn backspace(&mut self, source: &mut String) {
        let old_selection = self.selection.ordered();

        // for normal cursor, delete preceding character
        if old_selection.is_cursor() {
            // move cursor
            if old_selection.start.x == 0 {
                // abort if in position (0,0)
                if old_selection.start.y == 0 {
                    return;
                }

                // Move to the end of the line above.
                // Done before string modified so if a newline is deleted,
                // the cursor is sandwiched between the two newly joined lines.
                let above = old_selection.start.y - 1;
                self.selection = Selection::new_cursor(line_len(above, source), above);
            } else {
                // just move back one char
                self.selection =
                    Selection::new_cursor(old_selection.start.x - 1, old_selection.start.y);
            }

            // update source
            let offset = old_selection.start.offset_in(source);
            let removed = source.remove(offset - 1);

            // update tree
            let edits = InputEdit {
                start_byte: offset,
                old_end_byte: offset,
                new_end_byte: offset - removed.len_utf8(),
                start_position: old_selection.start.as_tree_sitter(),
                old_end_position: old_selection.start.as_tree_sitter(),
                new_end_position: self.selection.start.as_tree_sitter(),
            };
            self.tree_manager.borrow_mut().update(source, edits);

            // update vscode
            // FIXME: delete at start of line
            vscode::edited(
                "",
                old_selection.start.y,
                old_selection.start.x - 1,
                old_selection.start.y,
                old_selection.start.x,
            )
        }
        // for selection, delete text inside
        else {
            // set cursor to start of selection
            self.selection = Selection::new_cursor(old_selection.start.x, old_selection.start.y);

            // remove everything in range
            let offsets = old_selection.offset_in(source);
            source.replace_range(offsets.as_range(), "");

            // update tree
            let edits = InputEdit {
                start_byte: offsets.start,
                old_end_byte: offsets.end,
                new_end_byte: offsets.start,
                start_position: old_selection.start.as_tree_sitter(),
                old_end_position: old_selection.end.as_tree_sitter(),
                new_end_position: old_selection.start.as_tree_sitter(),
            };
            self.tree_manager.borrow_mut().update(source, edits);

            // update vscode
            Self::send_vscode_edit("", old_selection);
        }
    }

    fn send_vscode_edit(text: &str, range: Selection) {
        vscode::edited(text, range.start.y, range.start.x, range.end.y, range.end.x)
    }

    /* ----------------------------- Cursor Movement ---------------------------- */
    pub fn cursor_up(&mut self, source: &str) {
        // when moving up, use top of selection
        let cursor_pos = self.selection.ordered().start;

        self.selection = if cursor_pos.y == 0 {
            Selection::new_cursor(0, 0)
        } else {
            // the normal text editor experience has a "memory" of how far right
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
        let x = (mouse.pos.x / FONT_WIDTH).round() as usize;
        let y = (mouse.pos.y / FONT_HEIGHT) as usize;
        self.selection = Selection::new_cursor(clamp_col(y, x, source), y);

        // request keyboard focus if not already focused
        if !ctx.is_focused() {
            ctx.request_focus();
        }
    }

    pub fn mouse_dragged(&mut self, mouse: &MouseEvent, source: &str, ctx: &mut EventCtx) {
        // set selection end position to new position
        let x = (mouse.pos.x / FONT_WIDTH).round() as usize;
        let y = (mouse.pos.y / FONT_HEIGHT) as usize;
        self.selection.end = IntPoint::new(clamp_col(y, x, source), y);

        // request keyboard focus if not already focused
        if !ctx.is_focused() {
            ctx.request_focus();
        }
    }
}

fn clamp_col(row: usize, col: usize, source: &str) -> usize {
    std::cmp::min(col, line_len(row, source))
}
