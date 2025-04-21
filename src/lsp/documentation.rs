
use crate::block_editor::text_range::{TextPoint, TextRange};
use crate::vscode;

#[derive(Debug, PartialEq, Clone)]
pub struct Documentation {
    pub message: String,
    pub range: TextRange,
}

impl Documentation {
    pub fn set_hover(&mut self, message: String, range: TextRange) {
        self.message = message;
        self.range = range;
    }
    pub fn request_hover(&mut self, line: usize, col: usize) {
        vscode::request_hover(line, col);
    }

    pub fn new() -> Documentation {
        Documentation {
            message: " ".to_string(),
            range: TextRange::ZERO,
        }
    }

    #[allow(dead_code)]
    pub fn example() -> Documentation {
        Documentation {
            message: "example Documentation".to_string(),
            range: TextRange::new(TextPoint::new(2, 18), TextPoint::new(2, 25)),
        }
    }
}
