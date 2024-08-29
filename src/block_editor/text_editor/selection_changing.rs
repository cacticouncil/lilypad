use egui::{Modifiers, Pos2};
use ropey::Rope;
use tree_sitter_c2rust::TreeCursor;

use super::{text_editing::TextMovement, TextEditor};
use crate::{
    block_editor::{
        rope_ext::{RopeExt, RopeSliceExt},
        DragSession, MonospaceFont, TextPoint, TextRange, GUTTER_WIDTH, OUTER_PAD, TEXT_L_PAD,
        TOTAL_TEXT_X_OFFSET,
    },
    vscode,
};

impl TextEditor {
    // Set the selection as a result of user input
    fn set_selection(&mut self, selection: TextRange) {
        self.selection = selection;
        self.find_pseudo_selection();

        // make cursor visible whenever moved
        self.last_selection_time = self.frame_start_time;

        // clear input ignore stack
        self.input_ignore_stack.clear();
        self.paired_delete_stack.clear();

        // add a separator to the undo stack
        self.undo_manager.add_undo_stop()
    }

    pub fn find_pseudo_selection(&mut self) {
        self.pseudo_selection = None;
        if self.selection.is_cursor() {
            // find if the cursor is after a quote
            let cursor_loc = self.selection.start;
            let cursor_offset = cursor_loc.char_idx_in(&self.source);
            let (prev_char, _) = self.source.surrounding_chars(cursor_offset);

            if prev_char == '"' || prev_char == '\'' {
                self.pseudo_selection = self.string_pseudo_selection_range(
                    self.tree_manager.get_cursor(),
                    cursor_loc.into(),
                );
            }
        }
    }

    fn string_pseudo_selection_range(
        &self,
        mut cursor: TreeCursor,
        point: tree_sitter_c2rust::Point,
    ) -> Option<TextRange> {
        // go to lowest node for point
        // don't set if error (bc that would make things go wonky when unpaired)
        while cursor.goto_first_child_for_point(point).is_some() {
            if cursor.node().is_error() {
                return None;
            }
        }

        // verify that our current point is the start or end of a string (not an escape sequence)
        let current_kind = cursor.node().kind_id();
        let kinds = self.language.string_node_ids;
        if !kinds.string_bounds.contains(&current_kind) {
            return None;
        }

        // go up until we hit the string (node of id 230)
        while cursor.goto_parent() {
            let node = cursor.node();
            if node.kind_id() == kinds.string {
                let range =
                    TextRange::new(node.start_position().into(), node.end_position().into());
                return Some(range);
            }
        }

        // we hit the top without finding a string, just return none
        None
    }

    /* ----------------------------- Cursor Movement ---------------------------- */
    pub fn move_cursor(&mut self, movement: TextMovement) {
        let new_cursor = self
            .selection
            .find_movement_result(movement, &self.source, false);
        self.set_selection(TextRange::new_cursor(new_cursor));
    }

    pub fn move_selecting(&mut self, movement: TextMovement) {
        let new_sel = self.selection.expanded_by(movement, &self.source);
        self.set_selection(new_sel);
    }

    /* ------------------------------ Mouse Clicks ------------------------------ */
    pub fn mouse_clicked(
        &mut self,
        pos: Pos2,
        mods: Modifiers,
        drag_block: &mut Option<DragSession>,
        font: &MonospaceFont,
    ) {
        // if option is held, remove the current block from the source and place it in drag_block
        if mods.alt {
            if drag_block.is_none() {
                self.start_block_drag(pos, drag_block, font);
            }
            return;
        }

        // move the cursor and get selection start position
        let loc = self.mouse_to_coord(pos, font);

        if pos.x < GUTTER_WIDTH {
            // if in the gutter, set a breakpoint
            // TODO: clicking past the end currently adds a breakpoint to the last line, don't do that
            if self.breakpoints.contains(&loc.line) {
                self.breakpoints.remove(&loc.line);
            } else {
                self.breakpoints.insert(loc.line);
            }
            vscode::register_breakpoints(self.breakpoints.iter().cloned().collect());
        } else {
            let selection = TextRange::new_cursor(loc);
            self.set_selection(selection);
        }
    }

    pub fn expand_selection(&mut self, pos: Pos2, font: &MonospaceFont) {
        // set selection end position to dragged position
        let coord = self.mouse_to_coord(pos, font);
        self.selection.end = coord;

        // clear pseudo selection if making a selection
        if !self.selection.is_cursor() {
            self.pseudo_selection = None;
        }

        // show cursor
        self.last_selection_time = self.frame_start_time;
    }

