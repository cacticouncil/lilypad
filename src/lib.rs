mod block_editor;
mod lang;
mod parse;
mod theme;

use druid::{AppLauncher, ExtEventSink, PlatformError, Target, WindowDesc};
use std::sync::{Arc, Mutex, OnceLock};
use wasm_bindgen::prelude::*;

use block_editor::{
    completion::VSCodeCompletionItem,
    diagnostics::{Diagnostic, VSCodeCodeAction},
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
pub fn run_editor(file_name: String, font_name: String, font_size: f64) {
    // This hook is necessary to get panic messages in the console
    console_error_panic_hook::set_once();
    main(file_name, font_name, font_size).expect("could not launch")
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
        sink.submit_command(vscode::APPLY_VSCODE_EDIT_SELECTOR, edits, Target::Global)
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
        sink.submit_command(
            vscode::SET_DIAGNOSTICS_SELECTOR,
            diagnostics,
            Target::Global,
        )
        .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

#[wasm_bindgen]
pub fn set_quick_fixes(json: JsValue) {
    let fixes: Vec<VSCodeCodeAction> =
        serde_wasm_bindgen::from_value(json).expect("Could not deserialize quick fixes");

    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(vscode::SET_QUICK_FIX_SELECTOR, fixes, Target::Global)
            .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

#[wasm_bindgen]
pub fn set_completions(json: JsValue) {
    let fixes: Vec<VSCodeCompletionItem> =
        serde_wasm_bindgen::from_value(json).expect("Could not deserialize completions");

    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(vscode::SET_COMPLETIONS_SELECTOR, fixes, Target::Global)
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
        completion::VSCodeCompletionItem,
        diagnostics::{Diagnostic, VSCodeCodeAction},
        text_range::TextEdit,
    };

    pub const SET_TEXT_SELECTOR: Selector<String> = Selector::new("set_text");
    pub const APPLY_VSCODE_EDIT_SELECTOR: Selector<TextEdit> = Selector::new("apply_vscode_edit");
    pub const PASTE_SELECTOR: Selector<String> = Selector::new("paste");
    pub const SET_DIAGNOSTICS_SELECTOR: Selector<Vec<Diagnostic>> =
        Selector::new("set_diagnostics");
    pub const SET_QUICK_FIX_SELECTOR: Selector<Vec<VSCodeCodeAction>> =
        Selector::new("set_quick_fix");
    pub const SET_COMPLETIONS_SELECTOR: Selector<Vec<VSCodeCompletionItem>> =
        Selector::new("set_completions");

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
        #[wasm_bindgen(js_name = requestCompletions)]
        pub fn request_completions(line: usize, col: usize);
        #[wasm_bindgen(js_name = executeCommand)]
        pub fn execute_command(command: String, args: JsValue);
        #[wasm_bindgen(js_name = executeWorkspaceEdit)]
        pub fn execute_workspace_edit(edit: JsValue);
    }
}

/* ----- Interface ----- */

static EVENT_SINK: OnceLock<Arc<ExtEventSink>> = OnceLock::new();

pub type GlobalModel = EditorModel;

fn main(file_name: String, font_name: String, font_size: f64) -> Result<(), PlatformError> {
    block_editor::configure_font(font_name, font_size);

    // start with empty string
    let data = EditorModel {
        source: Arc::new(Mutex::new(ropey::Rope::new())),
        diagnostics: vec![],
        diagnostic_selection: None,
    };

    // create main window
    let main_window = WindowDesc::new(block_editor::widget(&file_name)).title("Lilypad Editor");
    let launcher = AppLauncher::with_window(main_window);

    // get event sink for launcher
    let _ = EVENT_SINK.set(Arc::new(launcher.get_external_handle()));

    vscode::started();

    // start app
    launcher.launch(data)
}
