use egui::{Event, Key};
use egui_inbox::{UiInbox, UiInboxSender};
use log::error;
use std::panic::{self, PanicHookInfo};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::block_editor::{
    text_editor::{StackFrameLines, TextEdit},
    text_range::TextRange,
    ExternalCommand,
};
use crate::lsp::{
    completion::VSCodeCompletionItem,
    diagnostics::{Diagnostic, VSCodeCodeAction},
};
use crate::theme::blocks_theme::BlocksTheme;
use crate::vscode;
use crate::LilypadWeb;

fn panic_hook(info: &PanicHookInfo) {
    console_error_panic_hook::hook(info);

    vscode::telemetry_crash(info.to_string().replace(['/', '\\'], ">"));
}

#[wasm_bindgen]
pub struct LilypadWebHandle {
    runner: eframe::WebRunner,
    command_sender: Option<UiInboxSender<ExternalCommand>>,
}

#[wasm_bindgen]
impl LilypadWebHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        eframe::WebLogger::init(log::LevelFilter::Debug).ok();

        panic::set_hook(Box::new(panic_hook));

        Self {
            runner: eframe::WebRunner::new(),
            command_sender: None,
        }
    }

    #[wasm_bindgen]
    pub async fn start(
        &mut self,
        canvas_id: &str,
        file_name: String,
        font_name: String,
        font_size: f32,
        blocks_theme: String,
    ) -> Result<(), wasm_bindgen::JsValue> {
        let document = web_sys::window()
            .ok_or("No window found")?
            .document()
            .ok_or("No document found")?;

        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or(format!("No element with id '{}' found", canvas_id))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| format!("Element with id '{}' is not a canvas", canvas_id))?;

        let options = eframe::WebOptions {
            should_propagate_event: Box::new(|event| Self::should_propagate_event(event)),
            ..Default::default()
        };

        let inbox = UiInbox::new();
        self.command_sender = Some(inbox.sender());
        self.runner
            .start(
                canvas,
                options,
                Box::new(move |cc| {
                    Ok(Box::new(LilypadWeb::new(
                        cc,
                        file_name,
                        blocks_theme,
                        font_name,
                        font_size,
                        inbox,
                    )))
                }),
            )
            .await
    }

    fn should_propagate_event(event: &egui::Event) -> bool {
        if let Event::Key {
            key,
            physical_key: _,
            pressed: _,
            repeat: _,
            modifiers,
        } = event
        {
            // pass through hotkeys (other than undo/redo/find) and function keys
            if modifiers.any() && !modifiers.shift_only() {
                matches!(key, Key::Z | Key::Y | Key::F) == false
            } else {
                matches!(
                    key,
                    Key::F1
                        | Key::F2
                        | Key::F3
                        | Key::F4
                        | Key::F5
                        | Key::F6
                        | Key::F7
                        | Key::F8
                        | Key::F9
                        | Key::F10
                        | Key::F11
                        | Key::F12
                )
            }
        } else {
            false
        }
    }

    #[wasm_bindgen]
    pub fn set_text(&self, text: String) {
        if let Some(sender) = &self.command_sender {
            if sender.send(ExternalCommand::SetText(text)).is_err() {
                error!("Failed to send command");
            }
        } else {
            error!("No command sender");
        }
    }

    #[wasm_bindgen]
    pub fn set_file(&self, file_name: String) {
        if let Some(sender) = &self.command_sender {
            if sender
                .send(ExternalCommand::SetFileName(file_name))
                .is_err()
            {
                error!("Failed to send command");
            }
            if sender
                .send(ExternalCommand::SetText("".to_string()))
                .is_err()
            {
                error!("Failed to send command");
            }
        } else {
            error!("No command sender");
        }
    }

    #[wasm_bindgen]
    pub fn set_font(&self, font_name: String, font_size: f32) {
        if let Some(sender) = &self.command_sender {
            if sender
                .send(ExternalCommand::SetFont(font_name, font_size))
                .is_err()
            {
                error!("Failed to send command");
            }
        } else {
            error!("No command sender");
        }
    }

    #[wasm_bindgen]
    pub fn apply_edit(&self, json: JsValue) {
        #[derive(serde::Deserialize)]
        struct VSCodeEdit {
            text: String,
            range: TextRange,
        }
        let vscode_edit: VSCodeEdit =
            serde_wasm_bindgen::from_value(json).expect("Could not deserialize edit");
        let edit =
            TextEdit::new_from_vscode(std::borrow::Cow::Owned(vscode_edit.text), vscode_edit.range);
        if let Some(sender) = &self.command_sender {
            if sender.send(ExternalCommand::ApplyEdit(edit)).is_err() {
                error!("Failed to send command");
            }
        } else {
            error!("No command sender");
        }
    }

    #[wasm_bindgen]
    pub fn set_blocks_theme(&self, theme: String) {
        if let Some(sender) = &self.command_sender {
            if sender
                .send(ExternalCommand::SetBlocksTheme(BlocksTheme::for_str(
                    &theme,
                )))
                .is_err()
            {
                error!("Failed to send command");
            }
        } else {
            error!("No command sender");
        }
    }

    #[wasm_bindgen]
    pub fn set_diagnostics(&self, json: JsValue) {
        let mut diagnostics: Vec<Diagnostic> =
            serde_wasm_bindgen::from_value(json).expect("Could not deserialize diagnostics");

        // set the id of each diagnostic to it's index
        for (i, diag) in diagnostics.iter_mut().enumerate() {
            diag.id = i;
        }

        if let Some(sender) = &self.command_sender {
            if sender
                .send(ExternalCommand::SetDiagnostics(diagnostics))
                .is_err()
            {
                error!("Failed to send command");
            }
        } else {
            error!("No command sender");
        }
    }

    #[wasm_bindgen]
    pub fn set_quick_fixes(&self, id: usize, fixes_json: JsValue) {
        let fixes: Vec<VSCodeCodeAction> =
            serde_wasm_bindgen::from_value(fixes_json).expect("Could not deserialize quick fixes");
        if let Some(sender) = &self.command_sender {
            if sender
                .send(ExternalCommand::SetQuickFix(id, fixes))
                .is_err()
            {
                error!("Failed to send command");
            }
        } else {
            error!("No command sender");
        }
    }

    #[wasm_bindgen]
    pub fn set_completions(&self, json: JsValue) {
        let completions: Vec<VSCodeCompletionItem> =
            serde_wasm_bindgen::from_value(json).expect("Could not deserialize completions");
        if let Some(sender) = &self.command_sender {
            if sender
                .send(ExternalCommand::SetCompletions(completions))
                .is_err()
            {
                error!("Failed to send command");
            }
        } else {
            error!("No command sender");
        }
    }

    #[wasm_bindgen]
    pub fn set_breakpoints(&self, json: JsValue) {
        let breakpoints: Vec<usize> =
            serde_wasm_bindgen::from_value(json).expect("Could not deserialize breakpoints");
        if let Some(sender) = &self.command_sender {
            if sender
                .send(ExternalCommand::SetBreakpoints(breakpoints))
                .is_err()
            {
                error!("Failed to send command");
            }
        } else {
            error!("No command sender");
        }
    }

    #[wasm_bindgen]
    pub fn set_stack_frame(&self, selected: Option<usize>, deepest: Option<usize>) {
        let lines = StackFrameLines { selected, deepest };
        if let Some(sender) = &self.command_sender {
            if sender.send(ExternalCommand::SetStackFrame(lines)).is_err() {
                error!("Failed to send command");
            }
        } else {
            error!("No command sender");
        }
    }

    #[wasm_bindgen]
    pub fn undo(&self) {
        if let Some(sender) = &self.command_sender {
            if sender.send(ExternalCommand::Undo).is_err() {
                error!("Failed to send command");
            }
        } else {
            error!("No command sender");
        }
    }

    #[wasm_bindgen]
    pub fn redo(&self) {
        if let Some(sender) = &self.command_sender {
            if sender.send(ExternalCommand::Redo).is_err() {
                error!("Failed to send command");
            }
        } else {
            error!("No command sender");
        }
    }
}
