use druid::text::{Direction, Movement};
use ropey::Rope;
use tree_sitter_c2rust::InputEdit;

use super::{
    rope_ext::{RopeExt, RopeSliceExt},
    text_range::{TextEdit, TextPoint},
    BlockEditor, TextRange,
};
use crate::vscode;

impl BlockEditor {
    fn replace_range(
        &mut self,
        source: &mut Rope,
        new: &str,
        range: TextRange,
        change_selection: bool,
        notify_vscode: bool,
    ) {
        // get edit ranges
        let range = range.ordered();
        let char_range = range.char_range_in(source);
        let byte_range = range.byte_range_in(source);

        // edit string
        source.remove(char_range.clone());
        source.insert(char_range.start, new);

        // find new ending (account for newlines present)
        let line_count = std::cmp::max(
            new.lines().count() + if new.ends_with('\n') { 1 } else { 0 },
            1,
        );
        let last_line_len = new.lines().last().unwrap_or("").chars().count();
        let new_end = TextPoint::new(
            if line_count == 1 { range.start.col } else { 0 } + last_line_len,
            range.start.row + (line_count - 1),
        );

        // update tree
        let edits = InputEdit {
            start_byte: byte_range.start,
            old_end_byte: byte_range.end,
            new_end_byte: byte_range.start + new.len(),
            start_position: range.start.into(),
            old_end_position: range.end.into(),
            new_end_position: new_end.into(),
        };
        self.tree_manager.update(source, edits);

        // show cursor whenever text changes
        self.cursor_visible = true;

        // set selection if should
        if change_selection {
            self.selection = TextRange::new_cursor(new_end);
        }

        // notify vscode if should
        // (conditional to prevent infinite loops)
        if notify_vscode {
            // TODO: this is a pretty major bottleneck in the extension
            vscode::edited(
                new,
                range.start.row,
                range.start.col,
                range.end.row,
                range.end.col,
            )
        }

        // will need to redraw because of edits
        self.text_changed = true;

        // cursor will have moved, so check for new pseudo selections
        self.find_pseudo_selection(source);
    }

    /// Apply an edit that originated from vscode (so does not notify vscode of the edit)
    pub fn apply_vscode_edit(&mut self, source: &mut Rope, edit: &TextEdit) {
        self.replace_range(source, &edit.text, edit.range, true, false);
    }

    pub fn apply_edit(&mut self, source: &mut Rope, edit: &TextEdit) {
        self.replace_range(source, &edit.text, edit.range, true, true)
    }

    pub fn insert_str(&mut self, source: &mut Rope, add: &str) {
        let old_selection = self.selection.ordered();

        // move cursor
        self.selection = TextRange::new_cursor(TextPoint::new(
            old_selection.start.col + add.chars().count(),
            old_selection.start.row,
        ));

        // don't insert if previously automatically inserted
        // this is cleared whenever the cursor is manually moved
        if Some(add) == self.input_ignore_stack.last().copied() {
            self.input_ignore_stack.pop();
            self.paired_delete_stack.clear();

            return;
        }

        // (what is added, full insertion, string)
        let pair_completion = match add {
            "'" => Some(("'", "''", true)),
            "\"" => Some(("\"", "\"\"", true)),
            "(" => Some((")", "()", false)),
            "[" => Some(("]", "[]", false)),
            "{" => Some(("}", "{}", false)),
            _ => None,
        };

        let actual_add = if let Some((additional, full_add, for_string)) = pair_completion {
            // only insert if the previous and next characters meet the conditions
            // (different conditions for string or not)
            let start_char = old_selection.start.char_idx_in(source);
            let (prev_char, next_char) = source.surrounding_chars(start_char);

            let should_insert = if for_string {
                let add_char = add.chars().next().unwrap();
                // if there is a character before, they probably want that to be inside (& allow f strings)
                !(prev_char.is_alphanumeric() && prev_char != 'f')
                    // if there is a character following, they probably want that to be inside
                    && !next_char.is_alphanumeric()
                    // multiline strings are 3 long, this prevents jumping to 4
                    && prev_char != add_char
                    && next_char != add_char
            } else {
                // only care about next character because parenthesis and brackets
                // can be attached to the character before
                !next_char.is_alphanumeric()
            };

            if should_insert {
                self.input_ignore_stack.push(additional);
                self.paired_delete_stack.push(true);
                full_add
            } else {
                if !self.paired_delete_stack.is_empty() {
                    self.paired_delete_stack.push(false);
                }
                add
            }
        } else {
            if !self.paired_delete_stack.is_empty() {
                self.paired_delete_stack.push(false);
            }
            add
        };

        self.replace_range(source, actual_add, old_selection, false, true);
    }

