mod block_editor;
mod file_picker;
mod lang;
mod parse;
mod theme;

use druid::widget::{Button, Either, Flex};
use druid::{
    AppDelegate, AppLauncher, Data, Env, FileDialogOptions, Lens, Menu, MenuItem, PlatformError,
    SysMods, Widget, WidgetExt, WindowDesc, WindowId,
};
use ropey::Rope;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use block_editor::EditorModel;

#[derive(Clone, Data)]
pub struct AppModel {
    #[data(eq)]
    pub dir: Option<PathBuf>,
    pub file: Option<String>,

    pub source: Arc<Mutex<Rope>>,
    #[data(eq)]
    pub diagnostics: Vec<block_editor::diagnostics::Diagnostic>,
    #[data(eq)]
    pub diagnostic_selection: Option<u64>,
}

pub type GlobalModel = AppModel;

struct EditorLens;

impl Lens<AppModel, EditorModel> for EditorLens {
    fn with<V, F: FnOnce(&EditorModel) -> V>(&self, data: &AppModel, f: F) -> V {
        f(&EditorModel {
            source: data.source.clone(),
            diagnostics: data.diagnostics.clone(),
            diagnostic_selection: data.diagnostic_selection,
        })
    }

    fn with_mut<V, F: FnOnce(&mut EditorModel) -> V>(&self, data: &mut AppModel, f: F) -> V {
        let mut editor_model = EditorModel {
            source: data.source.clone(),
            diagnostics: data.diagnostics.clone(),
            diagnostic_selection: data.diagnostic_selection,
        };
        let val = f(&mut editor_model);
        data.source = editor_model.source;
        data.diagnostics = editor_model.diagnostics;
        data.diagnostic_selection = editor_model.diagnostic_selection;
        val
    }
}

fn main() -> Result<(), PlatformError> {
    // SF Mono on macOS, Roboto Mono elsewhere
    if cfg!(target_os = "macos") {
        block_editor::configure_font("SF Mono".to_string(), 14.0);
    } else {
        block_editor::configure_font("Roboto Mono".to_string(), 14.0);
    }

    let data = AppModel {
        dir: None,
        file: None,
        source: Arc::new(Mutex::new(Rope::new())),
        diagnostics: vec![],
        diagnostic_selection: None,
    };
    // launch
    let main_window = WindowDesc::new(app_widget())
        .title("Lilypad Editor")
        .menu(make_menu);
    AppLauncher::with_window(main_window)
        .delegate(LilypadAppDelegate {})
        .launch(data)
}

fn app_widget() -> impl Widget<AppModel> {
    let editor = block_editor::widget("").lens(EditorLens).expand();

    let dir_picker = Button::new("Choose directory").on_click(|ctx, _data, _env| {
        let options = FileDialogOptions::new().select_directories();
        ctx.submit_command(druid::commands::SHOW_OPEN_PANEL.with(options))
    });

    Either::new(
        |data, _env| data.dir.is_some(),
        Flex::row()
            .with_child(file_picker::widget())
            .with_flex_child(editor, 1.0)
            .must_fill_main_axis(true),
        dir_picker,
    )
}

fn make_menu(_window: Option<WindowId>, _data: &AppModel, _env: &Env) -> Menu<AppModel> {
    use druid::platform_menus::*;

    let open_folder = MenuItem::new("Open folder…")
        .command(
            druid::commands::SHOW_OPEN_PANEL.with(FileDialogOptions::new().select_directories()),
        )
        .hotkey(SysMods::Cmd, "o"); // SysMods::Cmd is command on mac, control otherwise

    let file_menu = Menu::new("File")
        .entry(mac::file::new_file().enabled_if(|data: &AppModel, _| data.dir.is_some()))
        .entry(open_folder)
        .separator()
        .entry(mac::file::save().enabled_if(|data: &AppModel, _| data.file.is_some()));

    let edit_menu = Menu::new("Edit")
        .entry(common::undo())
        .entry(common::redo())
        .separator()
        .entry(common::cut())
        .entry(common::copy())
        .entry(common::paste());

    // only macOS has an applications menu
    let mut menu = Menu::empty();
    if cfg!(target_os = "macos") {
        menu = menu.entry(mac::application::default())
    }

    menu.entry(file_menu).entry(edit_menu)
}

struct LilypadAppDelegate;

impl AppDelegate<AppModel> for LilypadAppDelegate {
    fn command(
        &mut self,
        _ctx: &mut druid::DelegateCtx,
        _target: druid::Target,
        cmd: &druid::Command,
        data: &mut AppModel,
        _env: &druid::Env,
    ) -> druid::Handled {
        if let Some(dir) = cmd.get(druid::commands::OPEN_FILE) {
            data.dir = Some(dir.path.clone());
        }
        druid::Handled::No
    }
}

// temp shim
pub(crate) mod vscode {
    use druid::Selector;

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

    // pub fn started() {}
    pub fn edited(_: &str, _: usize, _: usize, _: usize, _: usize) {}
    pub fn set_clipboard(_: String) {}
    pub fn request_quick_fixes(_: usize, _: usize) {}
    pub fn request_completions(_: usize, _: usize) {}
    pub fn execute_command(_: String, _: wasm_bindgen::JsValue) {}
    pub fn execute_workspace_edit(_: wasm_bindgen::JsValue) {}
}

pub(crate) use println as console_log;
