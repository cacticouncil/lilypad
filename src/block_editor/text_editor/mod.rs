use ropey::Rope;
use std::collections::HashSet;

mod block_dragging;
mod completion_popup;
mod diagnostics_popup;
mod gutter_drawer;
mod selection_changing;
mod selection_drawer;
mod text_editing;
mod undo_manager;
mod widget;

use super::text_drawer::*;
use super::text_range::*;
use crate::block_editor::{block_drawer, text_range::TextRange};
use crate::lang::LanguageConfig;
use crate::lsp::diagnostics::Diagnostic;
use crate::parse::TreeManager;
use completion_popup::CompletionPopup;
use diagnostics_popup::DiagnosticPopup;
pub use text_editing::TextEdit;
use undo_manager::UndoManager;

/// the interval during which the cursor is on during the blink cycle (in seconds)
const CURSOR_ON_DURATION: f64 = 0.8;

/// the interval during which the cursor is on during the blink cycle (in seconds)
const CURSOR_OFF_DURATION: f64 = 0.4;

pub struct TextEditor {
    /// the source code to edit
    source: Rope,

    /// generates syntax tree from source code
    tree_manager: TreeManager,

    /// the current language used by the editor
    language: &'static LanguageConfig,

    /// handles undo/redo
    undo_manager: UndoManager,

    /// the currently selected text
    selection: TextRange,

    /// the frame that hitting backspace would delete
    pseudo_selection: Option<TextRange>,

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

    /// if the blocks and text need to be re-rendered
    text_changed: bool,

    /// blocks to draw
    blocks: Vec<block_drawer::Block>,

    /// the padding above each individual line
    padding: Vec<f32>,

    /// line numbers that have breakpoints
    breakpoints: HashSet<usize>,

    /// line number of the selected stack frame and the deepest stack frame
    stack_frame: StackFrameLines,

    /// pairs that were inserted and should be ignored on the next input
    input_ignore_stack: Vec<&'static str>,

    /// tracking which characters had pairs inserted with them, and should take
    /// the pair down with them if they are deleted
    paired_delete_stack: Vec<bool>,

    /// overlay view for diagnostics
    diagnostic_popup: DiagnosticPopup,

    /// overlay view for completions
    completion_popup: CompletionPopup,

    /// the time that the current frame started
    frame_start_time: f64,

    /// the time of the last selection change (used for cursor blinking)
    last_selection_time: f64,
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
    pub fn new(source: Rope, lang: &'static LanguageConfig) -> Self {
        TextEditor {
            source,
            tree_manager: TreeManager::new(lang),
            language: lang,
            undo_manager: UndoManager::new(),
            selection: TextRange::ZERO,
            pseudo_selection: None,
            ime_enabled: false,
            ime_selection: TextRange::ZERO,
            diagnostics: vec![],
            diagnostic_selection: Option::None,
            text_drawer: TextDrawer::new(lang),
            text_changed: true,
            blocks: vec![],
            padding: vec![],
            breakpoints: HashSet::new(),
            stack_frame: StackFrameLines::empty(),
            input_ignore_stack: vec![],
            paired_delete_stack: vec![],
            diagnostic_popup: DiagnosticPopup::new(),
            completion_popup: CompletionPopup::new(),
            frame_start_time: 0.0,
            last_selection_time: 0.0,
        }
    }
}
