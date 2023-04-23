use serde::{Deserialize, Deserializer};
use std::ops::Range;

use super::text_util::detect_linebreak;

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

    pub fn new_cursor(x: usize, y: usize) -> Self {
        TextRange {
            start: TextPoint::new(x, y),
            end: TextPoint::new(x, y),
        }
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

    pub fn offset_in(&self, string: &str) -> Range<usize> {
        self.start.offset_in(string)..self.end.offset_in(string)
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

    pub fn offset_in(&self, string: &str) -> usize {
        let mut offset: usize = 0;
        for (num, line) in string.lines().enumerate() {
            if num == self.row {
                // position in the current line
                // gets the byte offset of the cursor within the current line
                // (supports utf-8 characters)
                offset += line
                    .char_indices()
                    .nth(self.col)
                    .map(|x| x.0)
                    .unwrap_or(line.len());
                break;
            }

            offset += line.len() + detect_linebreak(string).len(); // factor in the linebreak
        }
        offset
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

impl Into<tree_sitter_c2rust::Point> for TextPoint {
    fn into(self) -> tree_sitter_c2rust::Point {
        tree_sitter_c2rust::Point {
            row: self.row,
            column: self.col,
        }
    }
}

/* ---------------------------------- Extra --------------------------------- */

#[derive(serde::Deserialize, Debug)]
pub struct TextEdit {
    pub text: String,
    pub range: TextRange,
}
