use ropey::Rope;
use serde::{Deserialize, Deserializer};
use std::ops::Range;

use super::rope_ext::RopeSliceExt;

pub mod movement;

/* ------------------------------- Text Range ------------------------------- */

/// An range of text points. Half open: [start, end).
/// A cursor is a special case where the start and end are the same.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TextRange {
    /// When selecting, this is where the mouse was pressed
    pub start: TextPoint,

    /// When selecting, this is where the mouse was released.
    /// It is where the blinking cursor is. And is what will normally change during expansions.
    pub end: TextPoint,
}

impl TextRange {
    pub const ZERO: Self = TextRange {
        start: TextPoint::ZERO,
        end: TextPoint::ZERO,
    };

    pub fn new(start: TextPoint, end: TextPoint) -> Self {
        TextRange { start, end }
    }

    pub fn new_cursor(pt: TextPoint) -> Self {
        TextRange { start: pt, end: pt }
    }

    pub fn is_cursor(&self) -> bool {
        self.start == self.end
    }

    pub fn ordered(&self) -> TextRange {
        if self.start.line < self.end.line {
            TextRange {
                start: self.start,
                end: self.end,
            }
        } else if self.start.line > self.end.line {
            TextRange {
                start: self.end,
                end: self.start,
            }
        } else if self.start.col < self.end.col {
            TextRange {
                start: self.start,
                end: self.end,
            }
        } else {
            TextRange {
                start: self.end,
                end: self.start,
            }
        }
    }

    pub fn byte_range_in(&self, source: &Rope) -> Range<usize> {
        self.start.byte_idx_in(source)..self.end.byte_idx_in(source)
    }

    pub fn char_range_in(&self, source: &Rope) -> Range<usize> {
        self.start.char_idx_in(source)..self.end.char_idx_in(source)
    }

    pub fn from_char_range_in(source: &Rope, range: Range<usize>) -> Self {
        TextRange {
            start: TextPoint::new(
                source.char_to_line(range.start),
                range.start - source.line_to_char(source.char_to_line(range.start)),
            ),
            end: TextPoint::new(
                source.char_to_line(range.end),
                range.end - source.line_to_char(source.char_to_line(range.end)),
            ),
        }
    }

    pub fn contains(&self, point: TextPoint, source: &Rope) -> bool {
        if self.start.line == self.end.line {
            // if a single line, can just check the column
            point.line == self.start.line
                && point.col >= self.start.col
                && point.col <= self.end.col
        } else if point.line < self.start.line || point.line > self.end.line {
            // if outside the range of lines, it is false
            false
        } else if point.line == self.end.line {
            // if on the last line, check that the column is less than the end
            point.col <= self.end.col
        } else if point.line == self.start.line {
            // if on the first line, check that the column is greater than the start
            // and less than the end of the line
            point.col >= self.start.col
                && point.line < source.len_lines() // make sure the line is in the source
                && point.col <= source.line(point.line).len_chars()
        } else {
            // if somewhere in the middle, check it is before the end of the line
            point.line < source.len_lines() // make sure the line is in the source
                && point.col <= source.line(point.line).len_chars()
        }
    }

    /// An individual visual text range for each line, with respect to the source rope.
    /// Note: these ranges are the visual ranges so they leave out the newline characters.
    pub fn individual_lines(&self, source: &Rope) -> Vec<TextRange> {
        let mut ranges = Vec::new();
        let ordered = self.ordered();
        for line in ordered.start.line..=ordered.end.line {
            // if the line is outside of the source, don't include it
            if line >= source.len_lines() {
                break;
            }

            // if the line is the start, start at the start of the range
            // otherwise, start at the beginning of the line
            let start = if line == ordered.start.line {
                ordered.start
            } else {
                TextPoint::new(line, 0)
            };

            // if the line is the end, end at the end of the range
            // otherwise, end at the end of the line
            let end = if line == ordered.end.line {
                ordered.end
            } else {
                TextPoint::new(line, source.line(line).len_chars_no_linebreak())
            };

            ranges.push(TextRange::new(start, end));
        }
        ranges
    }
}

impl<'de> Deserialize<'de> for TextRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // TODO: actual error handling
        let json = serde_json::Value::deserialize(deserializer)?;
        let arr = json.as_array().unwrap();
        Ok(TextRange::new(
            TextPoint::new(
                arr[0].get("line").unwrap().as_u64().unwrap() as usize,
                arr[0].get("character").unwrap().as_u64().unwrap() as usize,
            ),
            TextPoint::new(
                arr[1].get("line").unwrap().as_u64().unwrap() as usize,
                arr[1].get("character").unwrap().as_u64().unwrap() as usize,
            ),
        ))
    }
}

/* -------------------------------- Int Point ------------------------------- */

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct TextPoint {
    pub line: usize,
    pub col: usize,
}

impl TextPoint {
    pub const ZERO: Self = TextPoint { line: 0, col: 0 };

    pub fn new(line: usize, col: usize) -> TextPoint {
        TextPoint { line, col }
    }

    pub fn byte_idx_in(&self, source: &Rope) -> usize {
        let char_idx = source.line_to_char(self.line) + self.col;
        source.char_to_byte(char_idx)
    }

    pub fn char_idx_in(&self, source: &Rope) -> usize {
        source.line_to_char(self.line) + self.col
    }
}

impl PartialOrd for TextPoint {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.line.partial_cmp(&other.line) {
            Some(core::cmp::Ordering::Equal) => self.col.partial_cmp(&other.col),
            ord => ord,
        }
    }
}

impl From<tree_sitter::Point> for TextPoint {
    fn from(ts_pt: tree_sitter::Point) -> Self {
        TextPoint {
            line: ts_pt.row,
            col: ts_pt.column,
        }
    }
}

impl From<TextPoint> for tree_sitter::Point {
    fn from(text_pt: TextPoint) -> Self {
        Self {
            row: text_pt.line,
            column: text_pt.col,
        }
    }
}
