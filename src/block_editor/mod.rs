use druid::{Data, Lens, TimerToken, WidgetPod};
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;

use crate::parse::TreeManager;

mod block_drawer;
pub mod diagnostics;
mod gutter_drawer;
mod lifecycle;
mod selection_changing;
mod selection_drawer;
mod text_drawer;
mod text_editing;
pub mod text_range;
mod highlighter;

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

const OUTER_PAD: f64 = 16.0;
const TEXT_L_PAD: f64 = 2.0;
const GUTTER_WIDTH: f64 = 30.0;
const TOTAL_TEXT_X_OFFSET: f64 = OUTER_PAD + GUTTER_WIDTH + TEXT_L_PAD;

pub struct BlockEditor {
    tree_manager: Arc<RefCell<TreeManager>>,
    selection: TextRange,
    mouse_pressed: bool,
    timer_id: TimerToken,
    cursor_visible: bool,
    text_drawer: TextDrawer,
    text_changed: bool,
    blocks: Vec<block_drawer::Block>,
    padding: Vec<f64>,
    diagnostic_popup: WidgetPod<EditorModel, DiagnosticPopup>,
}

#[derive(Clone, Data, Lens)]
pub struct EditorModel {
    pub source: String,
    #[data(eq)]
    pub diagnostics: Vec<Diagnostic>,
    #[data(eq)]
    pub diagnostic_selection: Option<u64>,
}

impl BlockEditor {
    pub fn new() -> Self {
        BlockEditor {
            tree_manager: Arc::new(RefCell::new(TreeManager::new(""))),
            selection: TextRange::ZERO,
            mouse_pressed: false,
            timer_id: TimerToken::INVALID,
            cursor_visible: true,
            text_drawer: TextDrawer::new(),
            text_changed: true,
            blocks: vec![],
            padding: vec![],
            diagnostic_popup: WidgetPod::new(DiagnosticPopup::new()),
        }
    }
}

/// the number of characters in line of source
fn line_len(row: usize, source: &str) -> usize {
    source.lines().nth(row).unwrap_or("").chars().count()
}

fn line_count(source: &str) -> usize {
    // add one if the last line is a newline (because the lines method does not include that)
    source.lines().count() + if source.ends_with('\n') { 1 } else { 0 }
}
