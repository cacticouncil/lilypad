use druid::{Event, KbKey, LifeCycle, Modifiers, MouseButton, PaintCtx, Size, Widget};

use super::{block_drawer, BlockEditor, EditorModel, FONT_HEIGHT, FONT_WIDTH, TIMER_INTERVAL};
use crate::vscode;

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
                ctx.set_handled();
            }

            Event::MouseUp(mouse) if mouse.button == MouseButton::Left => {
                self.mouse_pressed = false;
                ctx.request_paint();
                ctx.set_handled();
            }

            Event::MouseMove(mouse) => {
                if self.mouse_pressed {
                    self.mouse_dragged(mouse, &data.source, ctx);
                    ctx.request_paint();
                    ctx.set_handled();
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

                    // mark new text layout
                    self.text_changed = true;

                    ctx.set_handled();
                    ctx.request_layout();

                    // prevent another widget from also responding
                    ctx.set_handled()
                }
                // VSCode Copy/Cut/Paste
                else if command.get(vscode::COPY_SELECTOR).is_some() {
                    let selection = self.selection.ordered().offset_in(&data.source);
                    let selected_text = data.source[selection.start..selection.end].to_string();
                    vscode::set_clipboard(selected_text);
                } else if command.get(vscode::CUT_SELECTOR).is_some() {
                    // get selection
                    let selection = self.selection.ordered().offset_in(&data.source);
                    let selected_text = data.source[selection.start..selection.end].to_string();

                    // remove selection
                    self.insert_str(&mut data.source, "");

                    // return selection
                    vscode::set_clipboard(selected_text);
                } else if let Some(text) = command.get(vscode::PASTE_SELECTOR) {
                    self.insert_str(&mut data.source, text)
                }
            }

            _ => (),
        }
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
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &EditorModel,
        _env: &druid::Env,
    ) -> Size {
        // width is max between text and window
        let max_chars = data
            .source
            .lines()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0);
        let text_width =
            max_chars as f64 * FONT_WIDTH + super::OUTER_PAD + super::TEXT_L_PAD + 40.0;
        let window_width = ctx.window().get_size().width;
        let width = f64::max(text_width, window_width);

        // height is just height of text
        let height = data.source.lines().count() as f64 * FONT_HEIGHT
            + super::OUTER_PAD
            + self.padding.iter().sum::<f64>();
        let desired = Size { width, height };

        bc.constrain(desired)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditorModel, _env: &druid::Env) {
        // recompute cached objects if text changed
        if self.text_changed {
            // get blocks
            let tree_manager = self.tree_manager.borrow();
            let mut cursor = tree_manager.get_cursor();
            self.blocks = block_drawer::blocks_for_tree(&mut cursor);

            // get padding
            let line_count = &data.source.lines().count();
            self.padding = block_drawer::make_padding(&self.blocks, *line_count);

            // layout text
            self.text_drawer.layout(&data.source, ctx);

            self.text_changed = false;
        }

        // draw selection under text and blocks
        if !self.selection.is_cursor() {
            self.draw_selection(&data.source, ctx);
        }

        // draw blocks
        block_drawer::draw_blocks(&self.blocks, ctx);

        // draw text on top of blocks
        self.text_drawer.draw(&self.padding, ctx);

        // draw cursor and selection
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
