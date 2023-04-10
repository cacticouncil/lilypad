use tree_sitter_c2rust::InputEdit;

use super::{line_count, line_len, os_linebreak, text_range::TextEdit, BlockEditor, TextRange};
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
        // TODO: maintain indent level
        let old_selection = self.selection.ordered();

        // update source
        let offsets = old_selection.ordered().offset_in(source);
        source.replace_range(offsets.clone(), os_linebreak());

        // move cursor
        self.selection = TextRange::new_cursor(0, old_selection.start.y + 1);

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

        // will need to redraw because of edits
        self.text_changed = true;
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
                self.selection = TextRange::new_cursor(line_len(above, source), above);

                // update vscode
                vscode::edited(
                    "",
                    old_selection.start.y - 1,
                    line_len(above, source),
                    old_selection.start.y,
                    old_selection.start.x,
                )
            } else {
                // just move back one char
                self.selection =
                    TextRange::new_cursor(old_selection.start.x - 1, old_selection.start.y);

                // update vscode
                vscode::edited(
                    "",
                    old_selection.start.y,
                    old_selection.start.x - 1,
                    old_selection.start.y,
                    old_selection.start.x,
                )
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
        }
        // for selection, delete text inside
        else {
            // set cursor to start of selection
            self.selection = TextRange::new_cursor(old_selection.start.x, old_selection.start.y);

            // remove everything in range
            let offsets = old_selection.offset_in(source);
            source.replace_range(offsets.clone(), "");

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

        // will need to redraw because of edits
        self.text_changed = true;
    }

    fn send_vscode_edit(text: &str, range: TextRange) {
        vscode::edited(text, range.start.y, range.start.x, range.end.y, range.end.x)
    }
}
