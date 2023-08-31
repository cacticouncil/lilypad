use druid::{
    piet::{PietTextLayout, Text, TextLayoutBuilder},
    Color, PaintCtx, Point, RenderContext,
};
use ropey::Rope;
use tree_sitter_c2rust::Node;

use std::{
    borrow::Cow,
    cmp::{max, min},
    ops::Range,
};

use super::highlighter::{Highlight, HighlightConfiguration, HighlightEvent};
use super::{FONT_FAMILY, FONT_HEIGHT, FONT_SIZE};
use crate::{lang::LanguageConfig, theme};

#[cfg(target_family = "wasm")]
use druid::piet::TextLayout;

#[cfg(not(target_family = "wasm"))]
use druid::piet::TextAttribute;

pub struct TextDrawer {
    highlighter_config: HighlightConfiguration,
    cache: Vec<ColoredText>,
}

impl TextDrawer {
    pub fn new(lang: &LanguageConfig) -> Self {
        let mut highlighter_config =
            HighlightConfiguration::new(lang.tree_sitter(), lang.highlight_query, "").unwrap();
        highlighter_config.configure(HIGHLIGHT_NAMES);

        Self {
            highlighter_config,
            cache: vec![],
        }
    }

    pub fn change_language(&mut self, lang: &LanguageConfig) {
        let mut highlighter_config =
            HighlightConfiguration::new(lang.tree_sitter(), lang.highlight_query, "").unwrap();
        highlighter_config.configure(HIGHLIGHT_NAMES);
        self.highlighter_config = highlighter_config
    }

    pub fn draw(&self, padding: &[f64], ctx: &mut PaintCtx) {
        let mut total_padding = 0.0;
        for (num, layout) in self.cache.iter().enumerate() {
            total_padding += padding[num];
            let pos = Point {
                x: super::TOTAL_TEXT_X_OFFSET,
                y: ((num as f64) * FONT_HEIGHT.get().unwrap()) + total_padding + super::OUTER_PAD,
            };
            layout.draw(pos, ctx);
        }
    }

