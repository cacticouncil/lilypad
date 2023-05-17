use ropey::Rope;
use serde::{Deserialize, Deserializer};
use std::ops::Range;

/* ------------------------------- Text Range ------------------------------- */

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
        if self.start.row < self.end.row {
            TextRange {
                start: self.start,
                end: self.end,
            }
        } else if self.start.row > self.end.row {
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

    pub fn contains(&self, point: TextPoint) -> bool {
        // TODO: multiline
        point.row >= self.start.row
            && point.row <= self.end.row
            && point.col >= self.start.col
            && point.col <= self.end.col
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
                arr[0].get("character").unwrap().as_u64().unwrap() as usize,
                arr[0].get("line").unwrap().as_u64().unwrap() as usize,
            ),
            TextPoint::new(
                arr[1].get("character").unwrap().as_u64().unwrap() as usize,
                arr[1].get("line").unwrap().as_u64().unwrap() as usize,
            ),
        ))
    }
}

/* -------------------------------- Int Point ------------------------------- */

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct TextPoint {
    // TODO: flip these
    pub col: usize,
    pub row: usize,
}

impl TextPoint {
    pub const ZERO: Self = TextPoint { col: 0, row: 0 };

    pub fn new(col: usize, row: usize) -> TextPoint {
        TextPoint { col, row }
    }

    pub fn byte_idx_in(&self, source: &Rope) -> usize {
        let char_idx = source.line_to_char(self.row) + self.col;
        source.char_to_byte(char_idx)
    }

    pub fn char_idx_in(&self, source: &Rope) -> usize {
        source.line_to_char(self.row) + self.col
    }
}

impl From<tree_sitter_c2rust::Point> for TextPoint {
    fn from(ts_pt: tree_sitter_c2rust::Point) -> Self {
        TextPoint {
            col: ts_pt.column,
            row: ts_pt.row,
        }
    }
}

impl From<TextPoint> for tree_sitter_c2rust::Point {
    fn from(text_pt: TextPoint) -> Self {
        Self {
            row: text_pt.row,
            column: text_pt.col,
        }
    }
}

/* ---------------------------------- Extra --------------------------------- */

#[derive(serde::Deserialize, Debug)]
pub struct TextEdit {
    pub text: String,
    pub range: TextRange,
}
