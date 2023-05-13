mod block_editor;
mod parse;
mod theme;

use druid::{AppLauncher, ExtEventSink, PlatformError, Target, WindowDesc};
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

use block_editor::{
    diagnostics::{Diagnostic, VSCodeCommand},
    text_range::TextEdit,
    EditorModel,
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (crate::log(&format_args!($($t)*).to_string()))
}

pub(crate) use console_log;

/* ----- Javascript -> WASM ----- */
#[wasm_bindgen]
pub fn run_editor() {
    // This hook is necessary to get panic messages in the console
    console_error_panic_hook::set_once();
    main().expect("could not launch")
}

#[wasm_bindgen]
pub fn set_text(text: String) {
    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(vscode::SET_TEXT_SELECTOR, text, Target::Global)
            .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

#[wasm_bindgen]
pub fn apply_edit(json: JsValue) {
    let edits: TextEdit = serde_wasm_bindgen::from_value(json).expect("Could not deserialize edit");
    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(vscode::APPLY_EDIT_SELECTOR, edits, Target::Global)
            .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

#[wasm_bindgen]
pub fn copy_selection() {
    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(druid::commands::COPY, (), Target::Global)
            .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

#[wasm_bindgen]
pub fn cut_selection() {
    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(druid::commands::CUT, (), Target::Global)
            .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

#[wasm_bindgen]
pub fn insert_text(text: String) {
    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(vscode::PASTE_SELECTOR, text, Target::Global)
            .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

#[wasm_bindgen]
pub fn new_diagnostics(json: JsValue) {
    let diagnostics: Vec<Diagnostic> =
        serde_wasm_bindgen::from_value(json).expect("Could not deserialize diagnostics");

    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(vscode::DIAGNOSTICS_SELECTOR, diagnostics, Target::Global)
            .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

#[wasm_bindgen]
pub fn set_quick_fixes(json: JsValue) {
    let fixes: Vec<VSCodeCommand> =
        serde_wasm_bindgen::from_value(json).expect("Could not deserialize quick fixes");

    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(vscode::QUICK_FIX_SELECTOR, fixes, Target::Global)
            .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

/* ----- WASM -> Javascript ----- */
pub mod vscode {
    use druid::Selector;
    use wasm_bindgen::prelude::*;

    use crate::block_editor::{
        diagnostics::{Diagnostic, VSCodeCommand},
        text_range::TextEdit,
    };

    pub const SET_TEXT_SELECTOR: Selector<String> = Selector::new("set_text");
    pub const APPLY_EDIT_SELECTOR: Selector<TextEdit> = Selector::new("apply_edit");
    pub const PASTE_SELECTOR: Selector<String> = Selector::new("paste");
    pub const DIAGNOSTICS_SELECTOR: Selector<Vec<Diagnostic>> = Selector::new("diagnostics");
    pub const QUICK_FIX_SELECTOR: Selector<Vec<VSCodeCommand>> = Selector::new("quick_fix");

    #[wasm_bindgen(raw_module = "./run.js")]
    extern "C" {
        pub fn started();
        pub fn edited(
            new_text: &str,
            start_line: usize,
            start_col: usize,
            end_line: usize,
            end_col: usize,
        );
        #[wasm_bindgen(js_name = setClipboard)]
        pub fn set_clipboard(text: String);
        #[wasm_bindgen(js_name = requestQuickFixes)]
        pub fn request_quick_fixes(line: usize, col: usize);
        #[wasm_bindgen(js_name = executeCommand)]
        pub fn execute_command(command: String, args: JsValue);
    }
}

/* ----- Interface ----- */

static EVENT_SINK: OnceCell<Arc<ExtEventSink>> = OnceCell::new();

pub type GlobalModel = EditorModel;

fn main() -> Result<(), PlatformError> {
    // start with empty string
    let data = EditorModel {
        source: Arc::new(Mutex::new(ropey::Rope::new())),
        diagnostics: vec![],
        diagnostic_selection: None,
    };

    // create main window
    let main_window = WindowDesc::new(block_editor::widget()).title("Lilypad Editor");
    let launcher = AppLauncher::with_window(main_window);

    // get event sink for launcher
    let _ = EVENT_SINK.set(Arc::new(launcher.get_external_handle()));

    vscode::started();

    // start app
    launcher.launch(data)
}
