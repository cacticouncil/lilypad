use druid::{
    Color, Data, Event, EventCtx, HotKey, KbKey, Lens, LifeCycle, Modifiers, MouseButton,
    MouseEvent, PaintCtx, Point, Rect, RenderContext, Size, SysMods, TimerToken, Widget,
};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;
use tree_sitter_c2rust::InputEdit;

use crate::parse::TreeManager;
use crate::vscode;

mod node;

//controls cursor blinking speed
static TIMER_INTERVAL: Duration = Duration::from_millis(700);

/*
Got these values by running:
    let font = FontDescriptor::new(FontFamily::new_unchecked("Roboto Mono")).with_size(15.0);
    let mut layout = TextLayout::<String>::from_text("A".to_string());
    layout.set_font(font);
    layout.rebuild_if_needed(ctx.text(), env);
    let size = layout.size();
    println!("{:}", size);
*/
pub const FONT_WIDTH: f64 = 9.0;
pub const FONT_HEIGHT: f64 = 20.0;

pub struct BlockEditor {
    tree_manager: Arc<RefCell<TreeManager>>,
    selection: Selection,
    mouse_pressed: bool,
    timer_id: TimerToken,
    cursor_visible: bool,
}

#[derive(Clone, Data, Lens)]
pub struct EditorModel {
    pub source: String,
}

impl BlockEditor {
    pub fn new() -> Self {
        BlockEditor {
            tree_manager: Arc::new(RefCell::new(TreeManager::new(""))),
            selection: Selection::ZERO,
            mouse_pressed: false,
            timer_id: TimerToken::INVALID,
            cursor_visible: true,
        }
    }

