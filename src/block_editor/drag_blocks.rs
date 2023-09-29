use druid::{
    piet::PietText,
    text::{Direction, Movement},
    EventCtx, PaintCtx, Point, Rect, RenderContext, Size, Widget,
};
use ropey::Rope;

use crate::{
    lang::{LanguageConfig, NewScopeChar},
    parse::TreeManager,
    theme,
};

use super::{
    block_drawer::{self, Block, BlockType},
    rope_ext::RopeSliceExt,
    text_drawer::TextDrawer,
    text_range::{TextEdit, TextPoint, TextRange},
    BlockEditor, EditorModel, FONT_HEIGHT, FONT_WIDTH,
};

/* ---------------------------- Editor Functions ---------------------------- */

impl BlockEditor {
    pub fn start_block_drag(&mut self, mouse_pos: Point, source: &mut Rope, ctx: &mut EventCtx) {
        let cursor_pos = self.mouse_to_coord(mouse_pos, source);

        if let Some(block) = block_for_point(&self.blocks, cursor_pos, source) {
            let mut text_range = block.text_range();

            // select the whole first line (to get all the indent)
            text_range.start.col = 0;

            // set the dragged block to the text
            let char_range = text_range.char_range_in(source);
            let mut block_text = source.slice(char_range.clone()).to_string();
            block_text = normalize_indent(block_text);
            if !block_text.ends_with('\n') {
                // add a newline to the end if it doesn't have one
                block_text.push('\n');
            }
            self.drag_block = Some(block_text.clone());

            // set the dragging popup text
            self.dragging_popup
                .widget_mut()
                .set_text(block_text, ctx.text());

            // offset the dragging popup so it matches where the mouse picked up the block
            let block_corner = self.coord_to_mouse(TextPoint {
                col: block.col,
                row: block.line,
            });
            let relative_pos =
                Point::new(mouse_pos.x - block_corner.x, mouse_pos.y - block_corner.y);
            self.dragging_popup
                .widget_mut()
                .set_mouse_pos_in_block(relative_pos);

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

    pub fn drop_block(&mut self, source: &mut Rope) -> bool {
        // note: using take() also sets to None
        if let (Some(mut drag_block), Some(insertion_point)) =
            (self.drag_block.take(), self.drag_insertion_line.take())
        {
            drag_block = set_indent(drag_block, insertion_point.col);

            // apply edit
            let insert_point = TextPoint {
                col: 0,
                row: insertion_point.row,
            };
            let edit = TextEdit {
                range: TextRange::new_cursor(insert_point),
                text: drag_block,
            };
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
            let y =
                (insertion_line.row as f64) * font_height + super::OUTER_PAD + line_padding_above;
            let x =
                (insertion_line.col as f64) * font_width + super::OUTER_PAD + super::GUTTER_WIDTH;
            let rect = Rect::new(x, y, ctx.size().width - super::OUTER_PAD, y + THICKNESS);
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

/* ----------------------------- Dragging Popup ----------------------------- */
pub struct DraggingPopup {
    block_text: String,
    mouse_pos_in_block: Point,

    tree_manager: TreeManager,
    text_drawer: TextDrawer,
    language: &'static LanguageConfig,
    blocks: Vec<Block>,
    padding: Vec<f64>,
}

impl DraggingPopup {
    pub fn new(lang: &'static LanguageConfig) -> Self {
        Self {
            block_text: String::new(),
            mouse_pos_in_block: Point::ZERO,
            tree_manager: TreeManager::new(lang),
            text_drawer: TextDrawer::new(lang),
            language: lang,
            blocks: vec![],
            padding: vec![],
        }
    }

    pub fn change_language(&mut self, lang: &'static LanguageConfig) {
        self.tree_manager.change_language(lang);
        self.text_drawer.change_language(lang);
        self.language = lang;
        self.blocks.clear();
        self.padding.clear();
    }

    pub fn set_text(&mut self, text: String, piet_text: &mut PietText) {
        self.block_text = text.clone();

        let rope = Rope::from(text);
        self.tree_manager.replace(&rope);
        self.text_drawer
            .layout(self.tree_manager.get_cursor().node(), &rope, piet_text);

        self.blocks = block_drawer::blocks_for_tree(
            &mut self.tree_manager.get_cursor(),
            &rope,
            self.language,
        );
        self.padding = block_drawer::make_padding(&self.blocks, rope.len_lines());
    }

    pub fn set_mouse_pos_in_block(&mut self, pos: Point) {
        self.mouse_pos_in_block = pos;
    }

    pub fn calc_origin(&self, mouse: Point) -> Point {
        Point {
            x: mouse.x - self.mouse_pos_in_block.x,
            y: mouse.y - self.mouse_pos_in_block.y,
        }
    }
}

impl Widget<EditorModel> for DraggingPopup {
    fn event(
        &mut self,
        _ctx: &mut druid::EventCtx,
        _event: &druid::Event,
        _data: &mut EditorModel,
        _env: &druid::Env,
    ) {
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
    ) -> druid::Size {
        // width is just the text
        let max_chars = self
            .block_text
            .lines()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0);
        const EXTRA_PAD_FOR_NESTING: f64 = 40.0;
        let width = max_chars as f64 * FONT_WIDTH.get().unwrap() + EXTRA_PAD_FOR_NESTING;

        // height is just height of text
        let height = self.block_text.lines().count() as f64 * FONT_HEIGHT.get().unwrap()
            + self.padding.iter().sum::<f64>();

        Size { width, height }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &EditorModel, _env: &druid::Env) {
        // draw background (transparent so you can see where you are dropping it)
        let rect = ctx.size().to_rect();
        ctx.fill(rect, &theme::BACKGROUND.with_alpha(0.75));

        // draw content
        block_drawer::draw_blocks(&self.blocks, Point::ZERO, ctx);
        self.text_drawer.draw(&self.padding, Point::ZERO, ctx)
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

/// Reduces the indent of the block such that the first line has no indent
fn normalize_indent(mut block: String) -> String {
    // set indent of string to the insertion point row
    // all lines after the first are indented relative to the first
    let existing_indent_count = block.chars().take_while(|c| c.is_whitespace()).count();
    let existing_indent = " ".repeat(existing_indent_count);

    // replace the indent at the start (which is not after a newline)
    block = block.replacen(&existing_indent, "", 1);

    // replace all other indents
    block = block.replace(&format!("\n{}", existing_indent), "\n");

    block
}

/// Increases the indent of a *normalized* block so the first line is indented to new_indent_count
fn set_indent(mut block: String, new_indent_count: usize) -> String {
    // get the new indent
    let new_indent = " ".repeat(new_indent_count);
    let new_linebreak_indent = format!("\n{}", new_indent);

    // add indent to start of string
    block = format!("{}{}", new_indent, block);

    // replace all linebreaks with linebreak indents
    block = block.replace('\n', &new_linebreak_indent);

    // remove trailing indent
    if block.ends_with(&new_linebreak_indent) {
        block.replace_range((block.len() - new_linebreak_indent.len()).., "\n");
    }

    block
}
