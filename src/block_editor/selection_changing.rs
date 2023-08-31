use druid::{text::Movement, EventCtx, MouseEvent};
use ropey::Rope;
use tree_sitter_c2rust::{Point, TreeCursor};

use super::{
    rope_ext::{RopeExt, RopeSliceExt},
    BlockEditor, TextPoint, TextRange, FONT_HEIGHT, FONT_WIDTH,
};

impl BlockEditor {
    fn set_selection(&mut self, selection: TextRange, source: &Rope) {
        self.selection = selection;
        self.find_pseudo_selection(source);

        // make cursor visible whenever moved
        self.cursor_visible = true;

        // clear input ignore stack
        self.input_ignore_stack.clear();
        self.paired_delete_stack.clear();
    }

    pub fn find_pseudo_selection(&mut self, source: &Rope) {
        self.pseudo_selection = None;
        if self.selection.is_cursor() {
            // find if the cursor is after a quote
            let cursor_loc = self.selection.start;
            let cursor_offset = cursor_loc.char_idx_in(source);
            let (prev_char, _) = source.surrounding_chars(cursor_offset);

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
        point: Point,
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
    pub fn move_cursor(&mut self, movement: Movement, source: &Rope) {
        let new_cursor = self.find_movement_result(movement, source, false);
        self.set_selection(TextRange::new_cursor(new_cursor), source);
    }

    pub fn move_selecting(&mut self, movement: Movement, source: &Rope) {
        let new_sel = self.expand_selection_by(movement, source);
        self.set_selection(new_sel, source);
    }

    pub fn expand_selection_by(&self, movement: Movement, source: &Rope) -> TextRange {
        let new_cursor = self.find_movement_result(movement, source, true);
        TextRange::new(self.selection.start, new_cursor)
    }

    pub fn find_movement_result(
        &self,
        movement: Movement,
        source: &Rope,
        expanding: bool,
    ) -> TextPoint {
        use druid::text::{Direction::*, Movement::*, VerticalMovement::*};
        match movement {
            Grapheme(dir) => match dir {
                Left | Upstream => self.cursor_left(source, expanding),
                Right | Downstream => self.cursor_right(source, expanding),
            },
            Word(dir) => match dir {
                Left | Upstream => self.cursor_to_prev_word_start(source),
                Right | Downstream => self.cursor_to_next_word_end(source),
            },
            Line(dir) => match dir {
                Left | Upstream => self.cursor_to_line_start(source),
                Right | Downstream => self.cursor_to_line_end(source),
            },
            Vertical(dir) => match dir {
                LineUp => self.cursor_up(source),
                LineDown => self.cursor_down(source),
                DocumentStart => TextPoint::ZERO,
                DocumentEnd => self.cursor_to_doc_end(source),
                _ => {
                    println!("unimplemented vertical move: {:?}", dir);
                    self.selection.start
                }
            },
            _ => {
                println!("unimplemented movement: {:?}", movement);
                self.selection.start
            }
        }
    }

    fn cursor_up(&self, source: &Rope) -> TextPoint {
        // when moving up, use top of selection
        let cursor_pos = self.selection.ordered().start;

        if cursor_pos.row == 0 {
            TextPoint::ZERO
        } else {
            // TODO: the normal text editor experience has a "memory" of how far right
            // the cursor started during a chain for arrow up/down (and then it snaps back there).
            // if that memory is implemented, it can replace self.cursor_pos.x
            TextPoint::new(
                clamp_col(cursor_pos.row - 1, cursor_pos.col, source),
                cursor_pos.row - 1,
            )
        }
    }

    fn cursor_down(&self, source: &Rope) -> TextPoint {
        // when moving down use bottom of selection
        let cursor_pos = self.selection.ordered().end;

        let last_line = source.lines().count() - 1;
        let next_line = std::cmp::min(cursor_pos.row + 1, last_line);

        if cursor_pos.row == last_line {
            // if on last line, just move to end of line
            TextPoint::new(
                source
                    .get_line(source.len_lines() - 1)
                    .map_or(0, |line| line.len_chars()),
                last_line,
            )
        } else {
            // same memory thing as above applies here
            TextPoint::new(clamp_col(next_line, cursor_pos.col, source), next_line)
        }
    }

    fn cursor_left(&self, source: &Rope, expanding: bool) -> TextPoint {
        if self.selection.is_cursor() || expanding {
            // actually move if cursor
            let cursor_pos = self.selection.end;
            if cursor_pos.col == 0 {
                // if at start of line, move to end of line above
                if cursor_pos.row != 0 {
                    TextPoint::new(
                        source.line(cursor_pos.row - 1).len_chars_no_linebreak(),
                        cursor_pos.row - 1,
                    )
                } else {
                    // already at top left
                    self.selection.start
                }
            } else {
                TextPoint::new(cursor_pos.col - 1, cursor_pos.row)
            }
        } else {
            // just move cursor to start of selection
            let start = self.selection.ordered().start;
            TextPoint::new(start.col, start.row)
        }
    }

    fn cursor_right(&self, source: &Rope, expanding: bool) -> TextPoint {
        if self.selection.is_cursor() || expanding {
            // actually move if cursor
            let cursor_pos = self.selection.end;
            let curr_line_len = source.line(cursor_pos.row).len_chars_no_linebreak();
            if cursor_pos.col == curr_line_len {
                // if at end of current line, go to next line
                let last_line = source.len_lines() - 1;
                if cursor_pos.row != last_line {
                    TextPoint::new(0, cursor_pos.row + 1)
                } else {
                    // already at end
                    self.selection.start
                }
            } else {
                TextPoint::new(cursor_pos.col + 1, cursor_pos.row)
            }
        } else {
            // just move cursor to end of selection
            let end = self.selection.ordered().end;
            TextPoint::new(end.col, end.row)
        }
    }

    fn cursor_to_prev_word_start(&self, source: &Rope) -> TextPoint {
        let mut cursor_pos = self.selection.end;

        // move to line above if at start of line
        let at_start = cursor_pos.col == 0;
        if at_start {
            cursor_pos.row -= 1;
        }
        let line = source.line(cursor_pos.row);
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

        let mut cursor_pos = self.selection.end;
        let line = source.line(cursor_pos.row);
        let mut chars = line.chars_at(cursor_pos.col);

        // find start of word (if not already in one)
        while let Some(c) = chars.next() {
            if c == '\n' || c == '\r' {
                // move to the next line if we hit it
                cursor_pos.row += 1;
                cursor_pos.col = 0;

                let line = source.line(cursor_pos.row);
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
        let cursor_pos = self.selection.end;
        let indent = source.line(cursor_pos.row).whitespace_at_start();

        if cursor_pos.col > indent {
            // move to indented start
            TextPoint::new(indent, cursor_pos.row)
        } else {
            // already at indented start, so move to true start
            TextPoint::new(0, cursor_pos.row)
        }
    }

    fn cursor_to_line_end(&self, source: &Rope) -> TextPoint {
        let cursor_pos = self.selection.end;
        TextPoint::new(
            source.line(cursor_pos.row).len_chars_no_linebreak(),
            cursor_pos.row,
        )
    }

    fn cursor_to_doc_end(&self, source: &Rope) -> TextPoint {
        let last_line = source.len_lines() - 1;
        let last_line_len = source.line(last_line).len_chars_no_linebreak();
        TextPoint::new(last_line_len, last_line)
    }

    /* ------------------------------ Mouse Clicks ------------------------------ */
    pub fn mouse_clicked(&mut self, mouse: &MouseEvent, source: &Rope, ctx: &mut EventCtx) {
        // move the cursor and get selection start position
        let loc = self.mouse_to_coord(mouse, source);
        let selection = TextRange::new_cursor(loc);
        self.set_selection(selection, source);

        // request keyboard focus if not already focused
        if !ctx.is_focused() {
            ctx.request_focus();
        }
    }

    pub fn mouse_dragged(&mut self, mouse: &MouseEvent, source: &Rope, _ctx: &mut EventCtx) {
        // set selection end position to dragged position
        self.selection.end = self.mouse_to_coord(mouse, source);

        // clear pseudo selection if making a selection
        if !self.selection.is_cursor() {
            self.pseudo_selection = None;
        }

        // show cursor
        self.cursor_visible = true;
    }

    pub fn mouse_to_coord(&self, mouse: &MouseEvent, source: &Rope) -> TextPoint {
        // find the line clicked on by finding the next one and then going back one
        let mut y: usize = 0;
        let mut total_pad = 0.0;
        for row_pad in &self.padding {
            total_pad += row_pad;
            let curr_line_start = total_pad + (y as f64 * FONT_HEIGHT.get().unwrap());
            let raw_y = mouse.pos.y - super::OUTER_PAD;
            if raw_y <= curr_line_start {
                break;
            }
            y += 1;
        }
        y = y.saturating_sub(1);

        // double check that we are in bounds
        // (clicking and deleting at the same time can cause the padding to not be updated yet)
        let line_count = source.len_lines();
        if y >= line_count {
            y = line_count - 1;
        }

        // TODO: if past last line, move to end of last line

        let x_raw = ((mouse.pos.x - super::OUTER_PAD - super::GUTTER_WIDTH - super::TEXT_L_PAD)
            / FONT_WIDTH.get().unwrap())
        .round() as usize;
        let x_bound = clamp_col(y, x_raw, source);

        TextPoint::new(x_bound, y)
    }

    /// Finds the text coordinate that the mouse is over, without clamping to a valid position within the text
    pub fn mouse_to_raw_coord(&self, point: druid::Point) -> TextPoint {
        let font_height = *FONT_HEIGHT.get().unwrap();
        let font_width = *FONT_WIDTH.get().unwrap();

        // find the line clicked on by finding the next one and then going back one
        let mut y: usize = 0;
        let mut total_pad = 0.0;
        for row_pad in &self.padding {
            total_pad += row_pad;
            let curr_line_start = total_pad + (y as f64 * font_height);
            let raw_y = point.y - super::OUTER_PAD;
            if raw_y <= curr_line_start {
                break;
            }
            y += 1;
        }

        // add any remaining lines past the last line
        y += ((point.y - (total_pad + (y as f64 * font_height))) / font_height) as usize;

        y = y.saturating_sub(1);

        let x = ((point.x - super::OUTER_PAD - super::GUTTER_WIDTH - super::TEXT_L_PAD)
            / font_width)
            .round() as usize;

        TextPoint::new(x, y)
    }
}

fn clamp_col(row: usize, col: usize, source: &Rope) -> usize {
    std::cmp::min(col, source.line(row).len_chars_no_linebreak())
}
