use druid::{
    piet::{PietText, TextLayout},
    Event, LifeCycle, MouseButton, Point, RenderContext, Size, Widget,
};
use std::collections::HashMap;
use std::sync::Arc;

use super::loose_block::LooseBlock;
use crate::block_editor::{commands, DragSession, EditorModel};
use crate::lang::Snippet;
use crate::vscode;
use crate::{
    lang::{lang_for_file, LanguageConfig},
    theme,
    util::make_label_text_layout,
};

pub struct BlockPalette {
    shown: bool,
    lang: &'static LanguageConfig,
    items: Vec<PaletteItem>,
}

struct PaletteItem {
    id: &'static str,
    block: LooseBlock,
}

impl PaletteItem {
    fn new(snippet: &Snippet, lang: &'static LanguageConfig, piet_text: &mut PietText) -> Self {
        Self {
            id: snippet.id,
            block: LooseBlock::make_from_text(snippet.source, lang, 10.0, piet_text),
        }
    }
}

impl BlockPalette {
    pub fn new(lang: &'static LanguageConfig) -> Self {
        Self {
            shown: true,
            lang,
            items: vec![],
        }
    }

    pub fn populate(&mut self, piet_text: &mut PietText) {
        self.items = self
            .lang
            .palette
            .iter()
            .map(|snippet| PaletteItem::new(snippet, self.lang, piet_text))
            .collect();
    }

    pub fn update_language(&mut self, lang: &'static LanguageConfig, piet_text: &mut PietText) {
        self.lang = lang;
        self.populate(piet_text);
    }
}

const H_PADDING: f64 = 10.0;
const V_PADDING: f64 = 8.0;
const HEADING_HEIGHT: f64 = 30.0;

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
                    // toggle shown if clicked on arrow
                    // sloppy hit box but close enough
                    if mouse.pos.y <= HEADING_HEIGHT && mouse.pos.x >= ctx.size().width - 30.0 {
                        self.shown = !self.shown;
                        ctx.request_layout();
                        ctx.request_paint();
                        return;
                    }

                    // start dragging the selected item
                    if self.shown {
                        let mut y = HEADING_HEIGHT;
                        for item in self.items.iter() {
                            if mouse.pos.y >= y && mouse.pos.y <= y + item.block.size().height {
                                vscode::log_event(
                                    "palette-blog-drag",
                                    HashMap::from([("type", item.id), ("lang", self.lang.name)]),
                                );

                                data.drag_block = Some(Arc::new(DragSession {
                                    text: item.block.text().to_string(),
                                    offset: Point::new(mouse.pos.x - H_PADDING, mouse.pos.y - y),
                                }));
                                break;
                            }

                            y += item.block.size().height + V_PADDING;
                        }
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
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        _data: &EditorModel,
        _env: &druid::Env,
    ) -> druid::Size {
        let size = if self.shown {
            let mut size = Size::ZERO;
            for item in &self.items {
                size.width = f64::max(size.width, item.block.size().width);
                size.height += item.block.size().height + V_PADDING;
            }
            size
        } else {
            let button = make_label_text_layout("⇤", ctx.text());
            Size::new(button.size().width + (H_PADDING * 2.0), 30.0)
        };

        bc.constrain(size)
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &EditorModel, _env: &druid::Env) {
        // set background color
        let bg_rect = ctx.size().to_rect();
        ctx.fill(bg_rect, &theme::POPUP_BACKGROUND);

        if self.shown {
            let mut y = 0.0;

            // top label
            let heading = make_label_text_layout("Palette:", ctx.text());
            ctx.draw_text(&heading, Point::new(H_PADDING, V_PADDING));

            let close_btn = make_label_text_layout("⇥", ctx.text());
            ctx.draw_text(
                &close_btn,
                Point::new(
                    bg_rect.width() - close_btn.size().width - H_PADDING,
                    V_PADDING,
                ),
            );

            y += HEADING_HEIGHT;

            // draw palettes
            for item in &self.items {
                let offset = Point::new(H_PADDING, y);
                item.block
                    .draw(offset, ctx.size().width - H_PADDING, data.block_theme, ctx);
                y += item.block.size().height + V_PADDING;
            }
        } else {
            let open_btn = make_label_text_layout("⇤", ctx.text());
            ctx.draw_text(
                &open_btn,
                Point::new(
                    bg_rect.width() - open_btn.size().width - H_PADDING,
                    V_PADDING,
                ),
            );
        }
    }
}
