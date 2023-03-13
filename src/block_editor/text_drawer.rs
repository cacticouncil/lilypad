use druid::{
    piet::{
        PietTextLayout, PietTextLayoutBuilder, Text, TextAttribute, TextLayout, TextLayoutBuilder,
    },
    Color, FontFamily, PaintCtx, Point, RenderContext,
};
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};

use crate::block_editor::FONT_HEIGHT;
use crate::theme;

pub struct TextDrawer {
    highlighter: Highlighter,
    highlighter_config: HighlightConfiguration,
    text_changed: bool,

    // piet-web does not support ranged attributes so every
    // color must be a separate text layout
    #[cfg(not(target_family = "wasm"))]
    cache: Vec<PietTextLayout>,
    #[cfg(target_family = "wasm")]
    cache: Vec<Vec<PietTextLayout>>, // potential optimization: use spacers instead of full layouts for whitespace
}

impl TextDrawer {
    pub fn new() -> Self {
        let highlighter = Highlighter::new();
        let mut highlighter_config = HighlightConfiguration::new(
            tree_sitter_python_wasm_compatible::language(),
            tree_sitter_python_wasm_compatible::HIGHLIGHT_QUERY,
            "",
            "",
        )
        .unwrap();
        highlighter_config.configure(HIGHLIGHT_NAMES);

        Self {
            highlighter,
            highlighter_config,
            cache: vec![],
            text_changed: true,
        }
    }

    pub fn text_changed(&mut self) {
        self.text_changed = true
    }
}

#[cfg(not(target_family = "wasm"))]
impl TextDrawer {
    pub fn draw(&mut self, source: &str, ctx: &mut PaintCtx) {
        if self.text_changed {
            self.layout(source, ctx);
        }

        for (num, layout) in self.cache.iter().enumerate() {
            let pos = Point {
                x: 0.0,
                y: (num as f64) * FONT_HEIGHT,
            };
            ctx.draw_text(layout, pos);
        }
    }

    fn layout(&mut self, source: &str, ctx: &mut PaintCtx) {
        // erase old values
        self.cache.clear();

        let font_family = FontFamily::new_unchecked("Roboto Mono");

        for line in source.lines() {
            let mut layout = ctx
                .text()
                .new_text_layout(line.to_string())
                .font(font_family.clone(), 15.0)
                .default_attribute(TextAttribute::TextColor(theme::syntax::DEFAULT));

            // get highlights and color them
            let highlights = self
                .highlighter
                .highlight(&self.highlighter_config, line.as_bytes(), None, |_| None)
                .unwrap();
            layout = Self::apply_highlight_events(highlights, layout);

            let built_layout = layout.build().unwrap();
            self.cache.push(built_layout);
        }
    }

    fn apply_highlight_events(
        events: impl Iterator<Item = Result<HighlightEvent, tree_sitter_highlight::Error>>,
        mut layout: PietTextLayoutBuilder,
    ) -> PietTextLayoutBuilder {
        let mut handled_up_to = 0;
        let mut next_to_handle = 0;
        let mut category_stack: Vec<Highlight> = vec![];

        for event in events {
            match event.unwrap() {
                HighlightEvent::Source { start: _, end } => {
                    // note: end is exclusive
                    next_to_handle = end;
                }
                HighlightEvent::HighlightStart(s) => {
                    // if there was a gap since the last,
                    // it should be handled as the category it falls into
                    if next_to_handle != handled_up_to {
                        if let Some(cat) = category_stack.last() {
                            layout = layout.range_attribute(
                                handled_up_to..next_to_handle,
                                TextAttribute::TextColor(get_text_color(*cat)),
                            );
                        }

                        // mark that range as handled
                        handled_up_to = next_to_handle;
                    }

                    // start new
                    category_stack.push(s);
                }
                HighlightEvent::HighlightEnd => {
                    let cat = category_stack.pop().unwrap();
                    layout = layout.range_attribute(
                        handled_up_to..next_to_handle,
                        TextAttribute::TextColor(get_text_color(cat)),
                    );
                    handled_up_to = next_to_handle;
                }
            }
        }

        layout
    }
}

