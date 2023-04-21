use std::sync::{Arc, Mutex};

use druid::{
    Event, KbKey, LifeCycle, Modifiers, MouseButton, PaintCtx, Point, Rect, RenderContext, Size,
    Widget,
};

use super::{
    block_drawer, gutter_drawer, text_range::TextRange, BlockEditor, EditorModel, FONT_HEIGHT,
    FONT_WIDTH, TIMER_INTERVAL,
};
use crate::{theme, vscode};

impl Widget<EditorModel> for BlockEditor {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut EditorModel,
        env: &druid::Env,
    ) {
        // first see if child handled it
        self.diagnostic_popup.event(ctx, event, data, env);
        if ctx.is_handled() {
            return;
        }

        match event {
            Event::WindowConnected => {
                //starts initial timer
                self.cursor_timer = ctx.request_timer(TIMER_INTERVAL);
            }
            Event::Timer(id) => {
                if *id == self.cursor_timer {
                    //make cursor blink and then reset timer
                    //println!("timer done");
                    self.cursor_visible = !self.cursor_visible;
                    ctx.request_paint();
                    self.cursor_timer = ctx.request_timer(TIMER_INTERVAL);
                }
            }
            Event::MouseDown(mouse) if mouse.button == MouseButton::Left => {
                self.mouse_clicked(mouse, &data.source.lock().unwrap(), ctx);
                self.mouse_pressed = true;
                ctx.set_handled();
            }

            Event::MouseUp(mouse) if mouse.button == MouseButton::Left => {
                self.mouse_pressed = false;
                ctx.request_paint();

                // diagnostic selection
                // TODO: change to a hover??
                // TODO: multiple diagnostics displayed at once
                if self.selection.is_cursor() {
                    let coord = self.mouse_to_coord(mouse, &data.source.lock().unwrap());
                    data.diagnostic_selection = None;
                    for diagnostic in &data.diagnostics {
                        if diagnostic.range.contains(coord) {
                            data.diagnostic_selection = Some(diagnostic.id);
                            break;
                        }
                    }
                } else {
                    data.diagnostic_selection = None
                }

                ctx.set_handled();
            }

            Event::MouseMove(mouse) => {
                if self.mouse_pressed {
                    self.mouse_dragged(mouse, &data.source.lock().unwrap(), ctx);
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
                    KbKey::Backspace => self.backspace(&mut data.source.lock().unwrap()),
                    KbKey::Enter => self.insert_newline(&mut data.source.lock().unwrap()),
                    KbKey::Tab => self.insert_str(&mut data.source.lock().unwrap(), "    "),
                    KbKey::Character(char) => {
                        self.insert_str(&mut data.source.lock().unwrap(), char)
                    }

                    // Arrow Keys
                    KbKey::ArrowUp => self.cursor_up(&data.source.lock().unwrap()),
                    KbKey::ArrowDown => self.cursor_down(&data.source.lock().unwrap()),
                    KbKey::ArrowLeft => self.cursor_left(&data.source.lock().unwrap()),
                    KbKey::ArrowRight => self.cursor_right(&data.source.lock().unwrap()),

                    // Home and End buttons
                    KbKey::Home => self.cursor_to_line_start(&data.source.lock().unwrap()),
                    KbKey::End => self.cursor_to_line_end(&data.source.lock().unwrap()),

                    _ => {}
                }

                // close any open popups
                data.diagnostic_selection = None;

                // redraw
                ctx.request_layout(); // probably should only conditionally do this
                ctx.request_paint();

                // prevent another widget from also responding
                ctx.set_handled();
            }

            Event::Command(command) => {
                // VSCode new text
                if let Some(new_text) = command.get(vscode::SET_TEXT_SELECTOR) {
                    // update state and tree
                    data.source = Arc::new(Mutex::new(new_text.clone()));
                    self.tree_manager.replace(new_text);

                    // reset cursor
                    self.selection = TextRange::ZERO;

                    // mark new text layout
                    self.text_changed = true;

                    ctx.request_layout();
                    ctx.request_paint();

                    ctx.set_handled();
                } else if let Some(edit) = command.get(vscode::APPLY_EDIT_SELECTOR) {
                    self.apply_edit(&mut data.source.lock().unwrap(), edit);
                    ctx.set_handled();
                }
                // VSCode Copy/Cut/Paste
                else if command.get(vscode::COPY_SELECTOR).is_some() {
                    let source = data.source.lock().unwrap();
                    let selection = self.selection.ordered().offset_in(&source);
                    let selected_text = source[selection.start..selection.end].to_string();
                    vscode::set_clipboard(selected_text);

                    ctx.set_handled();
                } else if command.get(vscode::CUT_SELECTOR).is_some() {
                    // get selection
                    let mut source = data.source.lock().unwrap();
                    let selection = self.selection.ordered().offset_in(&source);
                    let selected_text = source[selection.start..selection.end].to_string();

                    // remove selection
                    self.insert_str(&mut source, "");

                    // return selection
                    vscode::set_clipboard(selected_text);

                    ctx.set_handled()
                } else if let Some(text) = command.get(vscode::PASTE_SELECTOR) {
                    self.insert_str(&mut data.source.lock().unwrap(), text);
                    ctx.set_handled();
                }
                // VSCode Diagnostics
                else if let Some(diagnostics) = command.get(vscode::DIAGNOSTICS_SELECTOR) {
                    data.diagnostics = diagnostics.clone();

                    // this probably should be handled by the update function
                    // because diagnostics is in data
                    ctx.request_paint();

                    ctx.set_handled()
                }
            }

            _ => (),
        }
    }

    fn update(
        &mut self,
        ctx: &mut druid::UpdateCtx,
        _old_data: &EditorModel,
        data: &EditorModel,
        env: &druid::Env,
    ) {
        self.diagnostic_popup.update(ctx, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &EditorModel,
        env: &druid::Env,
    ) -> Size {
        // width is max between text and window
        let source = data.source.lock().unwrap();
        let max_chars = source.lines().map(|l| l.chars().count()).max().unwrap_or(0);
        let width = max_chars as f64 * FONT_WIDTH
            + super::OUTER_PAD
            + super::GUTTER_WIDTH
            + super::TEXT_L_PAD
            + 40.0; // extra space for nesting blocks

        // height is just height of text
        let height = super::line_count(&source) as f64 * FONT_HEIGHT
            + super::OUTER_PAD
            + self.padding.iter().sum::<f64>();

        let desired = Size { width, height };

        // add hover child
        let point = self.diagnostic_popup.widget().calc_origin(&self.padding);
        self.diagnostic_popup.set_origin(ctx, point);
        self.diagnostic_popup.layout(ctx, bc, data, env);

        bc.constrain(desired)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditorModel, env: &druid::Env) {
        // recompute cached objects if text changed
        if self.text_changed {
            let source = data.source.lock().unwrap();

            // get blocks
            let mut cursor = self.tree_manager.get_cursor();
            self.blocks = block_drawer::blocks_for_tree(&mut cursor);

            // get padding
            let line_count = super::line_count(&source);
            self.padding = block_drawer::make_padding(&self.blocks, line_count);

            // layout text
            self.text_drawer
                .layout(self.tree_manager.get_cursor().node(), &source, ctx);

            self.text_changed = false;
        }

        // draw background
        let bg_rect = Rect::from_origin_size(Point::ZERO, ctx.size());
        ctx.fill(bg_rect, &theme::BACKGROUND);

        // draw selection under text and blocks
        if !self.selection.is_cursor() {
            self.draw_selection(&data.source.lock().unwrap(), ctx);
        }

        // draw content
        block_drawer::draw_blocks(&self.blocks, ctx);
        self.text_drawer.draw(&self.padding, ctx);

        // draw diagnostics
        // TODO: draw higher priorities on top
        for diagnostic in &data.diagnostics {
            diagnostic.draw(&self.padding, ctx);
        }

        // draw diagnostic popup (if any)
        self.diagnostic_popup.paint(ctx, data, env);

        // draw gutter
        gutter_drawer::draw_line_numbers(&self.padding, self.selection.end.y, ctx);

        // draw cursor and selection
        self.draw_cursor(ctx);
    }

    fn lifecycle(
        &mut self,
        ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        data: &EditorModel,
        env: &druid::Env,
    ) {
        // replace the tree with a tree for the initial source
        if let LifeCycle::WidgetAdded = event {
            self.tree_manager.replace(&data.source.lock().unwrap())
        }

        // add diagnostic popup child
        self.diagnostic_popup.lifecycle(ctx, event, data, env);
    }
}
