use std::ops::Range;

pub struct Selection {
    pub start: IntPoint,
    pub end: IntPoint,
}

impl Selection {
    pub const ZERO: Self = Selection {
        start: IntPoint::ZERO,
        end: IntPoint::ZERO,
    };

    #[allow(dead_code)] // will be used later
    pub fn new(start: IntPoint, end: IntPoint) -> Self {
        Selection { start, end }
    }

    pub fn new_cursor(x: usize, y: usize) -> Self {
        Selection {
            start: IntPoint::new(x, y),
            end: IntPoint::new(x, y),
        }
    }

    pub fn is_cursor(&self) -> bool {
        self.start == self.end
    }

    pub fn ordered(&self) -> Selection {
        if self.start.y < self.end.y {
            Selection {
                start: self.start,
                end: self.end,
            }
        } else if self.start.y > self.end.y {
            Selection {
                start: self.end,
                end: self.start,
            }
        } else if self.start.x < self.end.x {
            Selection {
                start: self.start,
                end: self.end,
            }
        } else {
            Selection {
                start: self.end,
                end: self.start,
            }
        }
    }

    pub fn offset_in(&self, string: &str) -> TextRange {
        TextRange {
            start: self.start.offset_in(string),
            end: self.end.offset_in(string),
        }
    }
}

pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub fn as_range(&self) -> Range<usize> {
        self.start..self.end
    }
}

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

pub const fn os_linebreak() -> &'static str {
    if cfg!(target_os = "windows") {
        "\r\n"
    } else {
        "\n"
    }
}