    /* ------- Drawing ------- */
    fn draw_blocks(&self, ctx: &mut PaintCtx, data: &EditorModel) {
        // pre-order traversal because we want to draw the parent under their children
        let tree_manager = self.tree_manager.borrow();
        let mut cursor = tree_manager.get_cursor();

        'outer: loop {
            // first time encountering the node, so draw it
            node::draw(cursor.node(), &data.source, ctx);
            // keep traveling down the tree as far as we can
            if cursor.goto_first_child() {
                continue;
            }

            // if we can't travel any further down, try the next sibling
            if cursor.goto_next_sibling() {
                continue;
            }

            // travel back up
            // loop until we reach the root or can go to the next sibling of a node again
            'inner: loop {
                // break outer if we reached the root
                if !cursor.goto_parent() {
                    break 'outer;
                }

                // if there is a sibling at this level, visit the sibling's subtree
                if cursor.goto_next_sibling() {
                    break 'inner;
                }
            }
        }
    }

    fn draw_cursor(&self, ctx: &mut PaintCtx) {
        // we want to draw the cursor where the mouse has last been (selection end)
        let block = Rect::from_origin_size(
            Point::new(
                (self.selection.end.x as f64) * FONT_WIDTH,
                (self.selection.end.y as f64) * FONT_HEIGHT,
            ),
            Size::new(2.0, FONT_HEIGHT),
        );
        if self.cursor_visible {
            //colors cursor
            ctx.fill(block, &Color::WHITE);
        }
    }

    fn draw_selection(&self, source: &str, ctx: &mut PaintCtx) {
        let start_y = self.selection.start.y;
        let end_y = self.selection.end.y;

        match end_y.cmp(&start_y) {
            Ordering::Greater => {
                // Forward selection, multiple lines
                // Fill first line from cursor to end
                selection_block(
                    self.selection.start.x,
                    self.selection.start.y,
                    line_len(self.selection.start.y, source) - self.selection.start.x,
                    ctx,
                );

                // fill in any in between lines
                for line in (start_y + 1)..end_y {
                    selection_block(0, line, line_len(line, source), ctx);
                }

                // Fill last line from the left until cursor
                selection_block(0, self.selection.end.y, self.selection.end.x, ctx);
            }
            Ordering::Less => {
                // Backwards selection, multiple lines

                // Fill first line from cursor to beginning
                selection_block(0, self.selection.start.y, self.selection.start.x, ctx);

                // fill in between lines
                for line in (end_y + 1)..start_y {
                    selection_block(0, line, line_len(line, source), ctx);
                }

                // Fill last line from the right until cursor
                selection_block(
                    self.selection.end.x,
                    self.selection.end.y,
                    line_len(self.selection.end.y, source) - self.selection.end.x,
                    ctx,
                );
            }
            Ordering::Equal => {
                // Just one line
                let ord_sel = self.selection.ordered();
                selection_block(
                    ord_sel.start.x,
                    ord_sel.start.y,
                    ord_sel.end.x - ord_sel.start.x,
                    ctx,
                );
            }
        }
    }

    /* ------------------------------ Interactions ------------------------------ */

    /* ------- Mouse ------- */
    fn mouse_clicked(&mut self, mouse: &MouseEvent, source: &str, ctx: &mut EventCtx) {
        // move the cursor and get selection start position
        let x = (mouse.pos.x / FONT_WIDTH).round() as usize;
        let y = (mouse.pos.y / FONT_HEIGHT) as usize;
        self.selection = Selection::new_cursor(clamp_col(y, x, source), y);
        ctx.request_paint();

        // request keyboard focus if not already focused
        if !ctx.is_focused() {
            ctx.request_focus();
        }

        // prevent another widget from also responding
        ctx.set_handled();
    }

    fn mouse_moved(&mut self, mouse: &MouseEvent, source: &str, ctx: &mut EventCtx) {
        // set selection end position to new position
        let x = (mouse.pos.x / FONT_WIDTH).round() as usize;
        let y = (mouse.pos.y / FONT_HEIGHT) as usize;
        self.selection.end = IntPoint::new(clamp_col(y, x, source), y);

        ctx.request_paint();

        // request keyboard focus if not already focused
        if !ctx.is_focused() {
            ctx.request_focus();
        }

        // prevent another widget from also responding
        ctx.set_handled();
    }

    /* ------- Text Editing ------- */
    fn send_vscode_edit(text: &str, range: Selection) {
        vscode::edited(text, range.start.y, range.start.x, range.end.y, range.end.x)
    }

    fn insert_str(&mut self, source: &mut String, add: &str) {
        // update source
        let old_selection = self.selection.ordered();
        let offsets = old_selection.offset_in(source);
        source.replace_range(offsets.to_range(), add);

        // move cursor
        self.selection = Selection::new_cursor(
            old_selection.start.x + add.chars().count(),
            old_selection.start.y,
        );

        // update tree
        let edits = InputEdit {
            start_byte: offsets.start,
            old_end_byte: offsets.end,
            new_end_byte: offsets.start + add.len(),
            start_position: old_selection.start.to_tree_sitter(),
            old_end_position: old_selection.end.to_tree_sitter(),
            new_end_position: self.selection.end.to_tree_sitter(),
        };
        self.tree_manager.borrow_mut().update(source, edits);

        // update vscode
        Self::send_vscode_edit(add, old_selection);
    }

    fn insert_newline(&mut self, source: &mut String) {
        // TODO: maintain indent level
        let old_selection = self.selection.ordered();

        // update source
        let offsets = old_selection.ordered().offset_in(source);
        source.replace_range(offsets.to_range(), os_linebreak());

        // move cursor
        self.selection = Selection::new_cursor(0, old_selection.start.y + 1);

        // update tree
        let edits = InputEdit {
            start_byte: offsets.start,
            old_end_byte: offsets.end,
            new_end_byte: offsets.start + os_linebreak().len(),
            start_position: old_selection.start.to_tree_sitter(),
            old_end_position: old_selection.end.to_tree_sitter(),
            new_end_position: self.selection.end.to_tree_sitter(),
        };
        self.tree_manager.borrow_mut().update(source, edits);

        // update vscode
        Self::send_vscode_edit(os_linebreak(), old_selection);
    }

    fn backspace(&mut self, source: &mut String) {
        let old_selection = self.selection.ordered();

        // for normal cursor, delete preceding character
        if old_selection.is_cursor() {
            // move cursor
            if old_selection.start.x == 0 {
                // abort if in position (0,0)
                if old_selection.start.y == 0 {
                    return;
                }

                // Move to the end of the line above.
                // Done before string modified so if a newline is deleted,
                // the cursor is sandwiched between the two newly joined lines.
                let above = old_selection.start.y - 1;
                self.selection = Selection::new_cursor(line_len(above, source), above);
            } else {
                // just move back one char
                self.selection =
                    Selection::new_cursor(old_selection.start.x - 1, old_selection.start.y);
            }

            // update source
            let offset = old_selection.start.offset_in(source);
            let removed = source.remove(offset - 1);

            // update tree
            let edits = InputEdit {
                start_byte: offset,
                old_end_byte: offset,
                new_end_byte: offset - removed.len_utf8(),
                start_position: old_selection.start.to_tree_sitter(),
                old_end_position: old_selection.start.to_tree_sitter(),
                new_end_position: self.selection.start.to_tree_sitter(),
            };
            self.tree_manager.borrow_mut().update(source, edits);

            // update vscode
            // FIXME: delete at start of line
            vscode::edited(
                "",
                old_selection.start.y,
                old_selection.start.x - 1,
                old_selection.start.y,
                old_selection.start.x,
            )
        }
        // for selection, delete text inside
        else {
            // set cursor to start of selection
            self.selection = Selection::new_cursor(old_selection.start.x, old_selection.start.y);

            // remove everything in range
            let offsets = old_selection.offset_in(source);
            source.replace_range(offsets.to_range(), "");

            // update tree
            let edits = InputEdit {
                start_byte: offsets.start,
                old_end_byte: offsets.end,
                new_end_byte: offsets.start,
                start_position: old_selection.start.to_tree_sitter(),
                old_end_position: old_selection.end.to_tree_sitter(),
                new_end_position: old_selection.start.to_tree_sitter(),
            };
            self.tree_manager.borrow_mut().update(source, edits);

            // update vscode
            Self::send_vscode_edit("", old_selection);
        }
    }

    /* ------- Cursor Movement ------- */
    fn cursor_up(&mut self, source: &str) {
        // when moving up, use top of selection
        let cursor_pos = self.selection.ordered().start;

        self.selection = if cursor_pos.y == 0 {
            Selection::new_cursor(0, 0)
        } else {
            // the normal text editor experience has a "memory" of how far right
            // the cursor started during a chain for arrow up/down (and then it snaps back there).
            // if that memory is implemented, it can replace self.cursor_pos.x
            Selection::new_cursor(
                clamp_col(cursor_pos.y - 1, cursor_pos.x, source),
                cursor_pos.y - 1,
            )
        }
    }

    fn cursor_down(&mut self, source: &str) {
        // when moving down use bottom of selection
        let cursor_pos = self.selection.ordered().end;

        let last_line = source.lines().count() - 1;
        let next_line = std::cmp::min(cursor_pos.y + 1, last_line);

        self.selection = if cursor_pos.y == last_line {
            // if on last line, just move to end of line
            Selection::new_cursor(
                source.lines().last().unwrap_or("").chars().count(),
                last_line,
            )
        } else {
            // same memory thing as above applies here
            Selection::new_cursor(clamp_col(next_line, cursor_pos.x, source), next_line)
        }
    }

    fn cursor_left(&mut self, source: &str) {
        if self.selection.is_cursor() {
            // actually move if cursor
            let cursor_pos = self.selection.start;
            if cursor_pos.x == 0 {
                // if at start of line, move to end of line above
                if cursor_pos.y != 0 {
                    self.selection =
                        Selection::new_cursor(line_len(cursor_pos.y - 1, source), cursor_pos.y - 1);
                }
            } else {
                self.selection = Selection::new_cursor(cursor_pos.x - 1, cursor_pos.y);
            }
        } else {
            // just move cursor to start of selection
            let start = self.selection.ordered().start;
            self.selection = Selection::new_cursor(start.x, start.y);
        }
    }

    fn cursor_right(&mut self, source: &str) {
        if self.selection.is_cursor() {
            // actually move if cursor
            let cursor_pos = self.selection.start;

            let curr_line_len = line_len(cursor_pos.y, source);
            if cursor_pos.x == curr_line_len {
                // if at end of current line, go to next line
                let last_line = source.lines().count() - 1;
                if cursor_pos.y != last_line {
                    self.selection = Selection::new_cursor(0, cursor_pos.y + 1);
                }
            } else {
                self.selection = Selection::new_cursor(cursor_pos.x + 1, cursor_pos.y);
            }
        } else {
            // just move cursor to end of selection
            let end = self.selection.ordered().end;
            self.selection = Selection::new_cursor(end.x, end.y);
        }
    }

    fn cursor_to_line_start(&mut self, source: &str) {
        // go with whatever line the mouse was last on
        let cursor_pos = self.selection.end;

        let line = source.lines().nth(cursor_pos.y).unwrap_or("");
        let start_idx = line.len() - line.trim_start().len();
        self.selection = Selection::new_cursor(start_idx, cursor_pos.y);
    }

    fn cursor_to_line_end(&mut self, source: &str) {
        // go with whatever line the mouse was last on
        let cursor_pos = self.selection.end;

        self.selection = Selection::new_cursor(line_len(cursor_pos.y, source), cursor_pos.y);
    }
}

