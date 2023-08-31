use druid::{widget::Scroll, Data, Selector, TimerToken, Widget, WidgetPod};
use ropey::Rope;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use crate::lang::{lang_for_file, LanguageConfig};
use crate::parse::TreeManager;

mod block_drawer;
pub mod completion;
pub mod diagnostics;
mod gutter_drawer;
mod highlighter;
mod ime;
mod lifecycle;
mod rope_ext;
mod selection_changing;
mod selection_drawer;
mod text_drawer;
mod text_editing;
pub mod text_range;

pub use block_drawer::BlockType;

use completion::CompletionPopup;
use diagnostics::Diagnostic;
use text_drawer::*;
use text_range::*;

use self::diagnostics::DiagnosticPopup;
use self::ime::ImeComponent;

// controls cursor blinking speed
pub const TIMER_INTERVAL: Duration = Duration::from_millis(700);

static FONT_FAMILY: OnceLock<druid::FontFamily> = OnceLock::new();
static FONT_SIZE: OnceLock<f64> = OnceLock::new();
static FONT_WIDTH: OnceLock<f64> = OnceLock::new();
static FONT_HEIGHT: OnceLock<f64> = OnceLock::new();

pub fn configure_font(name: String, size: f64) {
    let family = druid::FontFamily::new_unchecked(name);
    FONT_FAMILY.set(family).unwrap();
    FONT_SIZE.set(size).unwrap();
}

pub fn find_font_dimensions(ctx: &mut druid::LifeCycleCtx, env: &druid::Env) {
    // find the size of a single character
    let font = druid::FontDescriptor::new(FONT_FAMILY.get().unwrap().clone())
        .with_size(*FONT_SIZE.get().unwrap());
    let mut layout = druid::TextLayout::<String>::from_text("A");
    layout.set_font(font);
    layout.rebuild_if_needed(ctx.text(), env);
    let dimensions = layout.size();

    FONT_WIDTH.set(dimensions.width).unwrap();
    FONT_HEIGHT.set(dimensions.height).unwrap();
}

/// padding around edges of entire editor
const OUTER_PAD: f64 = 16.0;

/// left padding on text (to position it nicer within the blocks)
const TEXT_L_PAD: f64 = 2.0;

/// width for the line number gutter
const GUTTER_WIDTH: f64 = 30.0;

/// convenience constant for all the padding that impacts text layout
const TOTAL_TEXT_X_OFFSET: f64 = OUTER_PAD + GUTTER_WIDTH + TEXT_L_PAD;

const SHOW_ERROR_BLOCK_OUTLINES: bool = false;

const APPLY_EDIT_SELECTOR: Selector<TextEdit> = Selector::new("apply_edit");
pub const SET_FILE_NAME_SELECTOR: Selector<String> = Selector::new("set_file_name");

pub fn widget(file_name: &str) -> impl Widget<EditorModel> {
    Scroll::new(BlockEditor::new(file_name)).content_must_fill(true)
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

    /// overlay view for completions
    completion_popup: WidgetPod<EditorModel, CompletionPopup>,

    /// pairs that were inserted and should be ignored on the next input
    input_ignore_stack: Vec<&'static str>,

    /// tracking which characters had pairs inserted with them, and should take
    /// the pair down with them if they are deleted
    paired_delete_stack: Vec<bool>,

    /// the current language used by the editor
    language: &'static LanguageConfig,

    ime: ImeComponent,
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
    fn new(file_name: &str) -> Self {
        let lang = lang_for_file(file_name);
        BlockEditor {
            tree_manager: TreeManager::new(lang),
            selection: TextRange::ZERO,
            pseudo_selection: None,
            mouse_pressed: false,
            cursor_timer: TimerToken::INVALID,
            cursor_visible: true,
            text_drawer: TextDrawer::new(lang),
            text_changed: true,
            blocks: vec![],
            padding: vec![],
            diagnostic_popup: WidgetPod::new(DiagnosticPopup::new()),
            completion_popup: WidgetPod::new(CompletionPopup::new()),
            input_ignore_stack: vec![],
            paired_delete_stack: vec![],
            language: lang,
            ime: ImeComponent::default(),
        }
    }
}
