mod block_editor;
mod parse;
mod shared;

use std::sync::Arc;

use druid::widget::Scroll;
use druid::{
    AppLauncher, ExtEventSink, FontDescriptor, FontFamily, Key, PlatformError, Target, Widget,
    WindowDesc,
};
use once_cell::sync::OnceCell;

use block_editor::{BlockEditor, EditorModel};

use wasm_bindgen::prelude::*;

extern crate wee_alloc;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

/* ----- Javascript -> WASM ----- */
#[wasm_bindgen]
pub fn run_editor() {
    // This hook is necessary to get panic messages in the console
    console_error_panic_hook::set_once();
    main().expect("could not launch")
}

#[wasm_bindgen]
pub fn update_text(text: String) {
    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(shared::UPDATE_TEXT_SELECTOR, text, Target::Global)
            .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

/* ----- WASM -> Javascript ----- */
pub mod vscode {
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
    }
}

/* ----- Interface ----- */

const MONO_FONT: Key<FontDescriptor> = Key::new("org.cacticouncil.lilypad.mono-font");

static EVENT_SINK: OnceCell<Arc<ExtEventSink>> = OnceCell::new();

fn main() -> Result<(), PlatformError> {
    // start with empty string
    let data = EditorModel {
        source: String::new(),
    };

    // create main window
    let main_window = WindowDesc::new(ui_builder()).title("Lilypad Editor");
    let launcher = AppLauncher::with_window(main_window).configure_env(|env, _state| {
        env.set(
            MONO_FONT,
            FontDescriptor::new(FontFamily::new_unchecked("Roboto Mono")).with_size(15.0),
        );
    });

    // get event sink for launcher
    EVENT_SINK.set(Arc::new(launcher.get_external_handle()));

    vscode::started();

    // start app
    launcher.launch(data)
}

fn ui_builder() -> impl Widget<EditorModel> {
    Scroll::new(BlockEditor::new()).vertical()
}
