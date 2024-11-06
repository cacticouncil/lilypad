use serde::{Deserialize, Serialize};

use crate::block_editor::text_range::{TextPoint, TextRange};

use crate::vscode;

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Documentation {
    pub message: String,
    pub range: TextRange,
}

impl Documentation {
    pub fn request_hover(&self) {
        vscode::request_hover(self.range.start.line, self.range.start.col);
    }
    pub fn set_hover(&mut self, message: String) {
        self.message = message;
    }

    #[allow(dead_code)]
    pub fn example() -> Documentation {
        Documentation {
            message: "example Documentation".to_string(),
            range: TextRange::new(TextPoint::new(2, 18), TextPoint::new(2, 25)),
        }
    }
}
