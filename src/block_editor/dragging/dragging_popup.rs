use druid::{PaintCtx, Point, RenderContext, Widget};
use std::sync::Arc;

use super::loose_block::LooseBlock;
use crate::block_editor::EditorModel;
use crate::{lang::LanguageConfig, theme};

pub struct DraggingPopup {
    block: LooseBlock,
    mouse_pos_in_block: Point,
}

impl DraggingPopup {
    pub fn new(lang: &'static LanguageConfig) -> Self {
        Self {
            block: LooseBlock::new(lang, 40.0),
            mouse_pos_in_block: Point::ZERO,
        }
    }

    pub fn change_language(&mut self, lang: &'static LanguageConfig) {
        self.block.change_language(lang);
    }

    pub fn calc_origin(&self, mouse: Point) -> Point {
        Point {
            x: mouse.x - self.mouse_pos_in_block.x,
            y: mouse.y - self.mouse_pos_in_block.y,
        }
    }
}

impl Widget<EditorModel> for DraggingPopup {
    fn event(
        &mut self,
        _ctx: &mut druid::EventCtx,
        _event: &druid::Event,
        _data: &mut EditorModel,
        _env: &druid::Env,
    ) {
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut druid::LifeCycleCtx,
        _event: &druid::LifeCycle,
        _data: &EditorModel,
        _env: &druid::Env,
    ) {
    }

    fn update(
        &mut self,
        ctx: &mut druid::UpdateCtx,
        old_data: &EditorModel,
        data: &EditorModel,
        _env: &druid::Env,
    ) {
        if let Some(block) = &data.drag_block {
            if let Some(old_block) = &old_data.drag_block {
                if Arc::ptr_eq(block, old_block) {
                    return;
                }
            }
            self.block.set_text(&block.text, ctx.text());
            self.mouse_pos_in_block = block.offset;
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut druid::LayoutCtx,
        _bc: &druid::BoxConstraints,
        _data: &EditorModel,
        _env: &druid::Env,
    ) -> druid::Size {
        self.block.size()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &EditorModel, _env: &druid::Env) {
        // draw background (transparent so you can see where you are dropping it)
        let rect = ctx.size().to_rect();
        ctx.fill(rect, &theme::BACKGROUND.with_alpha(0.75));

        // draw content
        self.block.draw(Point::ZERO, ctx.size().width, ctx);
    }
}
