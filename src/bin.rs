mod block_editor;
mod file_picker;
mod lang;
mod lsp;
mod parse;
mod theme;
mod util_widgets;
mod vscode;

use block_editor::{BlockEditor, MonospaceFont};
use egui::Frame;
use file_picker::FilePicker;

pub struct LilypadNative {
    file_picker: FilePicker,
    block_editor: BlockEditor,
}

impl LilypadNative {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            file_picker: FilePicker::new(),
            block_editor: BlockEditor::new(
                "untitled.py",
                "syntax_colored",
                MonospaceFont::new("SF Mono", 14.0),
            ),
        }
    }
}

impl eframe::App for LilypadNative {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut commands = vec![];

        egui::SidePanel::left("file-picker")
            .default_width(200.0)
            .resizable(false)
            .frame(Frame::none())
            .show(ctx, |ui| ui.add(self.file_picker.widget(&mut commands)));

        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                ui.add(self.block_editor.widget(&commands));
            });
    }
}

fn main() -> eframe::Result {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../app-icon.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "Lilypad",
        native_options,
        Box::new(|cc| Ok(Box::new(LilypadNative::new(cc)))),
    )
}
