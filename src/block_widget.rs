use druid::piet::{Text, TextLayoutBuilder};
use druid::{
    Color, Data, Event, FontFamily, Lens, LifeCycle, PaintCtx, Point, Rect, RenderContext, Size,
    Widget,
};
use std::cell::RefCell;
use std::sync::Arc;
use tree_sitter::Node;

use crate::parse::TreeManager;

pub struct BlockEditor {
    tree_manager: Arc<RefCell<TreeManager>>,
}

#[derive(Clone, Data, Lens)]
pub struct EditorModel {
    pub source: String,
}

impl BlockEditor {
    pub fn new() -> Self {
        BlockEditor {
            tree_manager: Arc::new(RefCell::new(TreeManager::new(""))),
        }
    }

    fn draw_blocks(&self, ctx: &mut PaintCtx, data: &EditorModel) {
        // pre-order traversal because we want to draw the parent under their children
        let tree_manager = self.tree_manager.borrow();
        let mut cursor = tree_manager.get_cursor();

        'outer: loop {
            // first time encountering the node, so draw it
            draw_node(cursor.node(), &data.source, ctx);
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
}

impl Widget<EditorModel> for BlockEditor {
    fn event(
        &mut self,
        _ctx: &mut druid::EventCtx,
        event: &druid::Event,
        _data: &mut EditorModel,
        _env: &druid::Env,
    ) {
        match event {
            Event::MouseDown(mouse) => {
                println!("click at {}", mouse.pos);
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
        let width = max_chars as f64 * FONT_WIDTH;
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

/*
Got these values by running:
    let font = FontDescriptor::new(FontFamily::new_unchecked("Roboto Mono")).with_size(15.0);
    let mut layout = TextLayout::<String>::from_text("A".to_string());
    layout.set_font(font);
    layout.rebuild_if_needed(ctx.text(), env);
    let size = layout.size();
    println!("{:}", size);
*/
const FONT_WIDTH: f64 = 9.00146484375;
const FONT_HEIGHT: f64 = 20.0;

fn draw_node(node: Node, source: &str, ctx: &mut PaintCtx) {
    // don't draw boxes for nodes that are just string literals
    // without this every space would get it's own color
    if !node.is_named() {
        return;
    }

    // get color/see if this node should be drawn
    // don't draw boxes for nodes that aren't high level
    let color = match color(&node) {
        Some(color) => color,
        None => return,
    };

    let start = node.start_position();
    let end = node.end_position();

    let start_pt = Point::new(
        (start.column as f64) * FONT_WIDTH,
        (start.row as f64) * FONT_HEIGHT,
    );

    let mut margin: f64 = 0.0;

    // Check that node is high level and then determine the margin based on tabbing
    if check_node_name(&node) {
        margin = (start_pt.x)
            + (((node.start_position().column as f64 / 4.0) + 1.0) * (1.0 * FONT_WIDTH));
    }

    let size = {
        if start.row == end.row {
            // if block is all on one row, then
            Size::new(ctx.size().width - margin, FONT_HEIGHT)
        } else {
            // if block is across rows
            let height = ((end.row - start.row + 1) as f64) * FONT_HEIGHT;
            // Fill entire screen width with block for module node
            if node.kind() == "module" {
                Size::new(ctx.size().width, height)
            } else {
                Size::new((ctx.size().width) - margin, height)
            }
        }
    };

    // draw the block in
    let block = Rect::from_origin_size(start_pt, size);
    ctx.fill(block, &color);
    // draw text in
    draw_text(node, source, ctx, start_pt.x, start_pt.y);
}

fn color(node: &Node) -> Option<Color> {
    match node.kind() {
        "class_definition" => Some(Color::rgb(0.9, 0.43, 0.212)),
        "function_definition" => Some(Color::rgb(0.0, 0.47, 0.47)),
        "import_statement" => Some(Color::rgb(0.77, 0.176, 0.188)),
        "expression_statement" => Some(Color::rgb(0.5, 0.2, 0.5)),
        "while_statement" => Some(Color::rgb(0.305, 0.0, 0.305)),
        "if_statement" => Some(Color::rgb(0.502, 0.086, 0.22)),
        "else_clause" => Some(Color::rgb(0.502, 0.086, 0.22)),
        "break_statement" => Some(Color::rgb(0.5, 0.2, 0.5)),
        "for_statement" => Some(Color::rgb(0.305, 0.0, 0.305)),
        "try_statement" => Some(Color::rgb(0.502, 0.086, 0.22)),
        "except_clause" => Some(Color::rgb(0.502, 0.086, 0.22)),
        "finally_clause" => Some(Color::rgb(0.502, 0.086, 0.22)),
        "elif_clause" => Some(Color::rgb(0.502, 0.086, 0.22)),
        "comment" => Some(Color::rgb(0.0, 0.4, 0.4)),
        "continue_statement" => Some(Color::rgb(0.5, 0.2, 0.5)),
        //"pair" => Some(Color::rgb(0.502, 0.086, 0.22)),
        _ => None,
    }
}

fn draw_text(node: Node, source: &str, ctx: &mut PaintCtx, start_x: f64, start_y: f64) {
    let source_line = get_first_node_line(node, source);

    let text = ctx.text();
    let layout = text
        .new_text_layout(source_line)
        .font(FontFamily::new_unchecked("Roboto Mono"), 15.0)
        .text_color(Color::WHITE)
        .build()
        .unwrap();
    ctx.draw_text(&layout, (start_x, start_y));
}

fn get_first_node_line(node: Node, source: &str) -> String {
    source[node.byte_range()]
        .lines()
        .next()
        .unwrap()
        .to_string()
}

fn check_node_name(node: &Node) -> bool {
    let node_names = [
        "class_definition",
        "function_definition",
        "import_statement",
        "expression_statement",
        "while_statement",
        "if_statement",
        "else_clause",
        "break_statement",
        "for_statement",
        "try_statement",
        "except_clause",
        "finally_clause",
        "elif_clause",
        "comment",
        "continue_statement",
    ];

    node_names.contains(&node.kind())
}