    /* -------------------- UI <-> Text Coordinate Conversion ------------------- */
    pub fn mouse_to_coord(&self, point: Pos2, font: &MonospaceFont) -> TextPoint {
        // find the line clicked on by finding the next one and then going back one
        let mut line: usize = 0;
        let mut total_pad = 0.0;
        for line_pad in &self.padding {
            total_pad += line_pad;
            let curr_line_start = total_pad + (line as f32 * font.size.y);
            let raw_y = point.y - OUTER_PAD;
            if raw_y <= curr_line_start {
                break;
            }
            line += 1;
        }
        line = line.saturating_sub(1);

        // double check that we are in bounds
        // (clicking and deleting at the same time can cause the padding to not be updated yet)
        let line_count = self.source.len_lines();
        if line >= line_count {
            line = line_count - 1;
        }

        // TODO: if past last line, move to end of last line

        let col_raw =
            ((point.x - OUTER_PAD - GUTTER_WIDTH - TEXT_L_PAD) / font.size.x).round() as usize;
        let col_bound = clamp_col(line, col_raw, &self.source);

        TextPoint::new(line, col_bound)
    }

    /// Finds the text coordinate that the mouse is over, without clamping to a valid position within the text
    pub fn mouse_to_raw_coord(&self, point: Pos2, font: &MonospaceFont) -> TextPoint {
        // find the line clicked on by finding the next one and then going back one
        let mut line: usize = 0;
        let mut total_pad = 0.0;
        for line_pad in &self.padding {
            total_pad += line_pad;
            let curr_line_start = total_pad + (line as f32 * font.size.y);
            let raw_y = point.y - OUTER_PAD;
            if raw_y <= curr_line_start {
                break;
            }
            line += 1;
        }

        // add any remaining lines past the last line
        line += ((point.y - (total_pad + (line as f32 * font.size.y))) / font.size.y) as usize;

        line = line.saturating_sub(1);

        let col =
            ((point.x - OUTER_PAD - GUTTER_WIDTH - TEXT_L_PAD) / font.size.x).round() as usize;

        TextPoint::new(line, col)
    }

    pub fn coord_to_mouse(&self, coord: TextPoint, font: &MonospaceFont) -> Pos2 {
        let y = OUTER_PAD
            + (coord.line as f32 * font.size.y)
            + self.padding.iter().take(coord.line).sum::<f32>();
        let x = TOTAL_TEXT_X_OFFSET + (coord.col as f32 * font.size.x);

        Pos2::new(x, y)
    }
}

impl TextRange {
    pub fn expanded_by(&self, movement: TextMovement, source: &Rope) -> TextRange {
        let new_cursor = self.find_movement_result(movement, source, true);
        TextRange::new(self.start, new_cursor)
    }

    pub fn find_movement_result(
        &self,
        movement: TextMovement,
        source: &Rope,
        expanding: bool,
    ) -> TextPoint {
        use super::text_editing::{HDir::*, HUnit, TextMovement::*, VDir::*, VUnit};
        match movement {
            Horizontal { unit, direction } => match unit {
                HUnit::Grapheme => match direction {
                    Left => self.cursor_left(source, expanding),
                    Right => self.cursor_right(source, expanding),
                },
                HUnit::Word => match direction {
                    Left => self.cursor_to_prev_word_start(source),
                    Right => self.cursor_to_next_word_end(source),
                },
                HUnit::Line => match direction {
                    Left => self.cursor_to_line_start(source),
                    Right => self.cursor_to_line_end(source),
                },
            },
            Vertical { unit, direction } => match unit {
                VUnit::Line => match direction {
                    Up => self.cursor_up(source),
                    Down => self.cursor_down(source),
                },
                VUnit::Document => match direction {
                    Up => TextPoint::ZERO,
                    Down => self.cursor_to_doc_end(source),
                },
            },
        }
    }

    fn cursor_up(&self, source: &Rope) -> TextPoint {
        // when moving up, use top of selection
        let cursor_pos = self.ordered().start;

        if cursor_pos.line == 0 {
            TextPoint::ZERO
        } else {
            // TODO: the normal text editor experience has a "memory" of how far right
            // the cursor started during a chain for arrow up/down (and then it snaps back there).
            // if that memory is implemented, it can replace self.cursor_pos.x
            TextPoint::new(
                cursor_pos.line - 1,
                clamp_col(cursor_pos.line - 1, cursor_pos.col, source),
            )
        }
    }

    fn cursor_down(&self, source: &Rope) -> TextPoint {
        // when moving down use bottom of selection
        let cursor_pos = self.ordered().end;

        let last_line = source.lines().count() - 1;
        let next_line = std::cmp::min(cursor_pos.line + 1, last_line);

        if cursor_pos.line == last_line {
            // if on last line, just move to end of line
            TextPoint::new(
                last_line,
                source
                    .get_line(source.len_lines() - 1)
                    .map_or(0, |line| line.len_chars()),
            )
        } else {
            // same memory thing as above applies here
            TextPoint::new(next_line, clamp_col(next_line, cursor_pos.col, source))
        }
    }

