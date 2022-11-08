mod block_editor;
mod parse;

//use druid::widget::{EnvScope, TextBox};
use druid::widget::Scroll;
use druid::{AppLauncher, FontDescriptor, FontFamily, Key, PlatformError, Widget, WindowDesc};
use std::{
    fs::File,
    io::{BufReader, Read},
};

use block_editor::{BlockEditor, EditorModel};

const MONO_FONT: Key<FontDescriptor> = Key::new("org.cacticouncil.lilypad.mono-font");

fn main() -> Result<(), PlatformError> {
    // get data from test file
    let source = get_test_string("test1.py");
    let data = EditorModel { source };
    // launch
    let main_window = WindowDesc::new(ui_builder).title("Lilypad Editor");
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
    Scroll::new(BlockEditor::new()).vertical()
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
