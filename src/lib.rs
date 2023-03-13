mod block_editor;
mod parse;
mod theme;

use std::sync::Arc;

use druid::widget::Scroll;
use druid::{
    AppLauncher, ExtEventSink, FontDescriptor, FontFamily, Key, PlatformError, Target, Widget,
    WidgetExt, WindowDesc,
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
pub fn update_text(text: String) {
    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(vscode::UPDATE_TEXT_SELECTOR, text, Target::Global)
            .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

#[wasm_bindgen]
pub fn copy_selection() {
    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(vscode::COPY_SELECTOR, (), Target::Global)
            .unwrap();
    } else {
        console_log!("could not get sink");
    }
}

#[wasm_bindgen]
pub fn cut_selection() {
    if let Some(sink) = EVENT_SINK.get() {
        sink.submit_command(vscode::CUT_SELECTOR, (), Target::Global)
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

/* ----- WASM -> Javascript ----- */
pub mod vscode {
    use druid::Selector;
    use wasm_bindgen::prelude::*;

    pub const UPDATE_TEXT_SELECTOR: Selector<String> = Selector::new("update_text");
    pub const COPY_SELECTOR: Selector<()> = Selector::new("copy");
    pub const CUT_SELECTOR: Selector<()> = Selector::new("cut");
    pub const PASTE_SELECTOR: Selector<String> = Selector::new("paste");

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
        ); // this is currently unused but could be used later to change the font as a setting
    });

    // get event sink for launcher
    let _ = EVENT_SINK.set(Arc::new(launcher.get_external_handle()));

    vscode::started();

    // start app
    launcher.launch(data)
}

fn ui_builder() -> impl Widget<EditorModel> {
    Scroll::new(BlockEditor::new())
        .vertical()
        .background(theme::BACKGROUND)
}
