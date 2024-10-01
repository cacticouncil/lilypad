use std::collections::HashSet;

mod block_dragging;
mod completion_popup;
mod coord_conversions;
mod diagnostics_popup;
mod documentation_popup;
mod gutter;
pub mod selections;
mod widget;

use super::text_drawer::*;
use super::text_range::*;
use crate::block_editor::{block_drawer, source::TextEdit, text_range::TextRange};
use crate::lsp::diagnostics::Diagnostic;
use completion_popup::CompletionPopup;
use diagnostics_popup::DiagnosticPopup;
use documentation_popup::DocumentationPopup;
use selections::Selections;

pub struct TextEditor {
    /// the actual and pseudo selection
    selections: Selections,

    // if IME candidate window is shown
    ime_enabled: bool,

    // selection for IME candidate window
    ime_selection: TextRange,

    /// diagnostics for current cursor position
    diagnostics: Vec<Diagnostic>,

    /// index of diagnostic selected in the popup
    diagnostic_selection: Option<usize>,

    /// object to calculate text views
    text_drawer: TextDrawer,

    /// blocks to draw
    blocks: Vec<block_drawer::Block>,

    /// the padding above each individual line
    padding: Vec<f32>,

    /// line numbers that have breakpoints
    breakpoints: HashSet<usize>,

    /// line number of the selected stack frame and the deepest stack frame
    stack_frame: StackFrameLines,

    /// overlay view for diagnostics
    diagnostic_popup: DiagnosticPopup,

    /// overlay view for completions
    completion_popup: CompletionPopup,

    /// overlay view for hover
    documentation_popup: DocumentationPopup,
}

#[derive(Clone, Copy)]
pub struct StackFrameLines {
    pub selected: Option<usize>,
    pub deepest: Option<usize>,
}

impl StackFrameLines {
    pub fn empty() -> Self {
        StackFrameLines {
            selected: None,
            deepest: None,
        }
    }
}

impl TextEditor {
    pub fn new() -> Self {
        TextEditor {
            selections: Selections::new(),
            ime_enabled: false,
            ime_selection: TextRange::ZERO,
            diagnostics: vec![],
            diagnostic_selection: Option::None,
            text_drawer: TextDrawer::new(),
            blocks: vec![],
            padding: vec![],
            breakpoints: HashSet::new(),
            stack_frame: StackFrameLines::empty(),
            diagnostic_popup: DiagnosticPopup::new(),
            completion_popup: CompletionPopup::new(),
            documentation_popup: DocumentationPopup::new(),
        }
    }
}
