use crate::block_editor::text_range::TextRange;

mod changing;
mod drawer;

/// the interval during which the cursor is on during the blink cycle (in seconds)
const CURSOR_ON_DURATION: f64 = 0.8;

/// the interval during which the cursor is on during the blink cycle (in seconds)
const CURSOR_OFF_DURATION: f64 = 0.4;

/// The actual text selection and the associated pseudo selection
pub struct Selections {
    /// the currently selected text
    selection: TextRange,

    /// the frame that hitting backspace would delete
    pseudo_selection: Option<TextRange>,

    /// the time that the current frame started
    frame_start_time: f64,

    /// the time of the last selection change (used for cursor blinking)
    last_selection_time: f64,
}

impl Selections {
    pub fn new() -> Self {
        Selections {
            selection: TextRange::ZERO,
            pseudo_selection: None,
            frame_start_time: 0.0,
            last_selection_time: 0.0,
        }
    }

    pub fn selection(&self) -> TextRange {
        self.selection
    }

    pub fn pseudo_selection(&self) -> Option<TextRange> {
        self.pseudo_selection
    }

    pub fn set_frame_start_time(&mut self, time: f64) {
        self.frame_start_time = time;
    }
}
