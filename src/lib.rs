mod block_editor;
mod lang;
mod lsp;
mod parse;
mod theme;
mod util;

use druid::{AppLauncher, Application, ExtEventSink, PlatformError, Target, WindowDesc};
use std::{
    panic::{self, PanicInfo},
    sync::{Arc, Mutex, OnceLock},
};
use wasm_bindgen::prelude::*;

use block_editor::{commands, text_editor::TextEdit, EditorModel};
use lsp::{
    completion::VSCodeCompletionItem,
    diagnostics::{Diagnostic, VSCodeCodeAction},
};

// provide rust implementation of stdlib functions to our C grammars if on wasm
#[cfg(target_arch = "wasm32")]
pub mod c_shim;

/* ----- Javascript -> WASM ----- */
#[wasm_bindgen]
pub fn run_editor(file_name: String, font_name: String, font_size: f64) {
    // send panic messages to console + telemetry
    panic::set_hook(Box::new(panic_hook));
    main(file_name, font_name, font_size).expect("could not launch")
}

#[wasm_bindgen]
pub fn set_text(text: String) {
    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(commands::SET_TEXT, text, Target::Global)
            .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

#[wasm_bindgen]
pub fn set_file(file_name: String) {
    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(commands::SET_FILE_NAME, file_name, Target::Global)
            .unwrap();
        sink.submit_command(commands::SET_TEXT, "".to_string(), Target::Global)
            .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

#[wasm_bindgen]
pub fn apply_edit(json: JsValue) {
    #[derive(serde::Deserialize)]
    struct VSCodeEdit {
        text: String,
        range: block_editor::text_range::TextRange,
    }
    let edit: VSCodeEdit =
        serde_wasm_bindgen::from_value(json).expect("Could not deserialize edit");
    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(
            commands::APPLY_EDIT,
            TextEdit::new_from_vscode(std::borrow::Cow::Owned(edit.text), edit.range),
            Target::Global,
        )
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
        sink.submit_command(commands::PASTE, text, Target::Global)
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
        sink.submit_command(commands::SET_DIAGNOSTICS, diagnostics, Target::Global)
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
        sink.submit_command(commands::SET_QUICK_FIX, fixes, Target::Global)
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
        sink.submit_command(commands::SET_COMPLETIONS, fixes, Target::Global)
            .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

/* ----- WASM -> Javascript ----- */
pub mod vscode {
    use std::collections::HashMap;
    use wasm_bindgen::prelude::*;

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

        #[wasm_bindgen(js_name = telemetryEvent)]
        fn telemetry_event(cat: String, info: JsValue);

        #[wasm_bindgen(js_name = telemetryCrash)]
        pub fn telemetry_crash(msg: String);
    }

    pub fn log_event(cat: &'static str, info: HashMap<&'static str, &str>) {
        telemetry_event(
            cat.to_string(),
            serde_wasm_bindgen::to_value(&info).unwrap(),
        );
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (crate::log(&format_args!($($t)*).to_string()))
}

pub(crate) use console_log;

fn panic_hook(info: &PanicInfo) {
    console_error_panic_hook::hook(info);

    vscode::telemetry_crash(
        info.to_string()
            .replace("/", ">") // workaround to avoid vscode redacting a non-sensitive path
            .replace("\\", ">"),
    );

    // don't continue in a broken state
    Application::global().quit();
}

/* ----- Interface ----- */

static EVENT_SINK: OnceLock<Arc<ExtEventSink>> = OnceLock::new();

pub type GlobalModel = EditorModel;

fn main(file_name: String, font_name: String, font_size: f64) -> Result<(), PlatformError> {
    block_editor::configure_font(font_name, font_size);

    // start with empty string
    let data = EditorModel {
        source: Arc::new(Mutex::new(ropey::Rope::new())),
        diagnostics: Arc::new(vec![]),
        diagnostic_selection: None,
        drag_block: None,
    };

    // create main window
    let main_window = WindowDesc::new(block_editor::widget(&file_name)).title("Lilypad Editor");
    let launcher = AppLauncher::with_window(main_window);

    // get event sink for launcher
    let _ = EVENT_SINK.set(Arc::new(launcher.get_external_handle()));

    // start app
    launcher.launch(data)
}
