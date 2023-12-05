use druid::{piet::PietText, Event, LifeCycle, MouseButton, Point, RenderContext, Size, Widget};
use std::sync::Arc;

use super::loose_block::LooseBlock;
use crate::block_editor::{commands, DragSession, EditorModel};
use crate::{
    lang::{lang_for_file, LanguageConfig},
    theme,
    util::make_label_text_layout,
};

pub struct BlockPalette {
    lang: &'static LanguageConfig,
    items: Vec<LooseBlock>,
}

impl BlockPalette {
    pub fn new(lang: &'static LanguageConfig) -> Self {
        Self {
            lang,
            items: vec![],
        }
    }

    pub fn populate(&mut self, piet_text: &mut PietText) {
        self.items = self
            .lang
            .palette
            .iter()
            .map(|text| LooseBlock::make_from_text(text, self.lang, 10.0, piet_text))
            .collect();
    }

    pub fn update_language(&mut self, lang: &'static LanguageConfig, piet_text: &mut PietText) {
        self.lang = lang;
        self.populate(piet_text);
    }
}

const H_PADDING: f64 = 5.0;
const V_PADDING: f64 = 8.0;
const HEADING_PADDING: f64 = 25.0;

impl Widget<EditorModel> for BlockPalette {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut EditorModel,
        _env: &druid::Env,
    ) {
        match event {
            Event::MouseDown(mouse) => {
                if mouse.button == MouseButton::Left {
                    // start dragging the selected item
                    let mut y = HEADING_PADDING;
                    for item in self.items.iter() {
                        if mouse.pos.y >= y && mouse.pos.y <= y + item.size().height {
                            data.drag_block = Some(Arc::new(DragSession {
                                text: item.text().to_string(),
                                offset: Point::new(mouse.pos.x - H_PADDING, mouse.pos.y - y),
                            }));
                            break;
                        }

                        y += item.size().height + V_PADDING;
                    }
                }
            }

            Event::MouseUp(mouse) if mouse.button == MouseButton::Left => {
                // cancel drag if block is dropped on the palette
                if data.drag_block.is_some() {
                    data.drag_block = None;
                    ctx.submit_command(commands::DRAG_CANCELLED);
                    ctx.request_paint();
                }
            }

            Event::Command(command) => {
                if let Some(file_name) = command.get(commands::SET_FILE_NAME) {
                    let new_lang = lang_for_file(file_name);
                    if self.lang.name != new_lang.name {
                        self.update_language(new_lang, ctx.text());
                    }
                    ctx.request_layout();
                }
            }
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        _data: &EditorModel,
        _env: &druid::Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            self.populate(ctx.text());
        }
    }

    fn update(
        &mut self,
        _ctx: &mut druid::UpdateCtx,
        _old_data: &EditorModel,
        _data: &EditorModel,
        _env: &druid::Env,
    ) {
    }

    fn layout(
        &mut self,
        _ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        _data: &EditorModel,
        _env: &druid::Env,
    ) -> druid::Size {
        let mut size = Size::ZERO;

        for item in &self.items {
            size.width = f64::max(size.width, item.size().width);
            size.height += item.size().height + V_PADDING;
        }

        bc.constrain(size)
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, _data: &EditorModel, _env: &druid::Env) {
        // set background color
        let bg_rect = ctx.size().to_rect();
        ctx.fill(bg_rect, &theme::POPUP_BACKGROUND);

        let mut y = 0.0;

        // top label
        let heading = make_label_text_layout("Palette:", ctx);
        ctx.draw_text(&heading, Point::new(5.0, 5.0));
        y += HEADING_PADDING;

        // draw palettes
        for item in &self.items {
            let offset = Point::new(H_PADDING, y);
            item.draw(offset, ctx.size().width - H_PADDING, ctx);
            y += item.size().height + V_PADDING;
        }
    }
}
