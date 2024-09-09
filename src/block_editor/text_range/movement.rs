use ropey::Rope;

use crate::block_editor::rope_ext::{RopeExt, RopeSliceExt};

use super::{TextPoint, TextRange};

/// A movement that can be applied to a text position.
/// Consists of a unit and a direction.
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
        use super::movement::{HDir::*, HUnit, TextMovement::*, VDir::*, VUnit};
        match movement {
            Horizontal { unit, direction } => match unit {
                HUnit::Grapheme => match direction {
                    Left => self.cursor_at_left(source, expanding),
                    Right => self.cursor_at_right(source, expanding),
                },
                HUnit::Word => match direction {
                    Left => self.cursor_at_prev_word_start(source),
                    Right => self.cursor_at_next_word_end(source),
                },
                HUnit::Line => match direction {
                    Left => self.cursor_at_line_start(source),
                    Right => self.cursor_at_line_end(source),
                },
            },
            Vertical { unit, direction } => match unit {
                VUnit::Line => match direction {
                    Up => self.cursor_at_line_above(source),
                    Down => self.cursor_at_line_below(source),
                },
                VUnit::Document => match direction {
                    Up => TextPoint::ZERO,
                    Down => self.cursor_at_doc_end(source),
                },
            },
        }
    }

    fn cursor_at_line_above(&self, source: &Rope) -> TextPoint {
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
                source.clamp_col(cursor_pos.line - 1, cursor_pos.col),
            )
        }
    }

    fn cursor_at_line_below(&self, source: &Rope) -> TextPoint {
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
            TextPoint::new(next_line, source.clamp_col(next_line, cursor_pos.col))
        }
    }

    fn cursor_at_left(&self, source: &Rope, expanding: bool) -> TextPoint {
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

    fn cursor_at_right(&self, source: &Rope, expanding: bool) -> TextPoint {
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

    fn cursor_at_prev_word_start(&self, source: &Rope) -> TextPoint {
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

    fn cursor_at_next_word_end(&self, source: &Rope) -> TextPoint {
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

    fn cursor_at_line_start(&self, source: &Rope) -> TextPoint {
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

    fn cursor_at_line_end(&self, source: &Rope) -> TextPoint {
        let cursor_pos = self.end;
        TextPoint::new(
            cursor_pos.line,
            source.line(cursor_pos.line).len_chars_no_linebreak(),
        )
    }

    fn cursor_at_doc_end(&self, source: &Rope) -> TextPoint {
        let last_line = source.len_lines() - 1;
        let last_line_len = source.line(last_line).len_chars_no_linebreak();
        TextPoint::new(last_line, last_line_len)
    }
}