    fn cursor_left(&self, source: &Rope, expanding: bool) -> TextPoint {
        if self.is_cursor() || expanding {
            // actually move if cursor
            let cursor_pos = self.end;
            if cursor_pos.col == 0 {
                // if at start of line, move to end of line above
                if cursor_pos.line != 0 {
                    TextPoint::new(
                        cursor_pos.line - 1,
                        source.line(cursor_pos.line - 1).len_chars_no_linebreak(),
                    )
                } else {
                    // already at top left
                    self.start
                }
            } else {
                TextPoint::new(cursor_pos.line, cursor_pos.col - 1)
            }
        } else {
            // just move cursor to start of selection
            let start = self.ordered().start;
            TextPoint::new(start.line, start.col)
        }
    }

    fn cursor_right(&self, source: &Rope, expanding: bool) -> TextPoint {
        if self.is_cursor() || expanding {
            // actually move if cursor
            let cursor_pos = self.end;
            let curr_line_len = source.line(cursor_pos.line).len_chars_no_linebreak();
            if cursor_pos.col == curr_line_len {
                // if at end of current line, go to next line
                let last_line = source.len_lines() - 1;
                if cursor_pos.line != last_line {
                    TextPoint::new(cursor_pos.line + 1, 0)
                } else {
                    // already at end
                    self.start
                }
            } else {
                TextPoint::new(cursor_pos.line, cursor_pos.col + 1)
            }
        } else {
            // just move cursor to end of selection
            let end = self.ordered().end;
            TextPoint::new(end.line, end.col)
        }
    }

    fn cursor_to_prev_word_start(&self, source: &Rope) -> TextPoint {
        let mut cursor_pos = self.end;

        // move to line above if at start of line
        let at_start = cursor_pos.col == 0;
        if at_start {
            if cursor_pos.line == 0 {
                return TextPoint::ZERO;
            } else {
                cursor_pos.line -= 1;
            }
        }
        let line = source.line(cursor_pos.line);
        if at_start {
            cursor_pos.col = line.len_chars_no_linebreak();
        }

        let mut chars = line.chars_at(cursor_pos.col);

        // find end of word (if not already in one)
        while let Some(c) = chars.prev() {
            // always shift bc we are consuming chars here
            cursor_pos.col -= 1;

            // break if we hit a word
            if c.is_alphanumeric() {
                break;
            }
        }

        // find start of word
        while let Some(c) = chars.prev() {
            if c.is_alphanumeric() {
                cursor_pos.col -= 1;
            } else {
                break;
            }
        }

        cursor_pos
    }

    fn cursor_to_next_word_end(&self, source: &Rope) -> TextPoint {
        // TODO: handle special characters more like vscode

        let mut cursor_pos = self.end;
        let line = source.line(cursor_pos.line);
        let mut chars = line.chars_at(cursor_pos.col);

        // find start of word (if not already in one)
        while let Some(c) = chars.next() {
            if c == '\n' || c == '\r' {
                // move to the next line if we hit it
                cursor_pos.line += 1;
                cursor_pos.col = 0;

                let line = source.line(cursor_pos.line);
                chars = line.chars_at(cursor_pos.col);
            } else {
                // always shift bc we are consuming chars here
                cursor_pos.col += 1;
            }

            // break if we hit a word
            if c.is_alphanumeric() {
                break;
            }
        }

        // find end of word
        for c in chars {
            if c.is_alphanumeric() {
                cursor_pos.col += 1;
            } else {
                break;
            }
        }

        cursor_pos
    }

    fn cursor_to_line_start(&self, source: &Rope) -> TextPoint {
        let cursor_pos = self.end;
        let indent = source.line(cursor_pos.line).whitespace_at_start();

        if cursor_pos.col > indent {
            // move to indented start
            TextPoint::new(cursor_pos.line, indent)
        } else {
            // already at indented start, so move to true start
            TextPoint::new(cursor_pos.line, 0)
        }
    }

    fn cursor_to_line_end(&self, source: &Rope) -> TextPoint {
        let cursor_pos = self.end;
        TextPoint::new(
            cursor_pos.line,
            source.line(cursor_pos.line).len_chars_no_linebreak(),
        )
    }

    fn cursor_to_doc_end(&self, source: &Rope) -> TextPoint {
        let last_line = source.len_lines() - 1;
        let last_line_len = source.line(last_line).len_chars_no_linebreak();
        TextPoint::new(last_line, last_line_len)
    }
}

fn clamp_col(line: usize, col: usize, source: &Rope) -> usize {
    std::cmp::min(col, source.line(line).len_chars_no_linebreak())
}
