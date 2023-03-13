use druid::{
    piet::{PietTextLayout, Text, TextAttribute, TextLayoutBuilder},
    Color, FontFamily, PaintCtx, Point, RenderContext,
};
use std::ops::Range;
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};

use crate::block_editor::FONT_HEIGHT;
use crate::theme;

pub struct TextDrawer {
    highlighter: Highlighter,
    highlighter_config: HighlightConfiguration,
    cache: Vec<PietTextLayout>,
    text_changed: bool,
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

            let highlights = self
                .highlighter
                .highlight(&self.highlighter_config, line.as_bytes(), None, |_| None)
                .unwrap();

            let highlight_ranges = Self::highlight_events_to_ranges(highlights);

            // add highlight attributes
            for x in highlight_ranges {
                layout = layout.range_attribute(
                    x.range,
                    TextAttribute::TextColor(get_text_color(x.highlight)),
                );
            }

            let built_layout = layout.build().unwrap();
            self.cache.push(built_layout);
        }
    }

    fn highlight_events_to_ranges(
        events: impl Iterator<Item = Result<HighlightEvent, tree_sitter_highlight::Error>>,
    ) -> Vec<HighlightRange> {
        let mut handled_up_to = 0;
        let mut next_to_handle = 0;
        let mut category_stack: Vec<Highlight> = vec![];
        let mut highlight_ranges: Vec<HighlightRange> = vec![];

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
                            highlight_ranges.push(HighlightRange::new(
                                *cat,
                                handled_up_to,
                                next_to_handle,
                            ));
                        }

                        // mark that range as handled
                        handled_up_to = next_to_handle;
                    }

                    // start new
                    category_stack.push(s);
                }
                HighlightEvent::HighlightEnd => {
                    let cat = category_stack.pop().unwrap();
                    highlight_ranges.push(HighlightRange::new(cat, handled_up_to, next_to_handle));
                    handled_up_to = next_to_handle;
                }
            }
        }

        highlight_ranges
    }
}

#[derive(Debug)]
struct HighlightRange {
    highlight: Highlight,
    range: Range<usize>,
}

impl HighlightRange {
    fn new(highlight: Highlight, start: usize, end: usize) -> Self {
        Self {
            highlight,
            range: Range { start, end },
        }
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
    "escape_sequence",
    "comment",
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
        _ => unreachable!(),
    }
}
