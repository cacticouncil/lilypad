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
        let old_range = range.ordered();
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
            if line_count == 1 {
                old_range.start.col
            } else {
                0
            } + last_line_len,
            old_range.start.row + (line_count - 1),
        );

        // update tree
        let edits = InputEdit {
            start_byte: byte_range.start,
            old_end_byte: byte_range.end,
            new_end_byte: byte_range.start + new.len(),
            start_position: old_range.start.into(),
            old_end_position: old_range.end.into(),
            new_end_position: new_end.into(),
        };
        self.tree_manager.update(source, edits);

        // show cursor whenever text changes
        self.cursor_visible = true;

        // set selection if should
        if change_selection {
            self.selection = TextRange::new(new_end, new_end);
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

    pub fn apply_vscode_edit(&mut self, source: &mut Rope, edit: &TextEdit) {
        self.replace_range(source, &edit.text, edit.range, true, false);
    }

    pub fn insert_str(&mut self, source: &mut Rope, add: &str) {
        let old_selection = self.selection.ordered();

        // move cursor
        self.selection = TextRange::new_cursor(
            old_selection.start.col + add.chars().count(),
            old_selection.start.row,
        );

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
        // find previous indent level and set new line to that many spaces
        let old_selection = self.selection.ordered();

        // find the indent level of the next line
        // (same as current line & increase if current line ends in colon)
        let curr_line = source.get_line(old_selection.start.row).unwrap();
        let indent_inc = if curr_line.ends_with(':') { 4 } else { 0 };
        let next_indent = curr_line.whitespace_at_start() + indent_inc;

        // update source
        let indent: &str = &" ".repeat(next_indent);
        let linebreak = &(source.detect_linebreak().to_owned() + indent);

        self.replace_range(source, linebreak, old_selection, true, true);
    }

    pub fn backspace(&mut self, source: &mut Rope) {
        let old_selection = self.selection.ordered();

        let delete_selection = if let Some(pseudo_selection) = self.pseudo_selection {
            // reset pair stacks because this could be deleting what they cover
            self.input_ignore_stack.clear();
            self.paired_delete_stack.clear();

            pseudo_selection
        } else if old_selection.is_cursor() {
            // if a cursor, delete the preceding character
            if old_selection.start.col == 0 {
                // abort if in position (0,0)
                if old_selection.start.row == 0 {
                    return;
                }

                // delete up to the end of the above line
                let above = old_selection.start.row - 1;
                TextRange::new(
                    TextPoint::new(source.len_char_for_line(above), above),
                    old_selection.start,
                )
            } else {
                // de-indent if at start of line
                let line_indent = source.line(old_selection.start.row).whitespace_at_start();
                let at_start = old_selection.start.col == line_indent;
                let before_delete_amount = if at_start { 4 } else { 1 };

                let paired = self.paired_delete_stack.pop().unwrap_or(false);
                let after_delete_amount = if paired {
                    // pop from ignore stack because we're going to delete the relevant character
                    self.input_ignore_stack.pop();
                    1
                } else {
                    0
                };

                TextRange::new(
                    TextPoint::new(
                        old_selection.start.col - before_delete_amount,
                        old_selection.start.row,
                    ),
                    TextPoint::new(
                        old_selection.start.col + after_delete_amount,
                        old_selection.start.row,
                    ),
                )
            }
        } else {
            // if a selection, delete the whole selection
            old_selection
        };

        self.replace_range(source, "", delete_selection, true, true);
    }
}
