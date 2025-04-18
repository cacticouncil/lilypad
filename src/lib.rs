#![feature(c_variadic)]

mod block_editor;
mod lang;
mod lsp;
mod theme;
mod util_widgets;
mod vscode;

#[cfg(target_arch = "wasm32")]
pub mod web_handle;

// provide rust implementation of stdlib functions to our C grammars if on wasm
#[cfg(target_arch = "wasm32")]
pub mod c_shim;

use block_editor::{BlockEditor, ExternalCommand, MonospaceFont};
use egui::Sense;
use egui_inbox::UiInbox;
use std::sync::Arc;
pub struct LilypadWeb {
    block_editor: BlockEditor,
    inbox: UiInbox<ExternalCommand>,
}

impl LilypadWeb {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        file_name: String,
        blocks_theme: String,
        font_family: String,
        font_size: f32,
        inbox: UiInbox<ExternalCommand>,
    ) -> Self {
        // Uncomment to enable debug options:

        // _cc.egui_ctx.set_style(Arc::new(egui::Style {
        //    debug: egui::style::DebugOptions {
        //       debug_on_hover: true,
        //      show_widget_hits: true,
        //      ..Default::default()
        //   },
        // ..Default::default()
        //}));
        Self {
            block_editor: BlockEditor::new(
                &file_name,
                &blocks_theme,
                MonospaceFont::new(&font_family, font_size),
            ),
            inbox,
        }
    }
}

impl eframe::App for LilypadWeb {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let available_rect = ctx.available_rect();
        egui::CentralPanel::default().show(ctx, |ui| {
            let external_commands = self.inbox.read(ui).collect::<Vec<ExternalCommand>>();

            let response = ui.allocate_rect(available_rect, Sense::hover());
            ui.add(self.block_editor.widget(&external_commands));
            response
        });
    }
}
