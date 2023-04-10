use serde::{Deserialize, Deserializer};
use std::ops::Range;

/* ------------------------------- Text Range ------------------------------- */

#[derive(Debug, PartialEq, Clone)]
pub struct TextRange {
    pub start: IntPoint,
    pub end: IntPoint,
}

impl TextRange {
    pub const ZERO: Self = TextRange {
        start: IntPoint::ZERO,
        end: IntPoint::ZERO,
    };

    #[allow(dead_code)] // will be used later
    pub fn new(start: IntPoint, end: IntPoint) -> Self {
        TextRange { start, end }
    }

    pub fn new_cursor(x: usize, y: usize) -> Self {
        TextRange {
            start: IntPoint::new(x, y),
            end: IntPoint::new(x, y),
        }
    }

    pub fn is_cursor(&self) -> bool {
        self.start == self.end
    }

    pub fn ordered(&self) -> TextRange {
        if self.start.y < self.end.y {
            TextRange {
                start: self.start,
                end: self.end,
            }
        } else if self.start.y > self.end.y {
            TextRange {
                start: self.end,
                end: self.start,
            }
        } else if self.start.x < self.end.x {
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

    pub fn contains(&self, point: IntPoint) -> bool {
        // TODO: multiline
        point.y >= self.start.y
            && point.y <= self.end.y
            && point.x >= self.start.x
            && point.x <= self.end.x
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
            IntPoint::new(
                arr[0].get("character").unwrap().as_u64().unwrap() as usize,
                arr[0].get("line").unwrap().as_u64().unwrap() as usize,
            ),
            IntPoint::new(
                arr[1].get("character").unwrap().as_u64().unwrap() as usize,
                arr[1].get("line").unwrap().as_u64().unwrap() as usize,
            ),
        ))
    }
}

/* -------------------------------- Int Point ------------------------------- */

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct IntPoint {
    pub x: usize,
    pub y: usize,
}

impl IntPoint {
    pub const ZERO: Self = IntPoint { x: 0, y: 0 };

    pub fn new(x: usize, y: usize) -> IntPoint {
        IntPoint { x, y }
    }

    pub fn as_tree_sitter(&self) -> tree_sitter_c2rust::Point {
        tree_sitter_c2rust::Point::new(self.x, self.y)
    }

    pub fn offset_in(&self, string: &str) -> usize {
        let mut offset: usize = 0;
        for (num, line) in string.lines().enumerate() {
            if num == self.y {
                // position in the current line
                // gets the byte offset of the cursor within the current line
                // (supports utf-8 characters)
                offset += line
                    .char_indices()
                    .nth(self.x)
                    .map(|x| x.0)
                    .unwrap_or(line.len());
                break;
            }

            offset += line.len() + os_linebreak().len(); // factor in the linebreak
        }
        offset
    }
}

/* ---------------------------------- Extra --------------------------------- */

#[derive(serde::Deserialize, Debug)]
pub struct TextEdit {
    pub text: String,
    pub range: TextRange,
}

pub const fn os_linebreak() -> &'static str {
    if cfg!(target_os = "windows") {
        "\r\n"
    } else {
        "\n"
    }
}
