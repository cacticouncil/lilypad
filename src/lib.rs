mod block_editor;
mod parse;

use druid::widget::Scroll;
use druid::{AppLauncher, FontDescriptor, FontFamily, Key, PlatformError, Widget, WindowDesc};

use block_editor::{BlockEditor, EditorModel};

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn wasm_main() {
    // This hook is necessary to get panic messages in the console
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    main().expect("could not launch")
}

const MONO_FONT: Key<FontDescriptor> = Key::new("org.cacticouncil.lilypad.mono-font");

fn main() -> Result<(), PlatformError> {
    // get data from test file
    let source = include_str!("../test-files/test1.py").to_string();
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
    Scroll::new(BlockEditor::new()).vertical()
}
