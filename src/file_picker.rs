use crate::{theme, AppModel};
use druid::{
    piet::{PietTextLayout, Text, TextLayout, TextLayoutBuilder},
    widget::Scroll,
    Event, FontFamily, LifeCycle, MouseButton, PaintCtx, Point, Rect, RenderContext, Size, Widget,
    WidgetExt,
};

pub fn widget() -> impl Widget<AppModel> {
    Scroll::new(FilePicker { files: vec![] })
        .vertical()
        .expand_height()
        .background(theme::POPUP_BACKGROUND)
}

struct FilePicker {
    files: Vec<String>,
}

const ROW_HEIGHT: f64 = 25.0;
const ROW_WIDTH: f64 = 150.0;

impl Widget<AppModel> for FilePicker {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut AppModel,
        _env: &druid::Env,
    ) {
        if let Event::MouseDown(mouse) = event {
            ctx.request_focus();

            if mouse.button == MouseButton::Left {
                let file_num = (mouse.pos.y / ROW_HEIGHT) as usize;
                if file_num < self.files.len() {
                    data.file = Some(self.files[file_num].clone());
                }
                ctx.set_handled();
            }
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        data: &AppModel,
        _env: &druid::Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            if let Some(dir) = &data.dir {
                self.files = std::fs::read_dir(dir)
                    .unwrap()
                    .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
                    .collect::<Vec<_>>();
            }
        }
    }

    fn update(
        &mut self,
        ctx: &mut druid::UpdateCtx,
        old_data: &AppModel,
        data: &AppModel,
        _env: &druid::Env,
    ) {
        if old_data.dir != data.dir {
            if let Some(dir) = &data.dir {
                self.files = std::fs::read_dir(dir)
                    .unwrap()
                    .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
                    .collect::<Vec<_>>();
                self.files.sort_unstable();
            } else {
                self.files.clear();
            }
        }

        // update source if file changed
        if old_data.file != data.file {
            if let Some(file) = &data.file {
                let mut file_path = data.dir.clone().unwrap();
                file_path.push(file);

                let file_contents = std::fs::read_to_string(&file_path)
                    .unwrap_or_else(|_| "# could not read file".to_string());
                let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
                ctx.submit_command(crate::block_editor::SET_FILE_NAME_SELECTOR.with(file_name));
                ctx.submit_command(crate::vscode::SET_TEXT_SELECTOR.with(file_contents));
            }
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        _data: &AppModel,
        _env: &druid::Env,
    ) -> druid::Size {
        let height = self.files.len() as f64 * ROW_HEIGHT;
        let desired = Size {
            width: ROW_WIDTH,
            height,
        };
        bc.constrain(desired)
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &AppModel, _env: &druid::Env) {
        if ctx.has_focus() {
            let outline = Rect::from_origin_size(Point::ORIGIN, ctx.size());
            ctx.stroke(outline, &theme::SELECTION, 2.0);
        }

        for (num, file) in self.files.iter().enumerate() {
            let layout = make_text_layout(file, ctx);

            let pos = Point::new(0.0, ROW_HEIGHT * num as f64);

            if Some(file) == data.file.as_ref() {
                // highlight background
                let rect = Rect::from_origin_size(pos, Size::new(ROW_WIDTH, ROW_HEIGHT));
                ctx.fill(rect, &theme::SELECTION);
            }

            let text_pos = Point::new(10.0, pos.y + ((ROW_HEIGHT - layout.size().height) / 2.0));

            ctx.draw_text(&layout, text_pos);
        }
    }
}

fn make_text_layout(text: &str, ctx: &mut PaintCtx) -> PietTextLayout {
    let font_family = if cfg!(target_os = "macos") {
        FontFamily::new_unchecked("SF Pro Text")
    } else {
        FontFamily::new_unchecked("San Serif")
    };

    ctx.text()
        .new_text_layout(text.to_string())
        .font(font_family, 15.0)
        .text_color(theme::INTERFACE_TEXT)
        .build()
        .unwrap()
}