fn clamp_col(row: usize, col: usize, source: &str) -> usize {
    std::cmp::min(col, line_len(row, source))
}

/// the number of characters in line of source
fn line_len(row: usize, source: &str) -> usize {
    source.lines().nth(row).unwrap_or("").chars().count()
}

impl Widget<EditorModel> for BlockEditor {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut EditorModel,
        _env: &druid::Env,
    ) {
        match event {
            Event::WindowConnected => {
                //starts initial timer
                self.timer_id = ctx.request_timer(TIMER_INTERVAL);
            }
            Event::Timer(id) => {
                if *id == self.timer_id {
                    //make cursor blink and then reset timer
                    //println!("timer done");
                    self.cursor_visible = !self.cursor_visible;
                    ctx.request_paint();
                    self.timer_id = ctx.request_timer(TIMER_INTERVAL);
                }
            }
            Event::MouseDown(mouse) if mouse.button == MouseButton::Left => {
                self.mouse_clicked(mouse, &data.source, ctx);
                self.mouse_pressed = true;
            }

            Event::MouseUp(mouse) if mouse.button == MouseButton::Left => {
                self.mouse_pressed = false;
            }

            Event::MouseMove(mouse) => {
                if self.mouse_pressed {
                    self.mouse_moved(mouse, &data.source, ctx);
                }
            }

            Event::KeyDown(key_event) => {
                // let VSCode handle hotkeys
                // TODO: hotkeys on native
                if key_event.mods.contains(Modifiers::META)
                    || key_event.mods.contains(Modifiers::CONTROL)
                {
                    return;
                }

                match &key_event.key {
                    // Text Inputs
                    KbKey::Backspace => self.backspace(&mut data.source),
                    KbKey::Enter => self.insert_newline(&mut data.source),
                    KbKey::Tab => self.insert_str(&mut data.source, "    "),
                    KbKey::Character(char) => self.insert_str(&mut data.source, char),

                    // Arrow Keys
                    KbKey::ArrowUp => self.cursor_up(&data.source),
                    KbKey::ArrowDown => self.cursor_down(&data.source),
                    KbKey::ArrowLeft => self.cursor_left(&data.source),
                    KbKey::ArrowRight => self.cursor_right(&data.source),

                    // Home and End buttons
                    KbKey::Home => self.cursor_to_line_start(&data.source),
                    KbKey::End => self.cursor_to_line_end(&data.source),

                    _ => {}
                }

                // redraw
                ctx.request_layout(); // probably should only conditionally do this
                ctx.request_paint();

                // prevent another widget from also responding
                ctx.set_handled();
            }

            Event::Command(command) => {
                // VSCode new text
                if let Some(new_text) = command.get(vscode::UPDATE_TEXT_SELECTOR) {
                    // update state and tree
                    data.source = new_text.clone();
                    self.tree_manager.borrow_mut().replace(&data.source);

                    ctx.set_handled();
                    ctx.request_layout();

                    // prevent another widget from also responding
                    ctx.set_handled()
                }
                // VSCode Copy/Cut/Paste
                else if let Some(_) = command.get(vscode::COPY_SELECTOR) {
                    let selection = self.selection.ordered().offset_in(&data.source);
                    let selected_text = data.source[selection.start..selection.end].to_string();
                    vscode::set_clipboard(selected_text);
                } else if let Some(_) = command.get(vscode::CUT_SELECTOR) {
                    // get selection
                    let selection = self.selection.ordered().offset_in(&data.source);
                    let selected_text = data.source[selection.start..selection.end].to_string();

                    // remove selection
                    self.insert_str(&mut data.source, "");

                    // return selection
                    vscode::set_clipboard(selected_text);
                } else if let Some(text) = command.get(vscode::PASTE_SELECTOR) {
                    self.insert_str(&mut data.source, &text)
                }
            }

            _ => (),
        }
    }

    fn update(
        &mut self,
        _ctx: &mut druid::UpdateCtx,
        _old_data: &EditorModel,
        data: &EditorModel,
        _env: &druid::Env,
    ) {
        // TODO: update the tree instead of replacing it every time
        // when does this fire???
        self.tree_manager.borrow_mut().replace(&data.source);
    }

    fn layout(
        &mut self,
        _ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &EditorModel,
        _env: &druid::Env,
    ) -> Size {
        let max_chars = data
            .source
            .lines()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0);
        let width = max_chars as f64 * FONT_WIDTH + (FONT_WIDTH * 4.0); // Setting the width of the window. May need to add a bit of a buffer (ex 4*width).
        let height = data.source.lines().count() as f64 * FONT_HEIGHT;
        let desired = Size { width, height };
        bc.constrain(desired)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditorModel, _env: &druid::Env) {
        // draw blocks
        self.draw_blocks(ctx, data);

        // draw cursor and selection
        self.draw_cursor(ctx);
        if !self.selection.is_cursor() {
            self.draw_selection(&data.source, ctx);
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        data: &EditorModel,
        _env: &druid::Env,
    ) {
        match event {
            // replace the tree with a tree for the initial source
            LifeCycle::WidgetAdded => self.tree_manager.borrow_mut().replace(&data.source),
            _ => (),
        }
    }
}

