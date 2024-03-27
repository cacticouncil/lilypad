use std::time::Duration;

use druid::TimerToken;
use druid::WidgetPod;

mod block_dragging;
mod completion_popup;
mod diagnostics_popup;
mod gutter_drawer;
mod ime;
mod lifecycle;
mod selection_changing;
mod selection_drawer;
mod text_editing;
mod undo_manager;

use self::undo_manager::UndoManager;
use super::block_drawer;
use super::text_drawer::*;
use super::text_range::*;
use super::EditorModel;
use crate::lang::LanguageConfig;
use crate::parse::TreeManager;
use completion_popup::CompletionPopup;
use diagnostics_popup::DiagnosticPopup;
use ime::ImeComponent;

pub use text_editing::TextEdit;

const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(700);

pub struct TextEditor {
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

    /// the timer that toggles the cursor
    cursor_timer: TimerToken,

    /// if the blinking cursor is visible
    cursor_visible: bool,

    /// object to calculate text views
    text_drawer: TextDrawer,

    /// if the blocks and text need to be re-rendered
    text_changed: bool,

    /// blocks to draw
    blocks: Vec<block_drawer::Block>,

    /// the padding above each individual line
    padding: Vec<f64>,

    /// pairs that were inserted and should be ignored on the next input
    input_ignore_stack: Vec<&'static str>,

    /// tracking which characters had pairs inserted with them, and should take
    /// the pair down with them if they are deleted
    paired_delete_stack: Vec<bool>,

    /// The line is where to draw the drag insertion line. The column is how much indent the insertion should have.
    drag_insertion_line: Option<TextPoint>,

    /// overlay view for diagnostics
    diagnostic_popup: WidgetPod<EditorModel, DiagnosticPopup>,

    /// overlay view for completions
    completion_popup: WidgetPod<EditorModel, CompletionPopup>,

    /// connects to the system IME to handle text input
    ime: ImeComponent,
}

impl TextEditor {
    pub fn new(lang: &'static LanguageConfig) -> Self {
        TextEditor {
            tree_manager: TreeManager::new(lang),
            language: lang,
            undo_manager: UndoManager::new(),
            selection: TextRange::ZERO,
            pseudo_selection: None,
            cursor_timer: TimerToken::INVALID,
            cursor_visible: true,
            text_drawer: TextDrawer::new(lang),
            text_changed: true,
            blocks: vec![],
            padding: vec![],
            input_ignore_stack: vec![],
            paired_delete_stack: vec![],
            drag_insertion_line: None,
            diagnostic_popup: WidgetPod::new(DiagnosticPopup::new()),
            completion_popup: WidgetPod::new(CompletionPopup::new()),
            ime: ImeComponent::default(),
        }
    }
}
