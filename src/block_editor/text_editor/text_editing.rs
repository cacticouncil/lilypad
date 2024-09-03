use ropey::Rope;
use std::borrow::Cow;
use tree_sitter::InputEdit;

use super::{
    undo_manager::UndoStopCondition::{self, *},
    TextEditor,
};
use crate::{
    block_editor::{
        rope_ext::{RopeExt, RopeSliceExt},
        text_range::TextPoint,
        TextRange,
    },
    lang::NewScopeChar,
    vscode,
};

const TAB_SIZE: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextMovement {
    Horizontal { unit: HUnit, direction: HDir },
    Vertical { unit: VUnit, direction: VDir },
}

impl TextMovement {
    pub fn horizontal(unit: HUnit, direction: HDir) -> Self {
        Self::Horizontal { unit, direction }
    }

    pub fn vertical(unit: VUnit, direction: VDir) -> Self {
        Self::Vertical { unit, direction }
    }

    pub fn is_grapheme(&self) -> bool {
        matches!(
            self,
            Self::Horizontal {
                unit: HUnit::Grapheme,
                ..
            }
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HUnit {
    /// A movement that stops when it reaches an extended grapheme cluster boundary.
    ///
    /// This movement is achieved on most systems by pressing the left and right
    /// arrow keys.  For more information on grapheme clusters, see
    /// [Unicode Text Segmentation](https://unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries).
    Grapheme,

    /// A movement that stops when it reaches a word boundary.
    ///
    /// This movement is achieved on most systems by pressing the left and right
    /// arrow keys while holding control. For more information on words, see
    /// [Unicode Text Segmentation](https://unicode.org/reports/tr29/#Word_Boundaries).
    Word,

    /// A movement that stops when it reaches a soft line break.
    ///
    /// This movement is achieved on macOS by pressing the left and right arrow
    /// keys while holding command.  `Line` should be idempotent: if the
    /// position is already at the end of a soft-wrapped line, this movement
    /// should never push it onto another soft-wrapped line.
    ///
    /// In order to implement this properly, your text positions should remember
    /// their affinity.
    Line,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HDir {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VUnit {
    Line,
    Document,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VDir {
    Up,
    Down,
}

#[derive(Debug)]
pub struct TextEdit<'a> {
    text: Cow<'a, str>,
    range: TextRange,
    new_end_point: TextPoint,
    origin: TextEditOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TextEditOrigin {
    Lilypad,
    Vscode,
}

impl<'a> TextEdit<'a> {
    pub fn new(text: Cow<'a, str>, range: TextRange) -> Self {
        let ordered = range.ordered();
        let new_end_point = Self::find_new_end_point(&text, ordered);
        Self {
            text,
            range: ordered,
            new_end_point,
            origin: TextEditOrigin::Lilypad,
        }
    }

    pub fn delete(range: TextRange) -> Self {
        Self {
            text: Cow::Borrowed(""),
            range: range.ordered(),
            new_end_point: range.start,
            origin: TextEditOrigin::Lilypad,
        }
    }

    /// Creates a new TextEdit that does not notify VSCode when it is applied
    #[allow(dead_code)]
    pub fn new_from_vscode(text: Cow<'a, str>, range: TextRange) -> Self {
        let ordered = range.ordered();
        let new_end_point = Self::find_new_end_point(&text, ordered);
        Self {
            text,
            range: ordered,
            new_end_point,
            origin: TextEditOrigin::Vscode,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn range(&self) -> TextRange {
        self.range
    }

    // cannot be an implementation of ToOwned because of existing blanked implementation:
    // https://stackoverflow.com/questions/72385586/implementing-toowned-a-static-for-an-enum-containing-cowa-str-causes
    pub fn owned_text(&self) -> TextEdit<'static> {
        TextEdit {
            text: Cow::Owned(self.text.to_string()),
            range: self.range,
            new_end_point: self.new_end_point,
            origin: self.origin,
        }
    }

    /// Apply the text edit on the rope and tree manager. Returns the inverse text edit.
    pub fn apply(
        &self,
        source: &mut Rope,
        tree_manager: &mut crate::parse::TreeManager,
    ) -> TextEdit {
        let char_range = self.range.char_range_in(source);
        let byte_range = self.range.byte_range_in(source);

        // update buffer
        let removed = source.get_slice(char_range.clone()).map(|x| x.to_string());
        source.remove(char_range.clone());
        source.insert(char_range.start, &self.text);

        // update tree
        let tree_edit = InputEdit {
            start_byte: byte_range.start,
            old_end_byte: byte_range.end,
            new_end_byte: byte_range.start + self.text.len(),
            start_position: self.range.start.into(),
            old_end_position: self.range.end.into(),
            new_end_position: self.new_end().into(),
        };
        tree_manager.update(source, tree_edit);

        // update vscode if not from vscode
        if self.origin != TextEditOrigin::Vscode {
            vscode::edited(
                &self.text,
                self.range.start.line,
                self.range.start.col,
                self.range.end.line,
                self.range.end.col,
            );
        }

        let affected_range = TextRange::new(self.range.start, self.new_end_point);
        if let Some(removed) = removed {
            TextEdit::new(Cow::Owned(removed), affected_range)
        } else {
            TextEdit::delete(affected_range)
        }
    }

    pub fn new_end(&self) -> TextPoint {
        self.new_end_point
    }

    #[cfg(test)]
    pub fn apply_to_rope(&self, source: &mut Rope) {
        let char_range = self.range.char_range_in(source);
        source.remove(char_range.clone());
        source.insert(char_range.start, &self.text);
    }

    /// The end of the last line that isn't just a newline
    fn find_new_end_point(new_text: &str, text_range: TextRange) -> TextPoint {
        // find new ending (account for newlines present)
        let ends_with_linebreak = new_text.ends_with('\n');
        let line_count = std::cmp::max(
            new_text.lines().count() + if ends_with_linebreak { 1 } else { 0 },
            1,
        );

        let last_line_len = if ends_with_linebreak {
            0
        } else {
            new_text.lines().last().unwrap_or("").chars().count()
        };

        let end_col = if line_count == 1 {
            text_range.start.col
        } else {
            0
        } + last_line_len;
        let end_line = text_range.start.line + line_count - 1;
        TextPoint::new(end_line, end_col)
    }
}

/// Find the edit for inserting a single character. Returns the edit and the new selection.
fn edit_for_insert_char<'a>(
    selection: TextRange,
    source: &Rope,
    add: &'a str,
    input_ignore_stack: &mut Vec<&'static str>,
    paired_delete_stack: &mut Vec<bool>,
) -> (Option<TextEdit<'a>>, TextRange) {
    let old_selection = selection.ordered();

    // move cursor
    let new_selection = TextRange::new_cursor(TextPoint::new(
        old_selection.start.line,
        old_selection.start.col + add.chars().count(),
    ));

    // don't insert if previously automatically inserted
    // this is cleared whenever the cursor is manually moved
    if Some(add) == input_ignore_stack.last().copied() {
        input_ignore_stack.pop();
        paired_delete_stack.clear();

        return (None, new_selection);
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

        let should_insert_pair = if for_string {
            let add_char = add.chars().next().unwrap();

            // if the user is typing a quote adjacent to an alphanumeric character (excluding f strings),
            // they are probably closing a string instead of creating a new one, so don't insert a pair
            !(next_char.is_alphanumeric() || prev_char.is_alphanumeric() && prev_char != 'f')
                // multiline strings are 3 long, this prevents jumping to 4
                && prev_char != add_char
                && next_char != add_char
        } else {
            // only care about next character because parenthesis and brackets
            // can be attached to the character before
            !next_char.is_alphanumeric()
        };

        if should_insert_pair {
            input_ignore_stack.push(additional);
            paired_delete_stack.push(true);
            full_add
        } else {
            if !paired_delete_stack.is_empty() {
                paired_delete_stack.push(false);
            }
            add
        }
    } else {
        if !paired_delete_stack.is_empty() {
            paired_delete_stack.push(false);
        }
        add
    };

    let edit = TextEdit::new(Cow::Borrowed(actual_add), old_selection);
    (Some(edit), new_selection)
}

fn edit_for_insert_newline<'a>(
    selection: TextRange,
    source: &Rope,
    new_scope_char: NewScopeChar,
) -> (TextEdit<'a>, TextRange) {
    // find linebreak used in source
    let linebreak = source.detect_linebreak();

    // find previous indent level and set new line to that many spaces
    let old_selection = selection.ordered();
    let curr_line = source.line(old_selection.start.line);
    let prev_indent = curr_line.whitespace_at_start();

    let middle_of_bracket = new_scope_char == NewScopeChar::Brace && {
        if old_selection.start.col > 1 {
            let char_before_cursor = curr_line.char(old_selection.start.col - 1);
            let char_after_cursor = curr_line.char(old_selection.start.col);
            char_before_cursor == '{' && char_after_cursor == '}'
        } else {
            false
        }
    };

    // find the indent level of the next line
    // (same as current line & increase if character before cursor is a scope char)
    let indent_inc = if old_selection.start.col > 1 {
        let char_before_cursor = curr_line.char(old_selection.start.col - 1);
        if char_before_cursor == new_scope_char.char() {
            TAB_SIZE
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

    if !middle_of_bracket {
        let edit = TextEdit::new(Cow::Owned(to_insert), old_selection);
        let new_selection = TextRange::new_cursor(edit.new_end());
        (edit, new_selection)
    } else {
        // if in the middle of a bracket, insert an extra linebreak and indent
        // but only move the cursor to the newline in the middle
        let following_indent = " ".repeat(prev_indent);
        let extra_to_insert = format!("{}{}{}", to_insert, linebreak, following_indent);

        let edit = TextEdit::new(Cow::Owned(extra_to_insert), old_selection);
        let new_selection =
            TextRange::new_cursor(TextPoint::new(old_selection.start.line + 1, next_indent));
        (edit, new_selection)
    }
}

fn edit_for_backspace<'a>(
    selection: TextRange,
    source: &Rope,
    movement: TextMovement,
    pseudo_selection: Option<TextRange>,
    input_ignore_stack: &mut Vec<&'static str>,
    paired_delete_stack: &mut Vec<bool>,
) -> (Option<TextEdit<'a>>, TextRange) {
    let old_selection = selection.ordered();

    let delete_selection = if let Some(pseudo_selection) = pseudo_selection {
        // reset pair stacks because this could be deleting what they cover
        input_ignore_stack.clear();
        paired_delete_stack.clear();

        pseudo_selection
    } else if old_selection.is_cursor() {
        // if single character not at start of line, backspace apply de-indent and paired delete
        if old_selection.start.col != 0
            && movement == TextMovement::horizontal(HUnit::Grapheme, HDir::Left)
        {
            // unindent if at start of line
            let line_indent = source.line(old_selection.start.line).whitespace_at_start();
            let at_indent = old_selection.start.col == line_indent;
            if at_indent {
                let (edit, new_selection) = edit_for_unindent(old_selection, source);
                return (Some(edit), new_selection);
            }

            // see if there is a paired character to delete
            let paired = paired_delete_stack.pop().unwrap_or(false);
            let after_delete_amount = if paired {
                // pop because we're going to delete the character to ignore
                input_ignore_stack.pop();
                1
            } else {
                0
            };

            TextRange::new(
                TextPoint::new(old_selection.start.line, old_selection.start.col - 1),
                TextPoint::new(
                    old_selection.start.line,
                    old_selection.start.col + after_delete_amount,
                ),
            )
        } else {
            old_selection.expanded_by(movement, source).ordered()
        }
    } else {
        // if a selection, delete the whole selection (applying a movement if necessary)
        if movement.is_grapheme() {
            // if just a single character, delete the current selection
            old_selection
        } else {
            // if more, delete the selection and the movement
            old_selection.expanded_by(movement, source).ordered()
        }
    };

    // if deleting nothing (start of file), don't return an edit to prevent adding to the undo stack
    if delete_selection.is_cursor() {
        return (None, old_selection);
    }

    let edit = TextEdit::delete(delete_selection);
    let new_selection = TextRange::new_cursor(edit.new_end());
    (Some(edit), new_selection)
}

fn edit_for_indent<'a>(selection: TextRange, source: &Rope) -> (TextEdit<'a>, TextRange) {
    let ordered = selection.ordered();

    // expand selection to include entire lines
    let full_selection = TextRange::new(
        TextPoint::new(ordered.start.line, 0),
        TextPoint::new(
            ordered.end.line,
            source.line(ordered.end.line).len_chars_no_linebreak(),
        ),
    );
    let mut new_text: Rope = source.slice(full_selection.char_range_in(source)).into();

    let mut new_selection = selection;

    // apply to every line of selection
    for line_num in 0..new_text.len_lines() {
        // get current indent of line
        let line = new_text.line(line_num);
        let curr_indent = line.whitespace_at_start();
        let line_len = line.len_chars_no_linebreak();

        // make what to add to start of line
        let indent_amount = TAB_SIZE - (curr_indent % TAB_SIZE);
        let indent = " ".repeat(indent_amount);

        // add it
        let start_of_line = TextPoint::new(line_num, 0);
        new_text.insert(start_of_line.char_idx_in(&new_text), &indent);

        // Adjust selection if first or last line if the cursor for that line is in the text.
        // If the line is entirely whitespace, move the cursor anyway.
        // This is more complicated than a single comparison because new_selection can be inverted.
        if full_selection.start.line + line_num == new_selection.start.line
            && (new_selection.start.col > curr_indent || line_len == curr_indent)
        {
            new_selection.start.col += indent_amount;
        }
        if full_selection.start.line + line_num == new_selection.end.line
            && (new_selection.end.col > curr_indent || line_len == curr_indent)
        {
            new_selection.end.col += indent_amount;
        }
    }

    (
        TextEdit::new(Cow::Owned(new_text.to_string()), full_selection),
        new_selection,
    )
}

fn edit_for_unindent<'a>(selection: TextRange, source: &Rope) -> (TextEdit<'a>, TextRange) {
    // apply to every line of selection
    let ordered = selection.ordered();

    // expand selection to include entire lines
    let full_selection = TextRange::new(
        TextPoint::new(ordered.start.line, 0),
        TextPoint::new(
            ordered.end.line,
            source.line(ordered.end.line).len_chars_no_linebreak(),
        ),
    );
    let mut new_text: Rope = source
        .slice(full_selection.start.char_idx_in(source)..full_selection.end.char_idx_in(source))
        .into();

    let mut new_selection = selection;

    for line_num in 0..new_text.len_lines() {
        // get current indent of line
        let line = new_text.line(line_num);
        let curr_indent = line.whitespace_at_start();
        if curr_indent == 0 {
            continue;
        }

        // remove start of line
        let unindent_amount = if curr_indent % TAB_SIZE == 0 {
            TAB_SIZE
        } else {
            curr_indent % TAB_SIZE
        };
        let remove_range = TextRange::new(
            TextPoint::new(line_num, 0),
            TextPoint::new(line_num, unindent_amount),
        );
        new_text.remove(remove_range.char_range_in(&new_text));

        // adjust selection if first or last line
        if full_selection.start.line + line_num == new_selection.start.line {
            new_selection.start.col = new_selection.start.col.saturating_sub(unindent_amount);
        }
        if full_selection.start.line + line_num == new_selection.end.line {
            new_selection.end.col = new_selection.end.col.saturating_sub(unindent_amount);
        }
    }

    (
        TextEdit::new(Cow::Owned(new_text.to_string()), full_selection),
        new_selection,
    )
}

impl TextEditor {
    /// Apply an edit to the source. Leaves selection unchanged.
    /// NOTE: requires selection to be updated before calling because it updates the pseudo selection based on that.
    fn apply_edit_helper(
        &mut self,
        edit: &TextEdit,
        undo_stop_before: UndoStopCondition,
        undo_stop_after: bool,
    ) {
        // update buffer and tree
        let undo = edit.apply(&mut self.source, &mut self.tree_manager);

        // update undo manager
        self.undo_manager
            .add_undo(&undo, edit, undo_stop_before, undo_stop_after);
        self.undo_manager.clear_redos();

        // show cursor whenever text changes
        self.last_selection_time = self.frame_start_time;

        // will need to redraw because of edits
        self.text_changed = true;

        // cursor will have moved, so check for new pseudo selections
        self.find_pseudo_selection();
    }

    /// Apply an edit to the source and selection.
    /// Notifies vscode depending on the origin property of the TextEdit.
    pub fn apply_edit(
        &mut self,
        edit: &TextEdit,
        undo_stop_before: UndoStopCondition,
        undo_stop_after: bool,
    ) {
        self.selection = TextRange::new_cursor(edit.new_end());
        self.apply_edit_helper(edit, undo_stop_before, undo_stop_after);
    }

    /// Handle inserting a string at the current selection
    pub fn insert_str(&mut self, add: &str) {
        let old_selection = self.selection.ordered();
        let edit = TextEdit::new(Cow::Borrowed(add), old_selection);
        self.selection = TextRange::new_cursor(edit.new_end());
        self.apply_edit_helper(&edit, Always, true);
    }

    /// Handle typing a single character
    /// (separate from `insert_str` because it also handles paired completion)
    pub fn insert_char(&mut self, add: &str) {
        let (edit, new_selection) = edit_for_insert_char(
            self.selection,
            &self.source,
            add,
            &mut self.input_ignore_stack,
            &mut self.paired_delete_stack,
        );

        self.selection = new_selection;

        if let Some(edit) = edit {
            self.apply_edit_helper(&edit, IfNotMerged, false);
        }
    }

    pub fn insert_newline(&mut self) {
        let (edit, new_selection) =
            edit_for_insert_newline(self.selection, &self.source, self.language.new_scope_char);
        self.selection = new_selection;
        self.apply_edit_helper(&edit, Always, false);
    }

    pub fn backspace(&mut self, movement: TextMovement) {
        let (edit, new_selection) = edit_for_backspace(
            self.selection,
            &self.source,
            movement,
            self.pseudo_selection,
            &mut self.input_ignore_stack,
            &mut self.paired_delete_stack,
        );
        self.selection = new_selection;
        if let Some(edit) = edit {
            self.apply_edit_helper(&edit, IfNotMerged, false);
        }
    }

    /// Delete just the currently selected text
    pub fn delete_selection(&mut self) {
        if !self.selection.is_cursor() {
            self.apply_edit(&TextEdit::delete(self.selection), IfNotMerged, false);
        }
    }

    pub fn indent(&mut self) {
        let (edit, new_selection) = edit_for_indent(self.selection, &self.source);
        self.selection = new_selection;
        self.apply_edit_helper(&edit, Always, true);
    }

    pub fn unindent(&mut self) {
        let (edit, new_selection) = edit_for_unindent(self.selection, &self.source);
        self.selection = new_selection;
        self.apply_edit_helper(&edit, Always, true);
    }

    pub fn undo(&mut self) {
        if let Some(new_selection) = self
            .undo_manager
            .apply_undo(&mut self.source, &mut self.tree_manager)
        {
            self.selection = new_selection;

            // same as apply_edit_helper
            self.last_selection_time = self.frame_start_time;
            self.text_changed = true;
            self.find_pseudo_selection();
        }
    }

    pub fn redo(&mut self) {
        if let Some(new_selection) = self
            .undo_manager
            .apply_redo(&mut self.source, &mut self.tree_manager)
        {
            self.selection = new_selection;

            // same as apply_edit_helper
            self.last_selection_time = self.frame_start_time;
            self.text_changed = true;
            self.find_pseudo_selection();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_insert() {
        // insert single line
        char_insert_test("print('hell→←')", "o", "print('hello→←')");

        // replace single line
        char_insert_test("print('wo→rld←')", "o", "print('woo→←')");

        // replace multi line
        char_insert_test(
            "print('hell→a world')\nprint('hello← world')",
            "o",
            "print('hello→← world')",
        );

        // Insert parenthesis into prepending strings
        char_insert_test(
            "print('hello, world'→←)",
            ")",
            "print('hello, world')→←)"
        );

        // Insert parenthesis into input ignore stack (is this wrong?)
        char_insert_test(
            "print(→←)",
            ")",
            "print()→←)"
        );

        // Insert brackets
        char_insert_test(
            "[→←]",
            "]",
            "[]→←]"
        );

        // Insert quote
        char_insert_test(
            "\"→←\"",
            "\"",
            "\"\"→←\""
        );

        // Insert unicode
        char_insert_test(
            "→←",
            "༄",
            "༄→←"
        );

        // Insert on new line
        char_insert_test(
            "aa\n→←bb",
            "a",
            "aa\na→←bb"
        );

        // Replace on new line
        char_insert_test(
            "aa\n→bb←",
            "a",
            "aa\na→←"
        )
    }

    #[test]
    fn test_indent() {
        // indent empty line
        indent_test("→←\n", "    →←\n");
        indent_test("    →←\n", "        →←\n");

        // indent single line normally
        indent_test("print('hello world→←')", "    print('hello world→←')");

        // indent single line with existing indent
        indent_test(
            "    print('hello world→←')",
            "        print('hello world→←')",
        );

        // indent unaligned single line
        indent_test("  print('hello world→←')", "    print('hello world→←')");

        // indent multiple lines
        indent_test(
            "→print('hello world')\nprint('hello world')←",
            "→    print('hello world')\n    print('hello world')←",
        );

        // when selection in text, it should travel with the line
        indent_test(
            "print('hello→ world')\nprint('hello← world')",
            "    print('hello→ world')\n    print('hello← world')",
        );

        // when selection is in the indent, it should not move
        indent_test(
            "  →  print('hello world')\n  ←  print('hello world')",
            "  →      print('hello world')\n  ←      print('hello world')",
        );

        // ...that is independent for the start and end.
        // and should preserve inverted selections
        indent_test(
            "  ←  print('hello world')\n    print('hello→ world')",
            "  ←      print('hello world')\n        print('hello→ world')",
        );

        // indent multiple lines with different existing indents
        indent_test(
            "→print('hello world')\n  print('hello world')\n    print('hello world')←",
            "→    print('hello world')\n    print('hello world')\n        print('hello world')←",
        );

        // indent unicode
        indent_test(
            "→༄༄༄ᜃᜃᜃ←",
            "→    ༄༄༄ᜃᜃᜃ←"
        );
    }

    #[test]
    fn test_unindent() {
        // unindent empty line
        unindent_test("→←", "→←");
        unindent_test("    →←", "→←");
        unindent_test("        →←", "    →←");

        // unindent single line normally
        unindent_test("    print('hello world→←')", "print('hello world→←')");

        // unindent single line with existing indent
        unindent_test(
            "        print('hello world→←')",
            "    print('hello world→←')",
        );

        // unindent unaligned single line
        unindent_test("   print('hello world→←')", "print('hello world→←')");

        // unindent multiple lines
        unindent_test(
            "→    print('hello world')\n    print('hello world')←",
            "→print('hello world')\nprint('hello world')←",
        );

        // when selection in text, it should travel with the line
        unindent_test(
            "    print('hello→ world')\n    print('hello← world')",
            "print('hello→ world')\nprint('hello← world')",
        );

        // when selection is in the indent, it should move too
        unindent_test(
            "  →      print('hello world')\n  ←      print('hello world')",
            "→    print('hello world')\n←    print('hello world')",
        );

        // it should preserve inverted selections
        unindent_test(
            "  ←      print('hello world')\n        print('hello→ world')",
            "←    print('hello world')\n    print('hello→ world')",
        );

        // unindent multiple lines with different existing indents
        unindent_test(
            "→    print('hello world')\n    print('hello world')\n        print('hello world')←",
            "→print('hello world')\nprint('hello world')\n    print('hello world')←",
        );

        // unindent unicode
        unindent_test(
            "→    ༄༄༄ᜃᜃᜃ←",
            "→༄༄༄ᜃᜃᜃ←"
        );
    }

    #[test]
    fn test_backspace() {
        // Paired delete test
        backspace_test("(→←)", "→←",
                       Movement::Grapheme(Direction::Upstream),
                       None, &mut vec![")"],
                       &mut vec![true]
        );

        // Select paired delete test
        backspace_test("→(←)", "→←)",
                       Movement::Grapheme(Direction::Upstream),
                       None, &mut vec![")"],
                       &mut vec![true]
        );

        // Paired delete a lot of them
        backspace_test("(((→←)))", "((→←))",
                       Movement::Grapheme(Direction::Upstream),
                       None, &mut vec![")",")",")"],
                       &mut vec![true, true],
        );

        // Select paired delete a lot of them
        backspace_test("→(((←)))", "→←)))",
                       Movement::Grapheme(Direction::Upstream),
                       None, &mut vec![")",")",")"],
                       &mut vec![true, true, true],
        );

        // Delete single character
        backspace_test("aaa→←", "aa→←",
                        Movement::Grapheme(Direction::Upstream),
                        None, &mut vec![], &mut vec![]
        );

        // Ctrl + Backspace
        backspace_test("aaa→←", "→←",
                       Movement::Line(Direction::Upstream),
                       None, &mut vec![], &mut vec![]
        );

        // Ctrl + Backspace paired delete
        backspace_test("(aaa→←)", "→←)",
                       Movement::Line(Direction::Upstream),
                       None, &mut vec![")"],
                       &mut vec![true, false, false, false]
        );

        // Ctrl + Backspace
        backspace_test("(aaa)→←", "→←",
                       Movement::Line(Direction::Upstream),
                       None, &mut vec![],
                       &mut vec![true, false, false, false]
        );

        // Select delete
        backspace_test("→a←", "→←",
                       Movement::Grapheme(Direction::Upstream),
                       None, &mut vec![],
                       &mut vec![]
        );

        // Large select delete
        backspace_test("→abcdefghijklmnopqrstuvwxyz←", "→←",
                       Movement::Grapheme(Direction::Upstream),
                       None, &mut vec![],
                       &mut vec![]
        );

        // Unicode select delete
        backspace_test("→༄←", "→←",
                       Movement::Grapheme(Direction::Upstream),
                       None, &mut vec![],
                       &mut vec![]
        );

        // Delete unicode
        backspace_test("༄→←", "→←",
                       Movement::Grapheme(Direction::Upstream),
                       None, &mut vec![],
                       &mut vec![]
        );

        // Large unicode select delete
        backspace_test("→߷߷߷༄༄༄←", "→←",
                       Movement::Grapheme(Direction::Upstream),
                       None, &mut vec![],
                       &mut vec![]
        );

        // Single delete, large unicode
        backspace_test("߷߷߷༄༄༄→←", "߷߷߷༄༄→←",
                       Movement::Grapheme(Direction::Upstream),
                       None, &mut vec![], &mut vec![]
        );

        // Select, mixed, unicode/ascii
        backspace_test("→a߷b߷c߷d༄e༄f༄g←", "→←",
                       Movement::Grapheme(Direction::Upstream),
                       None, &mut vec![],
                       &mut vec![]
        );

        // Single delete, large unicode/ascii
        backspace_test("a߷b߷c߷d༄e༄f༄g→←", "a߷b߷c߷d༄e༄f༄→←",
                       Movement::Grapheme(Direction::Upstream),
                       None, &mut vec![],
                       &mut vec![]
        );

        // Ctrl + Backspace, spaces
        backspace_test("lilypad is so cool →←", "→←",
                       Movement::Line(Direction::Upstream),
                       None, &mut vec![],
                       &mut vec![]
        );

        // Ctrl + Backspace, newline
        backspace_test("lilypad is so\n cool →←", "lilypad is so\n →←",
                       Movement::Line(Direction::Upstream),
                       None, &mut vec![],
                       &mut vec![]
        );
    }

    /* --------------------------------- helpers -------------------------------- */
    fn char_insert_test(start: &str, add: &str, target: &str) {
        let (mut src, start_sel) = generate_state(start);
        let (target_src, target_sel) = generate_state(target);
        let (edit, end_sel) =
            edit_for_insert_char(start_sel, &src, add, &mut Vec::new(), &mut Vec::new());

        edit.unwrap().apply_to_rope(&mut src);

        assert_eq!(src, target_src);
        assert_eq!(end_sel, target_sel);
    }

    fn indent_test(start: &str, target: &str) {
        let (mut src, start_sel) = generate_state(start);
        let (target_src, target_sel) = generate_state(target);
        let (edit, end_sel) = edit_for_indent(start_sel, &src);

        edit.apply_to_rope(&mut src);

        assert_eq!(src, target_src);
        assert_eq!(end_sel, target_sel);
    }

    fn unindent_test(start: &str, target: &str) {
        let (mut src, start_sel) = generate_state(start);
        let (target_src, target_sel) = generate_state(target);
        let (edit, end_sel) = edit_for_unindent(start_sel, &src);

        edit.apply_to_rope(&mut src);

        assert_eq!(src, target_src);
        assert_eq!(end_sel, target_sel);
    }

    fn backspace_test(
        start: &str, 
        target: &str, 
        movement: Movement,
        pseudo_selection: Option<TextRange>,
        input_ignore_stack: &mut Vec<&'static str>,
        paired_delete_stack: &mut Vec<bool>
    ) {
        let (mut src, start_sel) = generate_state(start);
        let (target_src, target_sel) = generate_state(target);
        let (edit, end_sel) = edit_for_backspace(start_sel, &src, movement, pseudo_selection, input_ignore_stack, paired_delete_stack);

        edit.unwrap().apply_to_rope(&mut src);

        assert_eq!(src, target_src);
        assert_eq!(end_sel, target_sel);
    }

    /// Generates a rope and selection from a string for testing.
    /// The start of a selection is marked with a → (u2192) and the end of a selection is marked with a ← (u2190).
    /// The arrows are removed from the returned rope.
    fn generate_state(str: &str) -> (Rope, TextRange) {
        let mut sel_start = TextPoint::ZERO;
        let mut sel_end = TextPoint::ZERO;
        for (line_num, line) in str.lines().enumerate() {
            let start = line.chars().position(|c| c == '→');
            let end = line.chars().position(|c| c == '←');

            if let Some(start) = start {
                sel_start = TextPoint::new(line_num, start);
            }
            if let Some(end) = end {
                sel_end = TextPoint::new(line_num, end);
            }

            // if on the same line, adjust the second to account for the first being removed
            if start.is_some() && end.is_some() {
                if sel_start.col < sel_end.col {
                    sel_end.col -= 1;
                } else {
                    sel_start.col -= 1;
                }
            }
        }

        let source_no_markers = str.replace(['→', '←'], "");
        (
            Rope::from(source_no_markers),
            TextRange::new(sel_start, sel_end),
        )
    }
}