/* ------------------------------ helper types ------------------------------ */

struct Selection {
    start: IntPoint,
    end: IntPoint,
}

impl Selection {
    const ZERO: Self = Selection {
        start: IntPoint::ZERO,
        end: IntPoint::ZERO,
    };

    fn new(start: IntPoint, end: IntPoint) -> Self {
        Selection { start, end }
    }

    fn new_cursor(x: usize, y: usize) -> Self {
        Selection {
            start: IntPoint::new(x, y),
            end: IntPoint::new(x, y),
        }
    }

    fn is_cursor(&self) -> bool {
        self.start == self.end
    }

    fn ordered(&self) -> Selection {
        if self.start.y < self.end.y {
            Selection {
                start: self.start,
                end: self.end,
            }
        } else if self.start.y > self.end.y {
            Selection {
                start: self.end,
                end: self.start,
            }
        } else if self.start.x < self.end.x {
            Selection {
                start: self.start,
                end: self.end,
            }
        } else {
            Selection {
                start: self.end,
                end: self.start,
            }
        }
    }

    fn offset_in(&self, string: &str) -> TextRange {
        TextRange {
            start: self.start.offset_in(string),
            end: self.end.offset_in(string),
        }
    }
}

struct TextRange {
    start: usize,
    end: usize,
}

