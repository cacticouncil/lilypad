use tree_sitter_c2rust::InputEdit;

use super::{
    detect_linebreak, line_count, line_len,
    text_range::{IntPoint, TextEdit},
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
        self.selection = TextRange::new_cursor(
            if line_count == 0 {
                old_selection.start.x
            } else {
                0
            } + last_line_len,
            old_selection.start.y + (line_count - 1),
        );

        // update tree
        let edits = InputEdit {
            start_byte: offsets.start,
            old_end_byte: offsets.end,
            new_end_byte: offsets.start + edit.text.len(),
            start_position: old_selection.start.as_tree_sitter(),
            old_end_position: old_selection.end.as_tree_sitter(),
            new_end_position: self.selection.end.as_tree_sitter(),
        };
        self.tree_manager.borrow_mut().update(source, edits);

        // will need to redraw because of edits
        self.text_changed = true;
    }

    pub fn insert_str(&mut self, source: &mut String, add: &str) {
        // update source
        let old_selection = self.selection.ordered();
        let offsets = old_selection.offset_in(source);
        source.replace_range(offsets.clone(), add);

        // move cursor
        self.selection = TextRange::new_cursor(
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

        // will need to redraw because of edits
        self.text_changed = true;
    }

    pub fn insert_newline(&mut self, source: &mut String) {
        // find previous indent level and set new line to that many spaces
        let old_selection = self.selection.ordered();

        // find the indent level of the next line
        // (same as current line & increase if current line ends in colon)
        let curr_line = source.lines().nth(old_selection.start.y).unwrap_or("");
        let indent_inc = if curr_line.ends_with(':') { 4 } else { 0 };
        let next_indent = whitespace_at_start(curr_line) + indent_inc;

        // update source
        let indent: &str = &" ".repeat(next_indent);
        let linebreak = &(detect_linebreak(source).to_owned() + indent);
        let offsets = old_selection.ordered().offset_in(source);
        source.replace_range(offsets.clone(), linebreak);

        // move cursor
        self.selection = TextRange::new_cursor(next_indent, old_selection.start.y + 1);

        // update tree
        let edits = InputEdit {
            start_byte: offsets.start,
            old_end_byte: offsets.end,
            new_end_byte: offsets.start + linebreak.len(),
            start_position: old_selection.start.as_tree_sitter(),
            old_end_position: old_selection.end.as_tree_sitter(),
            new_end_position: self.selection.end.as_tree_sitter(),
        };
        self.tree_manager.borrow_mut().update(source, edits);

        // update vscode
        Self::send_vscode_edit(linebreak, old_selection);

        // will need to redraw because of edits
        self.text_changed = true;
    }

    pub fn backspace(&mut self, source: &mut String) {
        let old_selection = self.selection.ordered();

        let delete_selection = if old_selection.is_cursor() {
            // if a cursor, delete the preceding character
            if old_selection.start.x == 0 {
                // abort if in position (0,0)
                if old_selection.start.y == 0 {
                    return;
                }

                // delete up to the end of the above line
                let above = old_selection.start.y - 1;
                TextRange::new(
                    IntPoint::new(line_len(above, source), above),
                    old_selection.start,
                )
            } else {
                let mut delete_amount = 1;

                // de-indent if at start of line
                let line_indent = indent_of_line(source, old_selection.start.y);
                if old_selection.start.x == line_indent {
                    delete_amount = 4;
                }

                TextRange::new(
                    IntPoint::new(old_selection.start.x - delete_amount, old_selection.start.y),
                    old_selection.start,
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
            start_position: delete_selection.start.as_tree_sitter(),
            old_end_position: delete_selection.end.as_tree_sitter(),
            new_end_position: delete_selection.start.as_tree_sitter(),
        };
        self.tree_manager.borrow_mut().update(source, edits);

        // update vscode
        Self::send_vscode_edit("", delete_selection);

        // will need to redraw because of edits
        self.text_changed = true;
    }

    fn send_vscode_edit(text: &str, range: TextRange) {
        vscode::edited(text, range.start.y, range.start.x, range.end.y, range.end.x)
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
