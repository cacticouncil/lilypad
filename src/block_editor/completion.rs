use druid::{
    piet::{PietTextLayout, Text, TextLayoutBuilder},
    Color, Event, MouseButton, PaintCtx, Point, Rect, RenderContext, Selector, Size, Widget,
};
use serde::Deserialize;

use super::{
    text_range::TextPoint, EditorModel, FONT_FAMILY, FONT_HEIGHT, FONT_SIZE, FONT_WIDTH,
    TOTAL_TEXT_X_OFFSET,
};
use crate::{theme, vscode};

pub const APPLY_COMPLETION_SELECTOR: Selector<String> = Selector::new("apply_completion");

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VSCodeCompletionItem {
    label: String,
    insert_text: String,
    kind: Option<VSCodeCompletionKind>,
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[repr(usize)]
enum VSCodeCompletionKind {
    Class,
    Color,
    Constant,
    Constructor,
    Enum,
    EnumMember,
    Event,
    Field,
    File,
    Folder,
    Function,
    Interface,
    Issue,
    Keyword,
    Method,
    Module,
    Operator,
    Property,
    Reference,
    Snippet,
    Struct,
    Text,
    TypeParameter,
    Unit,
    User,
    Value,
    Variable,
}

impl VSCodeCompletionKind {
    fn color(self) -> Color {
        use VSCodeCompletionKind::*;

        match self {
            Function => theme::syntax::FUNCTION,
            Constant | Variable => theme::syntax::VARIABLE,
            Keyword => theme::syntax::KEYWORD,
            _ => theme::syntax::DEFAULT,
        }
    }
}

pub struct CompletionPopup {
    completions: Option<Vec<VSCodeCompletionItem>>,
    selection: usize,
}

impl CompletionPopup {
    pub fn new() -> Self {
        Self {
            completions: None,
            selection: 0,
        }
    }

    pub fn calc_origin(&self, padding: &[f64], cursor: TextPoint) -> Point {
        // find the bottom of the current selection
        let total_padding: f64 = padding.iter().take(cursor.row + 1).sum();
        let y = (cursor.row as f64 + 2.0) * *FONT_HEIGHT.get().unwrap() + total_padding;
        let x = (cursor.col as f64) * *FONT_WIDTH.get().unwrap() + TOTAL_TEXT_X_OFFSET;
        Point::new(x, y)
    }

    pub fn clear(&mut self) {
        self.completions = None;
    }

    fn calc_size(&self) -> Size {
        let Some(completions) = &self.completions else {
            return Size::ZERO;
        };

        let height = *FONT_HEIGHT.get().unwrap() * completions.len() as f64;

        let max_label_len: usize = completions
            .iter()
            .map(|fix| fix.label.chars().count())
            .max()
            .unwrap_or(0);
        let width = max_label_len as f64 * FONT_WIDTH.get().unwrap();

        Size::new(width, height)
    }
}

impl Widget<EditorModel> for CompletionPopup {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &Event,
        _data: &mut EditorModel,
        _env: &druid::Env,
    ) {
        let font_height = *FONT_HEIGHT.get().unwrap();

        match event {
            Event::MouseDown(mouse) if mouse.button == MouseButton::Left => {
                if let Some(completions) = &self.completions {
                    // find fix for row clicked on
                    let row_clicked = (mouse.pos.y / font_height) as usize;
                    crate::console_log!("mouse pos: {:?}", mouse.pos);
                    crate::console_log!("row clicked: {}", row_clicked);

                    // catch clicking out of bounds
                    if row_clicked >= completions.len() {
                        return;
                    }

                    // highlight the row clicked on
                    self.selection = row_clicked;

                    ctx.request_paint();
                }
                ctx.set_handled();
            }
            Event::MouseUp(mouse) if mouse.button == MouseButton::Left => {
                // trigger the completion
                if let Some(completions) = &self.completions {
                    if let Some(completion) = completions.get(self.selection) {
                        ctx.submit_command(
                            APPLY_COMPLETION_SELECTOR.with(completion.insert_text.clone()),
                        );

                        self.completions = None;
                        ctx.request_layout();
                        ctx.request_paint();
                    }
                }

                ctx.set_handled();
            }
            Event::KeyDown(key) => {
                if let Some(completions) = &self.completions {
                    match key.key {
                        druid::keyboard_types::Key::ArrowUp => {
                            self.selection = self.selection.saturating_sub(1);
                            ctx.request_paint();
                            ctx.set_handled();
                        }
                        druid::keyboard_types::Key::ArrowDown => {
                            self.selection =
                                std::cmp::min(self.selection + 1, completions.len() - 1);
                            ctx.request_paint();
                            ctx.set_handled();
                        }
                        druid::keyboard_types::Key::Enter => {
                            if let Some(completion) = completions.get(self.selection) {
                                ctx.submit_command(
                                    APPLY_COMPLETION_SELECTOR.with(completion.insert_text.clone()),
                                );

                                self.completions = None;
                                ctx.request_layout();
                                ctx.request_paint();
                                ctx.set_handled();
                            }
                        }
                        druid::keyboard_types::Key::Escape => {
                            self.completions = None;
                            ctx.request_layout();
                            ctx.request_paint();
                            ctx.set_handled();
                        }
                        _ => {}
                    }
                }
            }
            Event::Command(command) => {
                if let Some(completions) = command.get(vscode::SET_COMPLETIONS_SELECTOR) {
                    let mut completions = completions.clone();
                    completions.truncate(10);

                    self.completions = Some(completions);
                    self.selection = 0;

                    ctx.request_layout();
                    ctx.request_paint();
                    ctx.set_handled();
                }
            }

            _ => {}
        }
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
        _ctx: &mut druid::UpdateCtx,
        _old_data: &EditorModel,
        _data: &EditorModel,
        _env: &druid::Env,
    ) {
    }

    fn layout(
        &mut self,
        _ctx: &mut druid::LayoutCtx,
        _bc: &druid::BoxConstraints,
        _data: &EditorModel,
        _env: &druid::Env,
    ) -> Size {
        self.calc_size()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &EditorModel, _env: &druid::Env) {
        // TODO: look good

        if let Some(completions) = &self.completions {
            // set background color
            let rect = Rect::from_origin_size(Point::ZERO, ctx.size());
            ctx.fill(rect, &theme::POPUP_BACKGROUND);

            // draw completions
            let font_height = *FONT_HEIGHT.get().unwrap();
            for (line, completion) in completions.iter().enumerate() {
                // highlight background if selected
                if line == self.selection {
                    let rect = Rect::from_origin_size(
                        Point::new(0.0, line as f64 * font_height),
                        Size::new(ctx.size().width, font_height),
                    );
                    ctx.fill(rect, &theme::SELECTION);
                }

                let pos = Point::new(0.0, line as f64 * font_height);
                let color = completion
                    .kind
                    .map_or(theme::syntax::DEFAULT, |k| k.color());
                let layout = make_text_layout(&completion.label, color, ctx);
                ctx.draw_text(&layout, pos);
            }
        }
    }
}

fn make_text_layout(text: &str, color: Color, ctx: &mut PaintCtx) -> PietTextLayout {
    ctx.text()
        .new_text_layout(text.to_string())
        .font(
            FONT_FAMILY.get().unwrap().clone(),
            *FONT_SIZE.get().unwrap(),
        )
        .text_color(color)
        .build()
        .unwrap()
}
