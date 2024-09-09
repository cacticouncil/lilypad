use std::borrow::Cow;
use tree_sitter::InputEdit;

use crate::{
    block_editor::{text_range::TextPoint, TextRange},
    vscode,
};

use super::Source;

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

    pub fn new_end(&self) -> TextPoint {
        self.new_end_point
    }

    #[cfg(test)]
    pub fn apply_to_rope(&self, source: &mut ropey::Rope) {
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

impl Source {
    /// Apply the text edit on the rope and tree manager. Returns the inverse text edit.
    pub(super) fn apply(&mut self, edit: &TextEdit) -> TextEdit<'static> {
        let char_range = edit.range.char_range_in(&self.text);
        let byte_range = edit.range.byte_range_in(&self.text);

        // update buffer
        let removed = self
            .text
            .get_slice(char_range.clone())
            .map(|x| x.to_string());
        self.text.remove(char_range.clone());
        self.text.insert(char_range.start, &edit.text);

        // update tree
        let tree_edit = InputEdit {
            start_byte: byte_range.start,
            old_end_byte: byte_range.end,
            new_end_byte: byte_range.start + edit.text.len(),
            start_position: edit.range.start.into(),
            old_end_position: edit.range.end.into(),
            new_end_position: edit.new_end().into(),
        };
        self.tree_manager
            .update(&self.text, tree_edit, &mut self.lang);

        // update vscode if not from vscode
        if edit.origin != TextEditOrigin::Vscode {
            vscode::edited(
                &edit.text,
                edit.range.start.line,
                edit.range.start.col,
                edit.range.end.line,
                edit.range.end.col,
            );
        }

        let affected_range = TextRange::new(edit.range.start, edit.new_end_point);
        if let Some(removed) = removed {
            TextEdit::new(Cow::Owned(removed), affected_range)
        } else {
            TextEdit::delete(affected_range)
        }
    }
}
