use ropey::Rope;
use std::borrow::Cow;
use tree_sitter::InputEdit;

use crate::{
    block_editor::{text_range::TextPoint, TextRange},
    vscode,
};

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