#[cfg(target_family = "wasm")]
impl TextDrawer {
    pub fn draw(&mut self, source: &str, ctx: &mut PaintCtx) {
        if self.text_changed {
            self.layout(source, ctx);
        }

        for (line_num, layouts) in self.cache.iter().enumerate() {
            let mut pos = Point {
                x: 0.0,
                y: (line_num as f64) * FONT_HEIGHT,
            };
            for layout in layouts {
                ctx.draw_text(layout, pos);
                pos.x += layout.trailing_whitespace_width();
            }
        }
    }

    fn layout(&mut self, source: &str, ctx: &mut PaintCtx) {
        // erase old values
        self.cache.clear();

        for line in source.lines() {
            // get highlights and color them
            let highlights = self
                .highlighter
                .highlight(&self.highlighter_config, line.as_bytes(), None, |_| None)
                .unwrap();
            let layouts = Self::highlight_events_to_layouts(highlights, line, ctx);

            // let built_layout = layout.build().unwrap();
            self.cache.push(layouts);
        }
    }

    /// generates a text layout for each color because piet-web doesn't support ranged attributes
    fn highlight_events_to_layouts(
        events: impl Iterator<Item = Result<HighlightEvent, tree_sitter_highlight::Error>>,
        line: &str,
        ctx: &mut PaintCtx,
    ) -> Vec<PietTextLayout> {
        let mut handled_up_to = 0;
        let mut next_to_handle = 0;
        let mut category_stack: Vec<Highlight> = vec![];

        let mut layouts = vec![];

        for event in events {
            match event.unwrap() {
                HighlightEvent::Source { start: _, end } => {
                    // note: end is exclusive
                    next_to_handle = end;
                }
                HighlightEvent::HighlightStart(s) => {
                    // if there was a gap since the last,
                    // it should be handled as the category it falls into
                    if next_to_handle != handled_up_to {
                        let color = match category_stack.last() {
                            Some(cat) => get_text_color(*cat),
                            None => theme::syntax::DEFAULT,
                        };
                        let text = &line[handled_up_to..next_to_handle];
                        layouts.push(Self::make_text_layout(text, color, ctx));

                        // mark that range as handled
                        handled_up_to = next_to_handle;
                    }

                    // start new
                    category_stack.push(s);
                }
                HighlightEvent::HighlightEnd => {
                    let cat = category_stack.pop().unwrap();
                    let text = &line[handled_up_to..next_to_handle];
                    layouts.push(Self::make_text_layout(text, get_text_color(cat), ctx));
                    handled_up_to = next_to_handle;
                }
            }
        }

        // add the rest of the line
        if handled_up_to != line.len() {
            let text = &line[handled_up_to..line.len()];
            layouts.push(Self::make_text_layout(text, theme::syntax::DEFAULT, ctx));
        }

        layouts
    }

    fn make_text_layout(text: &str, color: Color, ctx: &mut PaintCtx) -> PietTextLayout {
        let font_family = FontFamily::new_unchecked("Roboto Mono");
        ctx.text()
            .new_text_layout(text.to_string())
            .font(font_family, 15.0)
            .text_color(color)
            .build()
            .unwrap()
    }
}

const HIGHLIGHT_NAMES: &[&str] = &[
    "function",
    "function.builtin",
    "keyword",
    "operator",
    "property",
    "punctuation.special", // interpolation surrounding
    "string",
    "type",
    "variable",
    "constructor",
    "constant",
    "constant.builtin",
    "number",
    "escape",
    "comment",
    "embedded", // inside of interpolation
];

fn get_text_color(highlight: Highlight) -> Color {
    use theme::syntax::*;

    // indexes of the above array
    match highlight.0 {
        0 => FUNCTION,
        1 => FUNCTION_BUILT_IN,
        2 => KEYWORD,
        3 => OPERATOR,
        4 => PROPERTY,
        5 => INTERPOLATION,
        6 => STRING,
        7 => TYPE,
        8 => VARIABLE,
        9 => CONSTRUCTOR,
        10 => CONSTANT,
        11 => LITERAL,
        12 => LITERAL,
        13 => ESCAPE_SEQUENCE,
        14 => COMMENT,
        15 => DEFAULT, // treat inside of interpolation like top level
        _ => unreachable!(),
    }
}
