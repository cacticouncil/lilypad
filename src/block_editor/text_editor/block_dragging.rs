use std::{borrow::Cow, collections::HashMap};

use egui::{Painter, Pos2, Rect, Vec2};
use ropey::Rope;

use super::{
    coord_conversions::{pt_to_text_coord, pt_to_unbounded_text_coord, text_coord_to_pt},
    TextEdit, TextEditor,
};
use crate::{
    block_editor::{
        blocks::Block,
        rope_ext::{RopeExt, RopeSliceExt},
        source::{Source, UndoStopCondition},
        text_range::{
            movement::{HDir, HUnit, TextMovement},
            TextPoint, TextRange,
        },
        BlockType, DragSession, MonospaceFont, GUTTER_WIDTH, OUTER_PAD,
    },
    lang::config::NewScopeChar,
    theme, vscode,
};

impl TextEditor {
    pub fn start_block_drag(
        &mut self,
        mouse_pos: Pos2,
        drag_block: &mut Option<DragSession>,
        source: &mut Source,
        font: &MonospaceFont,
    ) {
        let cursor_pos = pt_to_text_coord(mouse_pos, &self.blocks.padding(), source.text(), font);

        if let Some(block) = block_for_point(self.blocks.trees(), cursor_pos, source.text()) {
            let mut text_range = block.text_range();

            vscode::log_event(
                "editor-block-drag",
                HashMap::from([
                    ("type", block.syntax_type.as_str()),
                    ("lang", source.lang.config.name),
                ]),
            );

            // select the whole first line (to get all the indent)
            text_range.start.col = 0;

            // normalize the text
            let char_range = text_range.char_range_in(source.text());
            let mut block_text = source.text().slice(char_range.clone()).to_string();
            block_text = normalize_indent(block_text);
            if !block_text.ends_with('\n') {
                // add a newline to the end if it doesn't have one
                block_text.push('\n');
            }

            // offset the dragging popup so it matches where the mouse picked up the block
            let block_corner = text_coord_to_pt(
                TextPoint {
                    col: block.col,
                    line: block.line,
                },
                self.blocks.padding(),
                font,
            );
            let relative_pos =
                Pos2::new(mouse_pos.x - block_corner.x, mouse_pos.y - block_corner.y);

            // set dragging popup
            *drag_block = Some(DragSession {
                text: block_text.clone(),
                offset: relative_pos,
            });

            // remove dragged block from source
            source.apply_edit(
                &TextEdit::delete(text_range),
                UndoStopCondition::Always,
                false,
                &mut self.selections,
            );
        }
    }

    pub fn drop_block(
        &mut self,
        drag_block: &mut Option<DragSession>,
        drop_point: TextPoint,
        source: &mut Source,
    ) -> bool {
        // note: using take() also sets to None
        if let Some(drag_block) = drag_block.take() {
            let mut indented_text = set_indent(&drag_block.text, drop_point.col);

            // if at the end of the file, and the last line doesn't have a newline, add one
            if drop_point.line == source.text().len_lines() {
                indented_text.insert_str(0, source.text().detect_linebreak());
            }

            // apply edit
            let insert_point = TextPoint::new(drop_point.line, 0);
            let edit = TextEdit::new(
                Cow::Owned(indented_text),
                TextRange::new_cursor(insert_point),
            );
            source.apply_edit(&edit, UndoStopCondition::Never, true, &mut self.selections);

            // move the cursor from the line after the block to the end of the text
            self.selections.move_cursor(
                TextMovement::horizontal(HUnit::Grapheme, HDir::Left),
                source,
            );

            true
        } else {
            false
        }
    }

    pub fn draw_dropping_line(
        &self,
        drop_point: TextPoint,
        viewport_width: f32,
        offset: Vec2,
        font: &MonospaceFont,
        painter: &Painter,
    ) {
        const THICKNESS: f32 = 4.0;

        let line_padding_above = self.blocks.padding().cumulative(drop_point.line);
        let y = (drop_point.line as f32) * font.size.y + OUTER_PAD + line_padding_above;
        let x = (drop_point.col as f32) * font.size.x + OUTER_PAD + GUTTER_WIDTH;

        let origin = Pos2::new(x, y);
        let size = Vec2::new(viewport_width - origin.x - 10.0, THICKNESS);
        let rect = Rect::from_min_size(origin + offset, size);
        painter.rect_filled(rect, 0.0, theme::CURSOR);
    }

