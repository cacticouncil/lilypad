use ropey::Rope;
use serde::{Deserialize, Deserializer};
use std::ops::Range;

/* ------------------------------- Text Range ------------------------------- */

/// An range of text points. Half open: [start, end).
/// A cursor is a special case where the start and end are the same.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TextRange {
    pub start: TextPoint,
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

    pub fn contains(&self, point: TextPoint, source: &Rope) -> bool {
        if self.start.line == self.end.line {
            // if a single line, can just check the column
            point.line == self.start.line
                && point.col >= self.start.col
                && point.col <= self.end.col
        } else if point.line == self.start.line {
            // if on the first line, check that the column is greater than the start
            point.col >= self.start.col
        } else if point.line == self.end.line {
            // if on the last line, check that the column is less than the end
            point.col <= self.end.col
        } else if point.line < self.start.line || point.line > self.end.line {
            // if outside the range of lines, it is false
            false
        } else {
            // if somewhere in the middle, check the length of the line
            point.col <= source.line(point.line).len_chars()
        }
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

impl From<tree_sitter_c2rust::Point> for TextPoint {
    fn from(ts_pt: tree_sitter_c2rust::Point) -> Self {
        TextPoint {
            line: ts_pt.row,
            col: ts_pt.column,
        }
    }
}

impl From<TextPoint> for tree_sitter_c2rust::Point {
    fn from(text_pt: TextPoint) -> Self {
        Self {
            row: text_pt.line,
            column: text_pt.col,
        }
    }
}
