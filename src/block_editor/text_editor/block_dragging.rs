use std::{borrow::Cow, collections::HashMap, sync::Arc};

use druid::{
    text::{Direction, Movement},
    EventCtx, PaintCtx, Point, Rect, RenderContext,
};
use ropey::Rope;

use super::{TextEdit, TextEditor};
use crate::{
    block_editor::{
        block_drawer::Block,
        rope_ext::RopeSliceExt,
        text_range::{TextPoint, TextRange},
        BlockType, DragSession, FONT_HEIGHT, FONT_WIDTH, GUTTER_WIDTH, OUTER_PAD,
    },
    lang::NewScopeChar,
    theme, vscode,
};

impl TextEditor {
    pub fn start_block_drag(
        &mut self,
        mouse_pos: Point,
        source: &mut Rope,
        drag_block: &mut Option<Arc<DragSession>>,
        ctx: &mut EventCtx,
    ) {
        let cursor_pos = self.mouse_to_coord(mouse_pos, source);

        if let Some(block) = block_for_point(&self.blocks, cursor_pos, source) {
            let mut text_range = block.text_range();

            vscode::log_event(
                "editor-block-drag",
                HashMap::from([
                    ("type", block.syntax_type.to_str()),
                    ("lang", self.language.name),
                ]),
            );

            // select the whole first line (to get all the indent)
            text_range.start.col = 0;

            // normalize the text
            let char_range = text_range.char_range_in(source);
            let mut block_text = source.slice(char_range.clone()).to_string();
            block_text = normalize_indent(block_text);
            if !block_text.ends_with('\n') {
                // add a newline to the end if it doesn't have one
                block_text.push('\n');
            }

            // offset the dragging popup so it matches where the mouse picked up the block
            let block_corner = self.coord_to_mouse(TextPoint {
                col: block.col,
                row: block.line,
            });
            let relative_pos =
                Point::new(mouse_pos.x - block_corner.x, mouse_pos.y - block_corner.y);

            // set dragging popup
            *drag_block = Some(Arc::new(DragSession {
                text: block_text.clone(),
                offset: relative_pos,
            }));

            // set the insertion point to where the block initially was
            self.drag_insertion_line = Some(TextPoint {
                col: block.col,
                row: block.line,
            });

            // remove dragged block from source
            self.apply_edit(source, &TextEdit::delete(text_range));

            // re-layout the dragging popup
            ctx.children_changed();
        }
    }

    pub fn drop_block(
        &mut self,
        source: &mut Rope,
        drag_block: &mut Option<Arc<DragSession>>,
    ) -> bool {
        // note: using take() also sets to None
        if let (Some(drag_block), Some(insertion_point)) =
            (drag_block.take(), self.drag_insertion_line.take())
        {
            let indented_text = set_indent(&drag_block.text, insertion_point.col);

            // apply edit
            let insert_point = TextPoint {
                col: 0,
                row: insertion_point.row,
            };
            let edit = TextEdit::new(
                Cow::Owned(indented_text),
                TextRange::new_cursor(insert_point),
            );
            self.apply_edit(source, &edit);

            // move the cursor from the line after the block to the end of the text
            self.move_cursor(Movement::Grapheme(Direction::Upstream), source);

            true
        } else {
            false
        }
    }

    pub fn draw_dropping_line(&self, ctx: &mut PaintCtx) {
        const THICKNESS: f64 = 4.0;

        if let Some(insertion_line) = self.drag_insertion_line {
            let font_height = *FONT_HEIGHT.get().unwrap();
            let font_width = *FONT_WIDTH.get().unwrap();

            let line_padding_above = self.padding.iter().take(insertion_line.row).sum::<f64>();
            let y = (insertion_line.row as f64) * font_height + OUTER_PAD + line_padding_above;
            let x = (insertion_line.col as f64) * font_width + OUTER_PAD + GUTTER_WIDTH;
            let rect = Rect::new(x, y, ctx.size().width - OUTER_PAD, y + THICKNESS);
            ctx.fill(rect, &theme::CURSOR);
        }
    }

    pub fn set_dropping_line(&mut self, mouse_pos: Point, source: &Rope) {
        // find the point adjusted so that it is based around between lines
        let adj_pos = druid::Point::new(
            mouse_pos.x,
            mouse_pos.y + (FONT_HEIGHT.get().unwrap() / 2.0),
        );
        let coord = self.mouse_to_raw_coord(adj_pos);

        // clamp row to end of source
        let row = coord.row.min(source.len_lines() - 1);

        // limit to the first non-empty above line's level
        // (or 1 more if it ends in a new scope character)
        let mut relative_whitespace_row = row;
        let allowed_indent = loop {
            if relative_whitespace_row == 0 {
                break 0;
            } else {
                let line_above = source.line(relative_whitespace_row - 1);
                let above_indent = line_above.whitespace_at_start();

                if above_indent == line_above.len_chars_no_linebreak() {
                    // if the line above is entirely whitespace, move to the line above
                    relative_whitespace_row -= 1;
                    continue;
                } else if line_above
                    .excluding_linebreak()
                    .ends_with(self.language.new_scope_char.char())
                {
                    // if the line above ends in a new scope character, allow one more indent
                    break above_indent + 4;
                } else {
                    // otherwise, allow up to the same indent as the line above
                    break above_indent;
                }
            }
        };
        let indent = match self.language.new_scope_char {
            // when scope is indent based, allow reducing scope when dragging
            NewScopeChar::Colon => ((coord.col / 4) * 4).min(allowed_indent),
            // when scope is brace based, only allow the maximum indent
            NewScopeChar::Brace => allowed_indent,
        };

        self.drag_insertion_line = Some(TextPoint { col: indent, row });
    }
}

/* ---------------------------- Helper Functions ---------------------------- */
fn block_for_point<'a>(
    blocks: &'a Vec<Block>,
    point: TextPoint,
    source: &Rope,
) -> Option<&'a Block> {
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
