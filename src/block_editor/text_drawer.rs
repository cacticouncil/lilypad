use egui::{Align2, Color32, Painter, Vec2};
use ropey::Rope;
use tree_sitter::Node;

use std::{
    borrow::Cow,
    cmp::{max, min},
    iter::Peekable,
    ops::{Range, RangeInclusive},
};

use super::{blocks::Padding, source::Source, MonospaceFont};
use crate::{
    lang::{
        config::LanguageConfig,
        highlighter::{Highlight, HighlightEvent},
        Language,
    },
    theme,
};

// TODO: probably should have text drawers share highlight configurations
pub struct TextDrawer {
    cache: Vec<ColoredText>,
}

impl TextDrawer {
    pub fn new() -> Self {
        Self { cache: vec![] }
    }

    pub fn draw(
        &self,
        padding: &Padding,
        offset: Vec2,
        visible_lines: Option<RangeInclusive<usize>>,
        font: &MonospaceFont,
        painter: &Painter,
    ) {
        for (num, layout) in self.cache.iter().enumerate() {
            if let Some(range) = &visible_lines {
                if num < *range.start() {
                    continue;
                } else if num > *range.end() {
                    break;
                }
            }

            let total_offset = Vec2 {
                x: offset.x,
                y: ((num as f32) * font.size.y) + padding.cumulative(num) + offset.y,
            };
            layout.draw(total_offset, font, painter);
        }
    }

    pub fn highlight_source(&mut self, source: &mut Source) {
        let mut highlighter = source.lang.highlighter.borrow_mut();
        let highlight_config = source.lang.highlight_config.borrow_mut();
        let node = source.get_tree_cursor().node();
        let highlights = highlighter
            .highlight_existing_tree(source.text().slice(..), node, &highlight_config)
            .peekable();

        self.handle_highlights(highlights, source.text(), source.lang.config);
    }

    pub fn highlight(&mut self, root_node: Node, source: &Rope, lang: &mut Language) {
        let mut highlighter = lang.highlighter.borrow_mut();
        let highlight_config = lang.highlight_config.borrow_mut();
        let highlights = highlighter
            .highlight_existing_tree(source.slice(..), root_node, &highlight_config)
            .peekable();

        self.handle_highlights(highlights, source, lang.config);
    }

    fn handle_highlights(
        &mut self,
        mut highlights: Peekable<impl Iterator<Item = HighlightEvent>>,
        source: &Rope,
        lang: &LanguageConfig,
    ) {
        // erase old values
        self.cache.clear();

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
                                    lang.highlight[cat.0].1,
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
                            lang.highlight[cat.0].1,
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
            self.cache.push(colored_text.build());

            // prepare for next
            start_of_line = end_of_line;
        }
    }
}

struct ColorRange {
    color: Color32,
    range: Range<usize>,
}

struct ColoredTextBuilder<'a> {
    text: Cow<'a, str>,
    color_ranges: Vec<ColorRange>,
}

struct ColoredText {
    chunks: Vec<(String, Color32)>,
}

impl<'a> ColoredTextBuilder<'a> {
    fn new(text: Cow<'a, str>) -> Self {
        Self {
            text,
            color_ranges: vec![],
        }
    }

    fn add_color(&mut self, color: Color32, range: Range<usize>) {
        self.color_ranges.push(ColorRange { color, range });
    }

    fn build(self) -> ColoredText {
        let mut chunks = vec![];

        // apply colors
        let mut handled_up_to = 0;
        for color_range in self.color_ranges {
            // add anything this might have skipped
            if handled_up_to < color_range.range.start {
                let text = &self.text[handled_up_to..color_range.range.start];
                chunks.push((text.to_string(), theme::syntax::DEFAULT));
            }

            // add this
            handled_up_to = color_range.range.end;
            let text = &self.text[color_range.range];
            chunks.push((text.to_string(), color_range.color));
        }

        // add the rest
        if handled_up_to != self.text.len() {
            let text = &self.text[handled_up_to..];
            chunks.push((text.to_string(), theme::syntax::DEFAULT));
        }

        ColoredText { chunks }
    }
}

impl ColoredText {
    fn draw(&self, mut offset: Vec2, font: &MonospaceFont, painter: &Painter) {
        // draw by character until egui fixes monospace layout by switching to cosmic-text:
        // https://github.com/emilk/egui/issues/3378
        for (text, color) in &self.chunks {
            for char in text.chars() {
                if !char.is_whitespace() {
                    painter.text(
                        offset.to_pos2(),
                        Align2::LEFT_TOP,
                        char,
                        font.id.clone(),
                        *color,
                    );
                }
                offset.x += font.size.x;
            }
        }
    }
}