impl TextRange {
    fn to_range(&self) -> Range<usize> {
        self.start..self.end
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
struct IntPoint {
    x: usize,
    y: usize,
}

impl IntPoint {
    const ZERO: Self = IntPoint { x: 0, y: 0 };

    fn new(x: usize, y: usize) -> IntPoint {
        IntPoint { x, y }
    }

    fn to_tree_sitter(&self) -> tree_sitter_c2rust::Point {
        tree_sitter_c2rust::Point::new(self.x, self.y)
    }

    fn offset_in(&self, string: &str) -> usize {
        let mut offset: usize = 0;
        for (num, line) in string.lines().enumerate() {
            if num == self.y {
                // position in the current line
                // gets the byte offset of the cursor within the current line
                // (supports utf-8 characters)
                offset += line
                    .char_indices()
                    .nth(self.x)
                    .map(|x| x.0)
                    .unwrap_or(line.len());
                break;
            }

            offset += line.len() + os_linebreak().len(); // factor in the linebreak
        }
        offset
    }
}

fn selection_block(x: usize, y: usize, width: usize, ctx: &mut PaintCtx) {
    let block = Rect::from_origin_size(
        Point::new(x as f64 * FONT_WIDTH, y as f64 * FONT_HEIGHT),
        Size::new(width as f64 * FONT_WIDTH, FONT_HEIGHT),
    );
    ctx.fill(block, &Color::rgba(0.255, 0.255, 0.255, 0.5));
}

const fn os_linebreak() -> &'static str {
    if cfg!(target_os = "windows") {
        "\r\n"
    } else {
        "\n"
    }
}
