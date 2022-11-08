use druid::{
    Color, Data, Event, EventCtx, HotKey, KbKey, Lens, LifeCycle, MouseEvent, PaintCtx, Point,
    Rect, RenderContext, Size, SysMods, Widget,
};
use std::cell::RefCell;
use std::env;
use std::sync::Arc;
use tree_sitter::InputEdit;

use crate::parse::TreeManager;

mod node;

/*
Got these values by running:
    let font = FontDescriptor::new(FontFamily::new_unchecked("Roboto Mono")).with_size(15.0);
    let mut layout = TextLayout::<String>::from_text("A".to_string());
    layout.set_font(font);
    layout.rebuild_if_needed(ctx.text(), env);
    let size = layout.size();
    println!("{:}", size);
*/
pub const FONT_WIDTH: f64 = 9.00146484375;
pub const FONT_HEIGHT: f64 = 20.0;
pub const OS: &str = env::consts::OS;

pub struct BlockEditor {
    tree_manager: Arc<RefCell<TreeManager>>,
    cursor_pos: IntPoint,
}

#[derive(Clone, Data, Lens)]
pub struct EditorModel {
    pub source: String,
}

impl BlockEditor {
    pub fn new() -> Self {
        BlockEditor {
            tree_manager: Arc::new(RefCell::new(TreeManager::new(""))),
            cursor_pos: IntPoint::ZERO,
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
        let block = Rect::from_origin_size(
            Point::new(
                (self.cursor_pos.x as f64) * FONT_WIDTH,
                (self.cursor_pos.y as f64) * FONT_HEIGHT,
            ),
            Size::new(2.0, FONT_HEIGHT),
        );
        ctx.fill(block, &Color::GREEN);
    }

    /* ------------------------------ Interactions ------------------------------ */

    /* ------- Mouse ------- */
    fn mouse_clicked(&mut self, mouse: &MouseEvent, ctx: &mut EventCtx) {
        // move the cursor
        let x = (mouse.pos.x / FONT_WIDTH).round() as usize;
        let y = (mouse.pos.y / FONT_HEIGHT) as usize;
        self.cursor_pos = IntPoint::new(x, y);
        ctx.request_paint();

        // request keyboard focus if not already focused
        if !ctx.is_focused() {
            ctx.request_focus();
        }

        // prevent another widget from also responding
        ctx.set_handled();
    }

    /* ------- Text Editing ------- */
    fn insert_str(&mut self, source: &mut String, add: &str) {
        // update source
        let offset = self.current_offset(source);
        source.insert_str(offset, add);

        // move cursor
        let start_pos = self.cursor_pos.to_tree_sitter();
        self.cursor_pos.x += add.chars().count();
        let end_pos = self.cursor_pos.to_tree_sitter();

        // update tree
        let edits = InputEdit {
            start_byte: offset,
            old_end_byte: offset,
            new_end_byte: offset + add.len(),
            start_position: start_pos,
            old_end_position: start_pos,
            new_end_position: end_pos,
        };
        self.tree_manager.borrow_mut().update(source, edits);
    }

    fn insert_newline(&mut self, source: &mut String) {
        // update source
        let offset = self.current_offset(source);
        if OS == "macos" {
            source.insert(offset, '\n');
        } else if OS == "windows" {
            source.insert_str(offset, "\r\n");
        } else {
            source.insert(offset, '\n');
        }

        // move cursor
        let start_pos = self.cursor_pos.to_tree_sitter();
        self.cursor_pos.x = 0;
        self.cursor_pos.y += 1;
        let end_pos = self.cursor_pos.to_tree_sitter();

        // update tree
        let edits = InputEdit {
            start_byte: offset,
            old_end_byte: offset,
            new_end_byte: offset + os_linebreak(OS),
            start_position: start_pos,
            old_end_position: start_pos,
            new_end_position: end_pos,
        };
        self.tree_manager.borrow_mut().update(source, edits);
    }

    fn backspace(&mut self, source: &mut String) {
        let offset = self.current_offset(source);

        // move cursor
        let start_pos = self.cursor_pos.to_tree_sitter();
        if self.cursor_pos.x == 0 {
            // abort if in position (0,0)
            if self.cursor_pos.y == 0 {
                return;
            }

            // Move to the end of the line above.
            // Done before string modified so if a newline is deleted,
            // the cursor is sandwiched between the two newly joined lines.
            self.cursor_pos.y -= 1;
            self.cursor_to_line_end(source);
        } else {
            self.cursor_pos.x -= 1;
        }
        let end_pos = self.cursor_pos.to_tree_sitter();

        // update source
        let removed = source.remove(offset - 1);

        // update tree
        let edits = InputEdit {
            start_byte: offset,
            old_end_byte: offset,
            new_end_byte: offset - removed.len_utf8(),
            start_position: start_pos,
            old_end_position: start_pos,
            new_end_position: end_pos,
        };
        self.tree_manager.borrow_mut().update(source, edits);
    }

    /* ------- Cursor Movement ------- */
    fn cursor_up(&mut self, source: &str) {
        if self.cursor_pos.y != 0 {
            self.cursor_pos.y -= 1;
            // the normal text editor experience has a "memory" of how far right
            // the cursor started during a chain for arrow up/down (and then it snaps back there).
            // if that memory is implemented, it can replace self.cursor_pos.x
            self.cursor_to_col(self.cursor_pos.x, source);
        }
    }

    fn cursor_down(&mut self, source: &str) {
        let next_line = self.cursor_pos.y + 1;
        let last_line = source.lines().count() - 1;
        self.cursor_pos.y = std::cmp::min(next_line, last_line);
        self.cursor_to_col(self.cursor_pos.x, source);
    }

    fn cursor_left(&mut self, source: &str) {
        if self.cursor_pos.x == 0 && self.cursor_pos.y != 0 {
            self.cursor_pos.y -= 1;
            self.cursor_to_line_end(source)
        } else {
            self.cursor_pos.x -= 1;
        }
    }

    fn cursor_right(&mut self, source: &str) {
        // go to next line if at end of current line
        let curr_line_len = source.lines().nth(self.cursor_pos.y).unwrap_or("").len();
        if self.cursor_pos.x == curr_line_len {
            let last_line = source.lines().count() - 1;
            if self.cursor_pos.y != last_line {
                self.cursor_pos.x = 0;
                self.cursor_pos.y += 1;
            }
        } else {
            self.cursor_pos.x += 1;
        }
    }

    fn cursor_to_line_end(&mut self, source: &str) {
        self.cursor_pos.x = source.lines().nth(self.cursor_pos.y).unwrap_or("").len();
    }

    /// goes to column without going past end of line
    fn cursor_to_col(&mut self, col: usize, source: &str) {
        let curr_line_len = source.lines().nth(self.cursor_pos.y).unwrap_or("").len();
        self.cursor_pos.x = std::cmp::min(col, curr_line_len);
    }

    /* ------- Helpers ------- */

    /// get byte offset from current cursor location
    fn current_offset(&self, source: &str) -> usize {
        let mut offset: usize = 0;
        for (num, line) in source.lines().enumerate() {
            if num == self.cursor_pos.y {
                // position in the current line
                // gets the byte offset of the cursor within the current line
                // (supports utf-8 characters)
                offset += line
                    .char_indices()
                    .nth(self.cursor_pos.x)
                    .map(|x| x.0)
                    .unwrap_or(line.len());
                break;
            }

            offset += line.len() + os_linebreak(OS); // + 1 for the \n at the end of the line
        }
        offset
    }
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
            Event::MouseDown(mouse) => self.mouse_clicked(mouse, ctx),

            Event::KeyDown(key_event) => {
                match key_event {
                    // Hotkeys
                    key if HotKey::new(SysMods::Cmd, "c").matches(key) => {
                        println!("example copy command")
                    }

                    // Normal typing
                    key => {
                        match &key.key {
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

                            _ => {}
                        }
                    }
                }

                // redraw
                ctx.request_layout(); // probably should only conditionally do this
                ctx.request_paint();

                // prevent another widget from also responding
                ctx.set_handled()
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
        self.tree_manager.borrow_mut().replace(&data.source);
    }

    fn layout(
        &mut self,
        _ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &EditorModel,
        _env: &druid::Env,
    ) -> Size {
        let max_chars = data.source.lines().map(|l| l.len()).max().unwrap_or(0);
        let width = max_chars as f64 * FONT_WIDTH + (FONT_WIDTH * 4.0); // Setting the width of the window. May need to add a bit of a buffer (ex 4*width).
        let height = data.source.lines().count() as f64 * FONT_HEIGHT;
        let desired = Size { width, height };
        bc.constrain(desired)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditorModel, _env: &druid::Env) {
        // draw background
        let background = Rect::from_origin_size(Point::ZERO, ctx.size());
        ctx.fill(background, &Color::rgb(0.0, 0.4, 0.4));

        // draw blocks
        self.draw_blocks(ctx, data);

        //draw cursor
        self.draw_cursor(ctx);
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

struct IntPoint {
    x: usize,
    y: usize,
}

impl IntPoint {
    const ZERO: IntPoint = IntPoint { x: 0, y: 0 };

    fn new(x: usize, y: usize) -> IntPoint {
        IntPoint { x, y }
    }

    fn to_tree_sitter(&self) -> tree_sitter::Point {
        tree_sitter::Point::new(self.x, self.y)
    }
}

// Determine linebreak size ("\r\n" vs '\n') based on OS
fn os_linebreak(os: &str) -> usize {
    match os {
        "windows" => 2,
        "macos" => 1,
        "linux" => 1,
        _ => 0,
    }
}
