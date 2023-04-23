use tree_sitter_c2rust::InputEdit;

use super::{
    text_range::{TextEdit, TextPoint},
    text_util::{detect_linebreak, line_count, line_len, surrounding_chars},
    BlockEditor, TextRange,
};
use crate::vscode;

impl BlockEditor {
    pub fn apply_edit(&mut self, source: &mut String, edit: &TextEdit) {
        // update source
        let old_selection = edit.range.ordered();
        let offsets = old_selection.offset_in(source);
        source.replace_range(offsets.clone(), &edit.text);

        // handle newlines inside inserted text
        let line_count = line_count(&edit.text);
        let last_line_len = edit.text.lines().last().unwrap_or("").chars().count();

        // move cursor
        // TODO: fix vscode undo/redo positioning
        self.selection = TextRange::new_cursor(
            if line_count == 0 {
                old_selection.start.col
            } else {
                0
            } + last_line_len,
            old_selection.start.row + (line_count - 1),
        );

        // update tree
        let edits = InputEdit {
            start_byte: offsets.start,
            old_end_byte: offsets.end,
            new_end_byte: offsets.start + edit.text.len(),
            start_position: old_selection.start.into(),
            old_end_position: old_selection.end.into(),
            new_end_position: self.selection.end.into(),
        };
        self.tree_manager.update(source, edits);

        // will need to redraw because of edits
        self.text_changed = true;

        self.input_ignore_stack.clear();
        self.paired_delete_stack.clear();

        self.find_pseudo_selection(source);
    }

    pub fn insert_str(&mut self, source: &mut String, add: &str) {
        // update source
        let old_selection = self.selection.ordered();
        let offsets = old_selection.offset_in(source);

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
            let (prev_char, next_char) = surrounding_chars(offsets.start, source);

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

        source.replace_range(offsets.clone(), actual_add);

        // update tree
        let edits = InputEdit {
            start_byte: offsets.start,
            old_end_byte: offsets.end,
            new_end_byte: offsets.start + actual_add.len(),
            start_position: old_selection.start.into(),
            old_end_position: old_selection.end.into(),
            new_end_position: tree_sitter_c2rust::Point::new(
                old_selection.start.row,
                old_selection.start.col + actual_add.chars().count(),
            ),
        };
        self.tree_manager.update(source, edits);

        // update vscode
        Self::send_vscode_edit(actual_add, old_selection);

        // will need to redraw because of edits
        self.text_changed = true;

        self.find_pseudo_selection(source);
    }

    pub fn insert_newline(&mut self, source: &mut String) {
        // find previous indent level and set new line to that many spaces
        let old_selection = self.selection.ordered();

        // find the indent level of the next line
        // (same as current line & increase if current line ends in colon)
        let curr_line = source.lines().nth(old_selection.start.row).unwrap_or("");
        let indent_inc = if curr_line.ends_with(':') { 4 } else { 0 };
        let next_indent = whitespace_at_start(curr_line) + indent_inc;

        // update source
        let indent: &str = &" ".repeat(next_indent);
        let linebreak = &(detect_linebreak(source).to_owned() + indent);
        let offsets = old_selection.ordered().offset_in(source);
        source.replace_range(offsets.clone(), linebreak);

        // move cursor
        self.selection = TextRange::new_cursor(next_indent, old_selection.start.row + 1);

        // update tree
        let edits = InputEdit {
            start_byte: offsets.start,
            old_end_byte: offsets.end,
            new_end_byte: offsets.start + linebreak.len(),
            start_position: old_selection.start.into(),
            old_end_position: old_selection.end.into(),
            new_end_position: self.selection.end.into(),
        };
        self.tree_manager.update(source, edits);

        // update vscode
        Self::send_vscode_edit(linebreak, old_selection);

        // will need to redraw because of edits
        self.text_changed = true;

        self.find_pseudo_selection(source);
    }

    pub fn backspace(&mut self, source: &mut String) {
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
                    TextPoint::new(line_len(above, source), above),
                    old_selection.start,
                )
            } else {
                // de-indent if at start of line
                let line_indent = indent_of_line(source, old_selection.start.row);
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

        // set cursor to start of what is being deleted
        self.selection = TextRange::new(delete_selection.start, delete_selection.start);

        // remove everything in range
        let delete_offsets = delete_selection.offset_in(source);
        source.replace_range(delete_offsets.clone(), "");

        // update tree
        let edits = InputEdit {
            start_byte: delete_offsets.start,
            old_end_byte: delete_offsets.end,
            new_end_byte: delete_offsets.start,
            start_position: delete_selection.start.into(),
            old_end_position: delete_selection.end.into(),
            new_end_position: delete_selection.start.into(),
        };
        self.tree_manager.update(source, edits);

        // update vscode
        Self::send_vscode_edit("", delete_selection);

        // will need to redraw because of edits
        self.text_changed = true;

        self.find_pseudo_selection(source);
    }

    fn send_vscode_edit(text: &str, range: TextRange) {
        vscode::edited(
            text,
            range.start.row,
            range.start.col,
            range.end.row,
            range.end.col,
        )
    }
}

fn indent_of_line(source: &str, line: usize) -> usize {
    whitespace_at_start(source.lines().nth(line).unwrap_or(""))
}

fn whitespace_at_start(input: &str) -> usize {
    input
        .chars()
        .take_while(|ch| ch.is_whitespace() && *ch != '\n')
        .count()
}
