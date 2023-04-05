mod block_editor;
mod parse;
mod theme;

use druid::widget::Scroll;
use druid::{
    AppLauncher, FontDescriptor, FontFamily, Key, PlatformError, Widget, WidgetExt, WindowDesc,
};
use std::{
    fs::File,
    io::{BufReader, Read},
};

use block_editor::{BlockEditor, EditorModel};

const MONO_FONT: Key<FontDescriptor> = Key::new("org.cacticouncil.lilypad.mono-font");

fn main() -> Result<(), PlatformError> {
    // get data from test file
    let source = get_test_string("test3.py");
    let data = EditorModel { source };
    // launch
    let main_window = WindowDesc::new(ui_builder()).title("Lilypad Editor");
    AppLauncher::with_window(main_window)
        .configure_env(|env, _state| {
            env.set(
                MONO_FONT,
                FontDescriptor::new(FontFamily::new_unchecked("Roboto Mono")).with_size(15.0),
            );
        })
        .launch(data)
}

fn ui_builder() -> impl Widget<EditorModel> {
    Scroll::new(BlockEditor::new())
        .content_must_fill(true)
        .background(theme::BACKGROUND)
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

    pub const UPDATE_TEXT_SELECTOR: Selector<String> = Selector::new("UNUSED");
    pub const COPY_SELECTOR: Selector<()> = Selector::new("UNUSED");
    pub const CUT_SELECTOR: Selector<()> = Selector::new("UNUSED");
    pub const PASTE_SELECTOR: Selector<String> = Selector::new("UNUSED");

    // pub fn started() {}
    pub fn edited(_: &str, _: usize, _: usize, _: usize, _: usize) {}
    pub fn set_clipboard(_: String) {}
}

// pub(crate) use println as console_log;
