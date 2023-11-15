use druid::widget::Flex;
use druid::{widget::Scroll, Data, Point, Widget, WidgetPod};
use druid::{Event, MouseButton, WidgetExt};
use ropey::Rope;
use std::sync::{Arc, Mutex, OnceLock};

use crate::lang::{lang_for_file, LanguageConfig};

mod block_drawer;
pub mod commands;
mod dragging;
mod highlighter;
mod rope_ext;
mod text_drawer;
mod text_editor;
pub mod text_range;

pub use block_drawer::BlockType;

use crate::lsp::diagnostics::Diagnostic;
use text_range::*;

use self::dragging::block_palette::BlockPalette;
use self::dragging::dragging_popup::DraggingPopup;
use self::text_editor::TextEditor;

static FONT_FAMILY: OnceLock<druid::FontFamily> = OnceLock::new();
static FONT_SIZE: OnceLock<f64> = OnceLock::new();
static FONT_WIDTH: OnceLock<f64> = OnceLock::new();
static FONT_HEIGHT: OnceLock<f64> = OnceLock::new();

pub fn configure_font(name: String, size: f64) {
    let family = druid::FontFamily::new_unchecked(name);
    FONT_FAMILY.set(family).unwrap();
    FONT_SIZE.set(size).unwrap();
}

pub fn find_font_dimensions(ctx: &mut druid::LifeCycleCtx, env: &druid::Env) {
    // find the size of a single character
    let font = druid::FontDescriptor::new(FONT_FAMILY.get().unwrap().clone())
        .with_size(*FONT_SIZE.get().unwrap());
    let mut layout = druid::TextLayout::<String>::from_text("A");
    layout.set_font(font);
    layout.rebuild_if_needed(ctx.text(), env);
    let dimensions = layout.size();

    FONT_WIDTH.set(dimensions.width).unwrap();
    FONT_HEIGHT.set(dimensions.height).unwrap();
}

/// padding around edges of entire editor
const OUTER_PAD: f64 = 16.0;

/// left padding on text (to position it nicer within the blocks)
const TEXT_L_PAD: f64 = 2.0;

/// width for the line number gutter
const GUTTER_WIDTH: f64 = 30.0;

/// convenience constant for all the padding that impacts text layout
const TOTAL_TEXT_X_OFFSET: f64 = OUTER_PAD + GUTTER_WIDTH + TEXT_L_PAD;

const SHOW_ERROR_BLOCK_OUTLINES: bool = false;

pub fn widget(file_name: &str) -> impl Widget<EditorModel> {
    BlockEditor::new(file_name)
}

struct BlockEditor {
    /// the current language used by the editor
    language: &'static LanguageConfig,

    /// a horizontal stack with the text editor and block palette
    content: WidgetPod<EditorModel, Flex<EditorModel>>,

    /// overlay view for dragging blocks
    dragging_popup: WidgetPod<EditorModel, DraggingPopup>,
}

pub struct DragSession {
    text: String,

    /// point within the block that it is dragged by
    offset: Point,
}

#[derive(Clone, Data)]
pub struct EditorModel {
    /// the source code to edit
    pub source: Arc<Mutex<Rope>>,

    /// diagnostics for current cursor position
    pub diagnostics: Arc<Vec<Diagnostic>>,

    /// id of diagnostic selected in the popup
    #[data(eq)]
    pub diagnostic_selection: Option<u64>,

    /// text that is currently getting dragged
    pub drag_block: Option<Arc<DragSession>>,
}

impl BlockEditor {
    fn new(file_name: &str) -> Self {
        let lang = lang_for_file(file_name);
        BlockEditor {
            language: lang,
            content: WidgetPod::new(Self::make_content(lang)),
            dragging_popup: WidgetPod::new(DraggingPopup::new(lang)),
        }
    }

    fn make_content(lang: &'static LanguageConfig) -> Flex<EditorModel> {
        Flex::row()
            .with_flex_child(
                Scroll::new(TextEditor::new(lang))
                    .content_must_fill(true)
                    .expand(),
                1.0,
            )
            .with_child(
                Scroll::new(BlockPalette::new(lang))
                    .content_must_fill(true)
                    .expand_height(),
            )
            .must_fill_main_axis(true)
    }
}

impl Widget<EditorModel> for BlockEditor {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut EditorModel,
        env: &druid::Env,
    ) {
        self.dragging_popup.event(ctx, event, data, env);
        self.content.event(ctx, event, data, env);

        match event {
            Event::MouseDown(mouse) => {
                if mouse.button == MouseButton::Left {
                    // set the origin of the dragging popup if dragging
                    if data.drag_block.is_some() {
                        let origin = self.dragging_popup.widget().calc_origin(mouse.pos);
                        self.dragging_popup.set_origin(ctx, origin);
                    }

                    ctx.request_layout();
                    ctx.request_paint();
                }
            }

            Event::MouseMove(mouse) => {
                if mouse.buttons.has_left() {
                    // set the origin of the dragging popup if dragging
                    if data.drag_block.is_some() {
                        let origin = self.dragging_popup.widget().calc_origin(mouse.pos);
                        self.dragging_popup.set_origin(ctx, origin);
                    }

                    ctx.request_paint();
                }
            }

            Event::Command(command) => {
                if let Some(file_name) = command.get(commands::SET_FILE_NAME) {
                    let new_lang = lang_for_file(file_name);
                    if self.language.name != new_lang.name {
                        self.language = new_lang;
                        self.dragging_popup.widget_mut().change_language(new_lang);
                    }
                }
            }

            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        data: &EditorModel,
        env: &druid::Env,
    ) {
        self.dragging_popup.lifecycle(ctx, event, data, env);
        self.content.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        ctx: &mut druid::UpdateCtx,
        _old_data: &EditorModel,
        data: &EditorModel,
        env: &druid::Env,
    ) {
        self.dragging_popup.update(ctx, data, env);
        self.content.update(ctx, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &EditorModel,
        env: &druid::Env,
    ) -> druid::Size {
        self.dragging_popup.layout(ctx, bc, data, env);
        self.content.layout(ctx, bc, data, env);

        bc.constrain(druid::Size {
            width: f64::INFINITY,
            height: f64::INFINITY,
        })
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &EditorModel, env: &druid::Env) {
        self.content.paint(ctx, data, env);

        if data.drag_block.is_some() {
            self.dragging_popup.paint(ctx, data, env);
        }
    }
}