    pub fn layout(&mut self, root_node: Node, source: &Rope, ctx: &mut PaintCtx) {
        // erase old values
        self.cache.clear();

        // get highlights
        let mut highlights = self
            .highlighter_config
            .highlight(source.slice(..), &root_node)
            .peekable();

        let mut handled_up_to = 0;
        let mut next_to_handle = 0;
        let mut start_of_line = 0;
        let mut category_stack: Vec<Highlight> = vec![];

        for line in source.lines() {
            // Cow::from uses a reference in most cases (since lines are usually short)
            // but if it crosses a chunk boundary, it will allocate a new string
            let mut colored_text = ColoredTextBuilder::new(Cow::from(line));
            let end_of_line = start_of_line + line.len_bytes();

            // apply highlight attributes
            loop {
                // break when out of highlights
                let Some(highlight) = highlights.peek() else {
                    break;
                };

                match highlight {
                    HighlightEvent::Source { start: _, end } => {
                        // note: end is exclusive
                        next_to_handle = *end;
                        highlights.next();
                    }
                    HighlightEvent::HighlightStart(cat) => {
                        // if starting beyond end of line, go to the next line
                        if handled_up_to >= end_of_line {
                            break;
                        }

                        // if there was a gap since the last,
                        // it should be handled as the category it falls into
                        if next_to_handle != handled_up_to {
                            // limit ranges to line
                            let start = max(handled_up_to, start_of_line);
                            let end = min(next_to_handle, end_of_line);

                            if let Some(cat) = category_stack.last() {
                                colored_text.add_color(
                                    get_text_color(*cat),
                                    (start - start_of_line)..(end - start_of_line),
                                );
                            }

                            // mark that range as handled
                            handled_up_to = end;
                        }

                        // start new if we have reached it on this line
                        if handled_up_to == next_to_handle {
                            category_stack.push(*cat);
                            highlights.next();
                        }
                    }
                    HighlightEvent::HighlightEnd => {
                        let cat = category_stack.pop().unwrap();

                        // limit ranges to line
                        let range_start = max(handled_up_to, start_of_line);
                        let range_end = min(next_to_handle, end_of_line);

                        colored_text.add_color(
                            get_text_color(cat),
                            (range_start - start_of_line)..(range_end - start_of_line),
                        );
                        handled_up_to = range_end;

                        // if category ends on future line,
                        // do not remove highlight end (so it triggers again on next line)
                        // and keep category on the stack (so it knows when triggered again)
                        if end_of_line < next_to_handle {
                            category_stack.push(cat);

                            // passed end of line so can just end here
                            break;
                        } else {
                            highlights.next();
                        }
                    }
                }
            }

            // build
            self.cache.push(colored_text.build(ctx));

            // prepare for next
            start_of_line = end_of_line;
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
        5 => INTERPOLATION_SURROUNDING,
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

struct ColorRange {
    color: Color,
    range: Range<usize>,
}

struct ColoredTextBuilder<'a> {
    text: Cow<'a, str>,
    color_ranges: Vec<ColorRange>,
}

struct ColoredText {
    #[cfg(not(target_family = "wasm"))]
    layout: PietTextLayout,

    #[cfg(target_family = "wasm")]
    /// html canvas (and therefor piet-web) does not
    /// support ranged attributes so every
    /// color must be a separate text layout
    layouts: Vec<PietTextLayout>,
}

impl<'a> ColoredTextBuilder<'a> {
    fn new(text: Cow<'a, str>) -> Self {
        Self {
            text,
            color_ranges: vec![],
        }
    }

    fn add_color(&mut self, color: Color, range: Range<usize>) {
        self.color_ranges.push(ColorRange { color, range });
    }

    #[cfg(not(target_family = "wasm"))]
    fn build(self, ctx: &mut PaintCtx) -> ColoredText {
        let mut layout = ctx
            .text()
            .new_text_layout(self.text.into_owned())
            .font(
                FONT_FAMILY.get().unwrap().clone(),
                *FONT_SIZE.get().unwrap(),
            )
            .default_attribute(TextAttribute::TextColor(theme::syntax::DEFAULT));

        // apply colors
        for color_range in self.color_ranges {
            layout = layout.range_attribute(
                color_range.range,
                TextAttribute::TextColor(color_range.color),
            );
        }

        ColoredText {
            layout: layout.build().unwrap(),
        }
    }

    #[cfg(target_family = "wasm")]
    fn build(self, ctx: &mut PaintCtx) -> ColoredText {
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

        let mut layouts = vec![];

        // apply colors
        let mut handled_up_to = 0;
        for color_range in self.color_ranges {
            // add anything this might have skipped
            if handled_up_to < color_range.range.start {
                let text = &self.text[handled_up_to..color_range.range.start];
                layouts.push(make_text_layout(text, theme::syntax::DEFAULT, ctx));
            }

            // add this
            handled_up_to = color_range.range.end;
            let text = &self.text[color_range.range];
            layouts.push(make_text_layout(text, color_range.color, ctx));
        }

        // add the rest
        if handled_up_to != self.text.len() {
            let text = &self.text[handled_up_to..];
            layouts.push(make_text_layout(text, theme::syntax::DEFAULT, ctx));
        }

        ColoredText { layouts }
    }
}

impl ColoredText {
    #[cfg(not(target_family = "wasm"))]
    fn draw(&self, pos: Point, ctx: &mut PaintCtx) {
        ctx.draw_text(&self.layout, pos);
    }

    #[cfg(target_family = "wasm")]
    fn draw(&self, mut pos: Point, ctx: &mut PaintCtx) {
        // potential optimization: use spacers instead of full layouts for whitespace
        for layout in &self.layouts {
            ctx.draw_text(layout, pos);
            pos.x += layout.trailing_whitespace_width();
        }
    }
}
