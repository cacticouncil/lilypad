use druid::{widget::Scroll, Data, TimerToken, Widget, WidgetPod};
use ropey::Rope;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::parse::TreeManager;

mod block_drawer;
pub mod diagnostics;
mod gutter_drawer;
mod highlighter;
mod lifecycle;
mod rope_ext;
mod selection_changing;
mod selection_drawer;
mod text_drawer;
mod text_editing;
pub mod text_range;

use diagnostics::Diagnostic;
use text_drawer::*;
use text_range::*;

use self::diagnostics::DiagnosticPopup;

//controls cursor blinking speed
pub const TIMER_INTERVAL: Duration = Duration::from_millis(700);

/*
Got these values by running:
    let font = FontDescriptor::new(FontFamily::new_unchecked("Roboto Mono")).with_size(15.0);
    let mut layout = TextLayout::<String>::from_text("A".to_string());
    layout.set_font(font);
    layout.rebuild_if_needed(ctx.text(), env);
    let size = layout.size();
    println!("{:}", size);
*/
pub const FONT_WIDTH: f64 = 9.0;
pub const FONT_HEIGHT: f64 = 20.0;
pub const FONT_SIZE: f64 = 15.0;

/// padding around edges of entire editor
const OUTER_PAD: f64 = 16.0;

/// left padding on text (to position it nicer within the blocks)
const TEXT_L_PAD: f64 = 2.0;

/// width for the line number gutter
const GUTTER_WIDTH: f64 = 30.0;

/// convenience constant for all the padding that impacts text layout
const TOTAL_TEXT_X_OFFSET: f64 = OUTER_PAD + GUTTER_WIDTH + TEXT_L_PAD;

pub fn widget() -> impl Widget<EditorModel> {
    Scroll::new(BlockEditor::new()).content_must_fill(true)
}

struct BlockEditor {
    /// generates syntax tree from source code
    tree_manager: TreeManager,

    /// the currently selected text
    selection: TextRange,

    /// the frame that hitting backspace would delete
    pseudo_selection: Option<TextRange>,

    /// if the left mouse button is currently pressed
    mouse_pressed: bool,

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

    /// padding between each line
    padding: Vec<f64>,

    /// overlay view for diagnostics
    diagnostic_popup: WidgetPod<EditorModel, DiagnosticPopup>,

    /// pairs that were inserted and should be ignored on the next input
    input_ignore_stack: Vec<&'static str>,

    /// tracking which characters had pairs inserted with them, and should take
    /// the pair down with them if they are deleted
    paired_delete_stack: Vec<bool>,
}

#[derive(Clone, Data)]
pub struct EditorModel {
    pub source: Arc<Mutex<Rope>>,
    #[data(eq)]
    pub diagnostics: Vec<Diagnostic>,
    #[data(eq)]
    pub diagnostic_selection: Option<u64>,
}

impl BlockEditor {
    fn new() -> Self {
        BlockEditor {
            tree_manager: TreeManager::new(),
            selection: TextRange::ZERO,
            pseudo_selection: None,
            mouse_pressed: false,
            cursor_timer: TimerToken::INVALID,
            cursor_visible: true,
            text_drawer: TextDrawer::new(),
            text_changed: true,
            blocks: vec![],
            padding: vec![],
            diagnostic_popup: WidgetPod::new(DiagnosticPopup::new()),
            input_ignore_stack: vec![],
            paired_delete_stack: vec![],
        }
    }
}
