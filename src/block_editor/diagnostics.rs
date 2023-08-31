use druid::{
    piet::{PietTextLayout, Text, TextLayoutBuilder},
    Color, Event, MouseButton, PaintCtx, Point, Rect, RenderContext, Size, Widget,
};
use serde::{Deserialize, Serialize};

use crate::{theme, vscode};

use super::{
    text_range::{TextPoint, TextRange},
    EditorModel, FONT_FAMILY, FONT_HEIGHT, FONT_SIZE, FONT_WIDTH,
};

/* -------------------------------- Data Type ------------------------------- */

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub range: TextRange,
    pub severity: DiagnosticSeverity,
    pub source: String,
    #[serde(skip, default = "rand_u64")]
    pub id: u64,
}

#[derive(Deserialize, Debug, Clone, PartialEq, PartialOrd)]
pub enum DiagnosticSeverity {
    Error = 3,
    Warning = 2,
    Information = 1,
    Hint = 0,
}

impl DiagnosticSeverity {
    fn color(&self) -> Color {
        use crate::theme::diagnostic::*;
        use DiagnosticSeverity::*;

        match self {
            Error => ERROR,
            Warning => WARNING,
            Information => INFO,
            Hint => HINT,
        }
    }
}

fn rand_u64() -> u64 {
    let mut buf = [0u8; 8];
    getrandom::getrandom(&mut buf).expect("Failed to generate random bytes");
    u64::from_le_bytes(buf)
}

impl Diagnostic {
    pub fn draw(&self, padding: &[f64], ctx: &mut PaintCtx) {
        // TODO: multiline diagnostic underlines
        // could probably share the same logic as selections

        // find bottom of current line
        let total_padding: f64 = padding.iter().take(self.range.start.row + 1).sum();
        let y = total_padding
            + ((self.range.start.row + 1) as f64 * FONT_HEIGHT.get().unwrap())
            + super::OUTER_PAD;

        // find the start and end of the line
        let x =
            super::TOTAL_TEXT_X_OFFSET + (self.range.start.col as f64 * FONT_WIDTH.get().unwrap());
        let width = (self.range.end.col - self.range.start.col) as f64 * FONT_WIDTH.get().unwrap();

        // draw
        let line = druid::kurbo::Line::new((x, y), (x + width, y));
        ctx.stroke(line, &self.severity.color(), 2.0);
    }

    pub fn request_fixes(&self) {
        crate::vscode::request_quick_fixes(self.range.start.row, self.range.start.col);
    }

    #[allow(dead_code)]
    pub fn example() -> Diagnostic {
        Diagnostic {
            message: "example diagnostic".to_string(),
            range: TextRange::new(TextPoint::new(18, 2), TextPoint::new(25, 2)),
            severity: DiagnosticSeverity::Error,
            source: "example".to_string(),
            id: rand_u64(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct VSCodeCodeAction {
    title: String,
    #[serde(rename = "edit")]
    workspace_edit: Option<serde_json::Value>,
    command: Option<VSCodeCommand>,
}

impl VSCodeCodeAction {
    pub fn run(&self) {
        // run workspace edit then command
        if let Some(workspace_edit) = &self.workspace_edit {
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            vscode::execute_workspace_edit(
                workspace_edit.serialize(&serializer).unwrap_or_default(),
            );
        }
        if let Some(command) = &self.command {
            command.run();
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct VSCodeCommand {
    command: String,
    arguments: serde_json::Value,
}

impl VSCodeCommand {
    pub fn run(&self) {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        vscode::execute_command(
            self.command.clone(),
            self.arguments.serialize(&serializer).unwrap_or_default(),
        );
    }
}

/* --------------------------------- Widget --------------------------------- */

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
        let total_padding: f64 = padding.iter().take(diagnostic.range.start.row + 1).sum();
        let y =
            total_padding + (diagnostic.range.start.row as f64 * font_height) + super::OUTER_PAD
                - height;

        // find the horizontal start
        let x = super::TOTAL_TEXT_X_OFFSET
            + (diagnostic.range.start.col as f64 * FONT_WIDTH.get().unwrap());

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
                if let Some(fixes) = command.get(vscode::SET_QUICK_FIX_SELECTOR) {
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
        let rect = Rect::from_origin_size(Point::ZERO, ctx.size());
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
