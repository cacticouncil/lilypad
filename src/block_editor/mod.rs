use druid::{Data, Lens, TimerToken};
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;

use crate::parse::TreeManager;

mod block;
mod drawing;
mod interactions;
mod lifecycle;
mod selection;
mod text_drawer;

use selection::*;
use text_drawer::*;

//controls cursor blinking speed
pub const TIMER_INTERVAL: Duration = Duration::from_millis(700);

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
    text_drawer: TextDrawer,
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
            text_drawer: TextDrawer::new(),
        }
    }
}

/// the number of characters in line of source
fn line_len(row: usize, source: &str) -> usize {
    source.lines().nth(row).unwrap_or("").chars().count()
}