    pub fn insert_newline(&mut self, source: &mut Rope) {
        // find linebreak used in source
        let linebreak = source.detect_linebreak();

        // find previous indent level and set new line to that many spaces
        let old_selection = self.selection.ordered();
        let curr_line = source.line(old_selection.start.row);
        let prev_indent = curr_line.whitespace_at_start();

        // find the indent level of the next line
        // (same as current line & increase if character before cursor is a scope char)
        let indent_inc = if old_selection.start.col > 1 {
            let char_before_cursor = curr_line.char(old_selection.start.col - 1);
            if char_before_cursor == self.language.new_scope_char {
                4
            } else {
                0
            }
        } else {
            0
        };
        let next_indent = prev_indent + indent_inc;

        // update source
        let indent: &str = &" ".repeat(next_indent);
        let to_insert = format!("{}{}", linebreak, indent);

        self.replace_range(source, &to_insert, old_selection, true, true);
    }

    pub fn backspace(&mut self, source: &mut Rope, movement: Movement) {
        let old_selection = self.selection.ordered();

        let delete_selection = if let Some(pseudo_selection) = self.pseudo_selection {
            // reset pair stacks because this could be deleting what they cover
            self.input_ignore_stack.clear();
            self.paired_delete_stack.clear();

            pseudo_selection
        } else if old_selection.is_cursor() {
            // if single character not at start of line, backspace apply de-indent and paired delete
            if old_selection.start.col != 0
                && (movement == Movement::Grapheme(Direction::Upstream)
                    || movement == Movement::Grapheme(Direction::Left))
            {
                // unindent if at start of line
                let line_indent = source.line(old_selection.start.row).whitespace_at_start();
                let at_indent = old_selection.start.col == line_indent;
                if at_indent {
                    self.unindent(source);
                    return;
                }

                // see if there is a paired character to delete
                let paired = self.paired_delete_stack.pop().unwrap_or(false);
                let after_delete_amount = if paired {
                    // pop because we're going to delete the character to ignore
                    self.input_ignore_stack.pop();
                    1
                } else {
                    0
                };

                TextRange::new(
                    TextPoint::new(old_selection.start.col - 1, old_selection.start.row),
                    TextPoint::new(
                        old_selection.start.col + after_delete_amount,
                        old_selection.start.row,
                    ),
                )
            } else {
                self.expand_selection_by(movement, source)
            }
        } else {
            // if a selection, delete the whole selection (applying a movement if necessary)
            if let Movement::Grapheme(_) = movement {
                // if just a single character, delete the current selection
                old_selection
            } else {
                // if more, delete the selection and the movement
                self.expand_selection_by(movement, source)
            }
        };

        self.replace_range(source, "", delete_selection, true, true);
    }

    pub fn indent(&mut self, source: &mut Rope) {
        // apply to every line of selection
        let ordered = self.selection.ordered();
        let lines = ordered.start.row..=ordered.end.row;

        for line_num in lines {
            // get current indent of line
            let line = source.line(line_num);
            let curr_indent = line.whitespace_at_start();

            // make what to add to start of line
            let indent_amount = 4 - (curr_indent % 4);
            let indent = &" ".repeat(indent_amount);

            // add it
            let start_of_line = TextRange::new_cursor(TextPoint::new(0, line_num));
            self.replace_range(source, indent, start_of_line, false, true);
        }

        // adjust selection
        self.selection.start.col += 4;
        self.selection.end.col += 4;
    }

    pub fn unindent(&mut self, source: &mut Rope) {
        // apply to every line of selection
        let ordered = self.selection.ordered();
        let lines = ordered.start.row..=ordered.end.row;

        for line_num in lines {
            // get current indent of line
            let line = source.line(line_num);
            let curr_indent = line.whitespace_at_start();
            if curr_indent == 0 {
                continue;
            }

            // remove start of line
            let unindent_amount = 4 - (curr_indent % 4);
            let remove_range = TextRange::new(
                TextPoint::new(0, line_num),
                TextPoint::new(unindent_amount, line_num),
            );
            self.replace_range(source, "", remove_range, false, true);
        }

        // adjust selection
        self.selection.start.col = self.selection.start.col.saturating_sub(4);
        self.selection.end.col = self.selection.end.col.saturating_sub(4);
    }
}