    pub fn find_drop_point(
        &mut self,
        mouse_pos: Pos2,
        source: &mut Source,
        font: &MonospaceFont,
    ) -> TextPoint {
        // find the point adjusted so that it is based around between lines
        let adj_pos = Pos2::new(mouse_pos.x, mouse_pos.y + (font.size.y / 2.0));
        let coord = pt_to_unbounded_text_coord(adj_pos, &self.blocks.padding(), font);

        // clamp line to end of source
        let line = coord.line.min(source.text().len_lines());

        // limit to the first non-empty above line's level
        // (or 1 more if it ends in a new scope character)
        let mut relative_whitespace_line = line;
        let allowed_indent = loop {
            if relative_whitespace_line == 0 {
                break 0;
            }

            let line_above = source.text().line(relative_whitespace_line - 1);
            let above_indent = line_above.whitespace_at_start();

            // if the line above is entirely whitespace, move to the line above
            if above_indent == line_above.len_chars_no_linebreak() {
                relative_whitespace_line -= 1;
                continue;
            }

            // if the line above ends in a new scope character, allow one more indent
            if line_above
                .excluding_linebreak()
                .ends_with(source.lang.config.new_scope_char.char())
            {
                break above_indent + 4;
            }

            // otherwise, allow up to the same indent as the line above
            break above_indent;
        };
        let indent = match source.lang.config.new_scope_char {
            // when scope is indent based, allow reducing scope when dragging
            NewScopeChar::Colon => ((coord.col / 4) * 4).min(allowed_indent),
            // when scope is brace based, only allow the maximum indent
            NewScopeChar::Brace => allowed_indent,
        };

        TextPoint::new(line, indent)
    }
}

/* ---------------------------- Helper Functions ---------------------------- */
fn block_for_point<'a>(blocks: &'a [Block], point: TextPoint, source: &Rope) -> Option<&'a Block> {
    let mut curr_block: Option<&Block> = None;
    let mut curr_level = blocks;
    'outer: while !curr_level.is_empty() {
        for block in curr_level {
            if block.text_range().contains(point, source) {
                // walk divider blocks but do not return them
                // dividers can also have nonsense columns walk them even if
                // the column is less than the point's
                if block.syntax_type != BlockType::Divider {
                    // check that the column of the block is less than the point's.
                    // this because the block's text range includes the indents but we want
                    // clicking on indents to select the scope above
                    if point.col < block.col {
                        continue;
                    }

                    curr_block = Some(block);
                }

                curr_level = &block.children;
                continue 'outer;
            }
        }
        break;
    }
    curr_block
}

/// Reduces the indent of the block such that the first line has no indent.
/// Assumes the indents of all lines are aligned.
fn normalize_indent(mut block: String) -> String {
    // set indent of string to the insertion point row
    // all lines after the first are indented relative to the first
    let existing_indent_count = block.chars().take_while(|c| c.is_whitespace()).count();
    let existing_indent = " ".repeat(existing_indent_count);

    // replace the indent at the start (which is not after a newline)
    block = block.replacen(&existing_indent, "", 1);

    // replace all other indents
    // works for both lf and crlf because they both end with lf
    block = block.replace(&format!("\n{}", existing_indent), "\n");

    block
}

/// Increases the indent of a *normalized* block so the first line is indented to new_indent_count
/// Assumes the indents of all lines are aligned.
fn set_indent(block: &str, new_indent_count: usize) -> String {
    // get the new indent
    let new_indent = " ".repeat(new_indent_count);
    let new_linebreak_indent = format!("\n{}", new_indent);

    // add indent to start of string
    let mut indented = format!("{}{}", new_indent, block);

    // replace all linebreaks with linebreak indents
    indented = indented.replace('\n', &new_linebreak_indent);

    // remove trailing indent
    if indented.ends_with(&new_linebreak_indent) {
        indented.replace_range((indented.len() - new_linebreak_indent.len()).., "\n");
    }

    indented
}
