use druid::{
    piet::{PietTextLayout, Text, TextLayoutBuilder},
    Color, Event, MouseButton, PaintCtx, Point, RenderContext, Size, Widget,
};
use ropey::Rope;

use crate::{
    block_editor::{
        commands, EditorModel, FONT_FAMILY, FONT_HEIGHT, FONT_SIZE, FONT_WIDTH, OUTER_PAD,
        TOTAL_TEXT_X_OFFSET,
    },
    lsp::diagnostics::{Diagnostic, VSCodeCodeAction},
    theme,
};

pub struct DiagnosticPopup {
    fixes: Option<Vec<VSCodeCodeAction>>,
    curr_diagnostic: Option<Diagnostic>,
}

impl DiagnosticPopup {
    pub fn new() -> Self {
        DiagnosticPopup {
            fixes: None,
            curr_diagnostic: None,
        }
    }

    fn get_diagnostic(data: &EditorModel) -> Option<Diagnostic> {
        if let Some(selection) = data.diagnostic_selection {
            if let Some(diagnostic) = data.diagnostics.iter().find(|d| d.id == selection) {
                return Some(diagnostic.clone());
            } else {
                crate::console_log!("could not find diagnostic to present fixes");
            }
        }
        None
    }

    pub fn calc_origin(&self, padding: &[f64]) -> Point {
        // TODO: if not enough room, put below line

        let Some(diagnostic) = &self.curr_diagnostic else {
            return Point::ZERO;
        };

        // find dimensions
        let font_height = *FONT_HEIGHT.get().unwrap();
        let mut height = font_height;
        if let Some(fixes) = &self.fixes {
            height += fixes.len() as f64 * font_height;
        }

        // find the vertical start by finding top of line and then subtracting box size
        let total_padding: f64 = padding.iter().take(diagnostic.range.start.line + 1).sum();
        let y =
            total_padding + (diagnostic.range.start.line as f64 * font_height) + OUTER_PAD - height;

        // find the horizontal start
        let x =
            TOTAL_TEXT_X_OFFSET + (diagnostic.range.start.col as f64 * FONT_WIDTH.get().unwrap());

        Point::new(x, y)
    }

    fn calc_size(&self) -> Size {
        let Some(diagnostic) = &self.curr_diagnostic else {
            return Size::ZERO;
        };

        // find dimensions
        let font_height = *FONT_HEIGHT.get().unwrap();
        let mut height = font_height;
        if let Some(fixes) = &self.fixes {
            height += fixes.len() as f64 * font_height;
        }

        let text_len = diagnostic.message.chars().count();
        let total_fix_len: usize = if let Some(fixes) = &self.fixes {
            fixes
                .iter()
                .map(|fix| fix.title.chars().count())
                .max()
                .unwrap_or(0)
        } else {
            0
        };
        let width = std::cmp::max(text_len, total_fix_len) as f64 * FONT_WIDTH.get().unwrap();

        Size::new(width, height)
    }
}

impl Widget<EditorModel> for DiagnosticPopup {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut EditorModel,
        _env: &druid::Env,
    ) {
        let font_width = *FONT_WIDTH.get().unwrap();
        let font_height = *FONT_HEIGHT.get().unwrap();

        match event {
            Event::MouseDown(mouse) if mouse.button == MouseButton::Left => {
                if let Some(fixes) = &self.fixes {
                    // ignore message row
                    if mouse.pos.y > font_height {
                        // find fix for row clicked on
                        let fix_idx = ((mouse.pos.y - font_height) / font_height) as usize;
                        let fix = &fixes[fix_idx];

                        // run action if clicked in text in row
                        let x_char_clicked = (mouse.pos.x / font_width) as usize;
                        if x_char_clicked < fix.title.chars().count() {
                            fix.run();
                            data.diagnostic_selection = None;
                        }
                    }
                }
                ctx.set_handled();
            }
            Event::MouseUp(mouse) if mouse.button == MouseButton::Left => {
                ctx.set_handled();
            }
            Event::MouseMove(_) => {
                ctx.set_handled();
            }

            Event::Command(command) => {
                if let Some(fixes) = command.get(commands::SET_QUICK_FIX) {
                    // TODO: verify id matches
                    self.fixes = Some(fixes.clone());

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
        ctx: &mut druid::UpdateCtx,
        old_data: &EditorModel,
        data: &EditorModel,
        _env: &druid::Env,
    ) {
        if data.diagnostic_selection != old_data.diagnostic_selection {
            self.fixes = None;
            self.curr_diagnostic = Self::get_diagnostic(data);

            if let Some(diagnostic) = &self.curr_diagnostic {
                diagnostic.request_fixes();
            }

            ctx.request_layout();
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut druid::LayoutCtx,
        _bc: &druid::BoxConstraints,
        _data: &EditorModel,
        _env: &druid::Env,
    ) -> druid::Size {
        self.calc_size()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &EditorModel, _env: &druid::Env) {
        // TODO: look good

        // set background color
        let rect = ctx.size().to_rect();
        ctx.fill(rect, &theme::POPUP_BACKGROUND);

        // draw message
        if let Some(diagnostic) = &self.curr_diagnostic {
            let message = &diagnostic.message;
            let layout = make_text_layout(message, crate::theme::syntax::DEFAULT, ctx);
            let pos = Point::new(0.0, 0.0);
            ctx.draw_text(&layout, pos);
        }

        // draw fixes
        if let Some(fixes) = &self.fixes {
            for (line, fix) in fixes.iter().enumerate() {
                let pos = Point::new(0.0, (line + 1) as f64 * FONT_HEIGHT.get().unwrap());
                let layout = make_text_layout(&fix.title, crate::theme::syntax::FUNCTION, ctx);
                ctx.draw_text(&layout, pos);
            }
        }
    }
}

impl Diagnostic {
    pub fn draw(&self, padding: &[f64], source: &Rope, ctx: &mut PaintCtx) {
        let range = self.range.ordered();
        let line_ranges = range.individual_lines(source);

        let mut total_padding: f64 = padding.iter().take(range.start.line).sum();

        for line_range in line_ranges {
            let line_num = line_range.start.line;

            total_padding += padding[line_num];

            // find bottom of current line
            let y =
                total_padding + ((line_num + 1) as f64 * FONT_HEIGHT.get().unwrap()) + OUTER_PAD;

            // find the start and end of the line
            let x = TOTAL_TEXT_X_OFFSET + (line_range.start.col as f64 * FONT_WIDTH.get().unwrap());
            let width =
                (line_range.end.col - line_range.start.col) as f64 * FONT_WIDTH.get().unwrap();

            // draw line
            let line = druid::kurbo::Line::new((x, y), (x + width, y));
            ctx.stroke(line, &self.severity.color(), 2.0);
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
