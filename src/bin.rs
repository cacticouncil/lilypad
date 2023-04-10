mod block_editor;
mod parse;
mod theme;

use druid::widget::Scroll;
use druid::{AppLauncher, PlatformError, Widget, WindowDesc};
use std::{
    fs::File,
    io::{BufReader, Read},
};

use block_editor::{BlockEditor, EditorModel};

fn main() -> Result<(), PlatformError> {
    // get data from test file
    let source = get_test_string("test3.py");
    let data = EditorModel {
        source,
        diagnostics: vec![block_editor::diagnostics::Diagnostic::example()],
        diagnostic_selection: None,
    };
    // launch
    let main_window = WindowDesc::new(ui_builder()).title("Lilypad Editor");
    AppLauncher::with_window(main_window).launch(data)
}

fn ui_builder() -> impl Widget<EditorModel> {
    Scroll::new(BlockEditor::new()).content_must_fill(true)
}

/* -------------------------------------------------------------------------- */
fn get_test_string(name: &'static str) -> String {
    let file = File::open(format!("{}{}", "test-files/", name)).expect("test file not found");
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader
        .read_to_string(&mut contents)
        .expect("could not read file");
    contents
}

// temp shim
pub(crate) mod vscode {
    use druid::Selector;

    use crate::block_editor::{
        diagnostics::{Diagnostic, VSCodeCommand},
        text_range::TextEdit,
    };

    pub const SET_TEXT_SELECTOR: Selector<String> = Selector::new("UNUSED");
    pub const APPLY_EDIT_SELECTOR: Selector<TextEdit> = Selector::new("UNUSED");
    pub const COPY_SELECTOR: Selector<()> = Selector::new("UNUSED");
    pub const CUT_SELECTOR: Selector<()> = Selector::new("UNUSED");
    pub const PASTE_SELECTOR: Selector<String> = Selector::new("UNUSED");
    pub const DIAGNOSTICS_SELECTOR: Selector<Vec<Diagnostic>> = Selector::new("UNUSED");
    pub const QUICK_FIX_SELECTOR: Selector<Vec<VSCodeCommand>> = Selector::new("UNUSED");

    // pub fn started() {}
    pub fn edited(_: &str, _: usize, _: usize, _: usize, _: usize) {}
    pub fn set_clipboard(_: String) {}
    pub fn request_quick_fixes(_: usize, _: usize) {}
    pub fn execute_command(_: String, _: wasm_bindgen::JsValue) {}
}

pub(crate) use println as console_log;
