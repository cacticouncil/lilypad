use egui::{Button, FontId, Rect, Response, ScrollArea, Sense, Ui, Vec2, Widget};
use rfd::FileDialog;
use std::path::PathBuf;

use crate::{block_editor::ExternalCommand, theme, util_widgets::SelectableRow};

pub struct FilePicker {
    dir: Option<PathBuf>,
    files: Vec<String>,
    selected_file: Option<usize>,
}

const ROW_HEIGHT: f32 = 25.0;

impl FilePicker {
    pub fn new() -> Self {
        Self {
            dir: None,
            files: vec![],
            selected_file: None,
        }
    }

    pub fn widget<'a>(&'a mut self, commands: &'a mut Vec<ExternalCommand>) -> impl Widget + 'a {
        move |ui: &mut Ui| -> Response {
            ScrollArea::vertical()
                .show(ui, |ui| {
                    let (id, rect) = ui.allocate_space(
                        ui.available_size()
                            .max(Vec2::new(0.0, self.files.len() as f32 * ROW_HEIGHT)),
                    );
                    let response = ui.interact(rect, id, Sense::hover());

                    ui.painter().rect_filled(rect, 0.0, theme::POPUP_BACKGROUND);

                    if self.dir.is_some() {
                        for (idx, file) in self.files.iter().enumerate() {
                            if ui
                                .put(
                                    Rect::from_min_size(
                                        rect.min + Vec2::new(0.0, idx as f32 * ROW_HEIGHT),
                                        Vec2::new(rect.width(), ROW_HEIGHT),
                                    ),
                                    SelectableRow::new(
                                        file,
                                        theme::syntax::DEFAULT,
                                        self.selected_file == Some(idx),
                                        FontId::proportional(15.0),
                                    ),
                                )
                                .clicked()
                            {
                                self.selected_file = Some(idx);
                                self.set_file(commands);
                            }
                        }
                    } else if ui
                        .put(
                            Rect::from_min_size(rect.min, Vec2::new(rect.width(), ROW_HEIGHT)),
                            Button::new("Pick Folder"),
                        )
                        .clicked()
                    {
                        if let Some(dir) = FileDialog::new().pick_folder() {
                            self.files = std::fs::read_dir(&dir)
                                .unwrap()
                                .map(|entry| {
                                    entry.unwrap().file_name().to_string_lossy().to_string()
                                })
                                .collect::<Vec<_>>();
                            self.dir = Some(dir);
                        }
                    }

                    response
                })
                .inner
        }
    }

    fn set_file(&self, commands: &mut Vec<ExternalCommand>) {
        if let (Some(dir), Some(selected_file)) = (self.dir.as_ref(), self.selected_file) {
            let file_path = dir.join(&self.files[selected_file]);

            let file_contents = std::fs::read_to_string(&file_path)
                .unwrap_or_else(|_| "# could not read file".to_string());
            let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();

            commands.push(ExternalCommand::SetFile {
                name: file_name,
                contents: file_contents,
            });
        }
    }
}
