use std::sync::{Arc, Mutex};

use druid::{
    text::TextAction, Event, LifeCycle, Menu, MouseButton, PaintCtx, Point, Rect, RenderContext,
    Size, Widget,
};
use ropey::Rope;

use super::{
    block_drawer, gutter_drawer, text_range::TextRange, BlockEditor, EditorModel,
    APPLY_EDIT_SELECTOR, FONT_HEIGHT, FONT_WIDTH, SET_FILE_NAME_SELECTOR, TIMER_INTERVAL,
};
use crate::{lang::lang_for_file, theme, vscode, GlobalModel};

impl Widget<EditorModel> for BlockEditor {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut EditorModel,
        env: &druid::Env,
    ) {
        // first see if a child handled it
        self.diagnostic_popup.event(ctx, event, data, env);
        self.completion_popup.event(ctx, event, data, env);
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
                    // blink cursor and set new timer
                    self.cursor_visible = !self.cursor_visible;
                    self.cursor_timer = ctx.request_timer(TIMER_INTERVAL);
                    ctx.request_paint();
                    ctx.set_handled();
                }
            }
            Event::MouseDown(mouse) => {
                match mouse.button {
                    MouseButton::Left => {
                        self.mouse_clicked(mouse, &data.source.lock().unwrap(), ctx);
                        self.mouse_pressed = true;
                    }
                    MouseButton::Right => {
                        // move mouse to right click if not a selection
                        // TODO: also check if the click was inside of the selection
                        if self.selection.is_cursor() {
                            self.mouse_clicked(mouse, &data.source.lock().unwrap(), ctx);
                        }

                        // custom menus do not work for druid on web
                        // need to do them via javascript externally
                        if cfg!(not(target_family = "wasm")) {
                            let menu = Menu::<GlobalModel>::empty()
                                .entry(druid::platform_menus::common::cut())
                                .entry(druid::platform_menus::common::copy())
                                .entry(druid::platform_menus::common::paste());
                            ctx.show_context_menu(menu, mouse.pos);
                        }
                    }
                    _ => {}
                };

                // clear any current completion
                self.completion_popup.widget_mut().clear();

                ctx.set_handled();
            }
            Event::MouseUp(mouse) if mouse.button == MouseButton::Left => {
                self.mouse_pressed = false;
                ctx.request_paint();
                ctx.set_handled();
            }

            Event::MouseMove(mouse) => {
                if self.mouse_pressed {
                    self.mouse_dragged(mouse, &data.source.lock().unwrap(), ctx);
                    ctx.request_paint();
                    ctx.set_handled();
                } else {
                    // diagnostic selection
                    // TODO: delay to pop up
                    // TODO: multiple diagnostics displayed at once
                    if self.selection.is_cursor() {
                        let coord = self.mouse_to_raw_coord(mouse.pos);
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
                }

                // when cursor is inside of editor, have it be in text edit mode
                ctx.set_cursor(&druid::Cursor::IBeam);
            }

            Event::KeyDown(_) => {
                // manually pass key events to completion popup so it can intercept them before ime
                self.completion_popup
                    .widget_mut()
                    .event(ctx, event, data, env);
            }

            Event::ImeStateChange => {
                let mut source = data.source.lock().unwrap();

                // clear any current completion
                self.completion_popup.widget_mut().clear();

                // apply action
                let action = self.ime.borrow_mut().take_external_action();
                if let Some(action) = action {
                    match action {
                        TextAction::InsertNewLine { .. } => self.insert_newline(&mut source),
                        TextAction::InsertTab { .. } => self.indent(&mut source),
                        TextAction::InsertBacktab => self.unindent(&mut source),
                        TextAction::Delete(mov) => {
                            self.backspace(&mut source, mov);
                            self.completion_popup
                                .widget_mut()
                                .request_completions(&source, self.selection);
                        }
                        TextAction::Move(mov) => self.move_cursor(mov, &source),
                        TextAction::MoveSelecting(mov) => self.move_selecting(mov, &source),
                        _ => crate::console_log!("unexpected external action '{:?}'", action),
                    };
                }

                // apply text change
                let text_change = self.ime.borrow_mut().take_external_text_change();
                if let Some(text_change) = text_change {
                    self.insert_str(&mut source, &text_change);
                    self.completion_popup
                        .widget_mut()
                        .request_completions(&source, self.selection);
                }

                // cursor has moved, so close diagnostics popup
                data.diagnostic_selection = None;

                // redraw
                ctx.request_layout();
                ctx.request_paint();

                // prevent another widget from also responding
                ctx.set_handled();
            }

            Event::Command(command) => {
                // VSCode new text
                if let Some(new_text) = command.get(vscode::SET_TEXT_SELECTOR) {
                    // update state and tree
                    let rope = Rope::from_str(new_text);
                    data.source = Arc::new(Mutex::new(rope));
                    self.tree_manager.replace(&data.source.lock().unwrap());

                    // reset view properties
                    self.selection = TextRange::ZERO;
                    self.pseudo_selection = None;
                    self.input_ignore_stack.clear();
                    self.paired_delete_stack.clear();

                    // mark new text layout
                    self.text_changed = true;

                    ctx.request_layout();
                    ctx.request_paint();

                    ctx.set_handled();
                } else if let Some(edit) = command.get(vscode::APPLY_VSCODE_EDIT_SELECTOR) {
                    self.apply_vscode_edit(&mut data.source.lock().unwrap(), edit);
                    ctx.set_handled();
                }
                // New file name from the native file picker
                else if let Some(file_name) = command.get(SET_FILE_NAME_SELECTOR) {
                    let new_lang = lang_for_file(file_name);
                    if self.language.name != new_lang.name {
                        self.language = new_lang;
                        self.text_drawer.change_language(new_lang);
                        self.tree_manager.change_language(new_lang);
                    }
                    ctx.set_handled();
                }
                // Copy, Cut, & (VSCode) Paste
                else if command.get(druid::commands::COPY).is_some() {
                    // get selected text
                    let source = data.source.lock().unwrap();
                    let selection = self.selection.ordered().char_range_in(&source);
                    let selected_text = source.slice(selection).to_string();

                    // set to platform's clipboard
                    if cfg!(target_family = "wasm") {
                        vscode::set_clipboard(selected_text);
                    } else {
                        druid::Application::global()
                            .clipboard()
                            .put_string(selected_text);
                    }

                    ctx.set_handled();
                } else if command.get(druid::commands::CUT).is_some() {
                    // get selection
                    let mut source = data.source.lock().unwrap();
                    let selection = self.selection.ordered().char_range_in(&source);
                    let selected_text = source.slice(selection).to_string();

                    // delete current selection
                    self.insert_str(&mut source, "");

                    // set to platform's clipboard
                    if cfg!(target_family = "wasm") {
                        vscode::set_clipboard(selected_text);
                    } else {
                        druid::Application::global()
                            .clipboard()
                            .put_string(selected_text);
                    }

                    ctx.set_handled()
                } else if let Some(clip_text) = command.get(vscode::PASTE_SELECTOR) {
                    // paste from vscode provides string
                    self.insert_str(&mut data.source.lock().unwrap(), clip_text);
                    ctx.set_handled();
                }
                // VSCode Diagnostics
                else if let Some(diagnostics) = command.get(vscode::SET_DIAGNOSTICS_SELECTOR) {
                    data.diagnostics = diagnostics.clone();

                    // this probably should be handled by the update function
                    // because diagnostics is in data
                    ctx.request_paint();

                    ctx.set_handled()
                }
                // Applying an edit from elsewhere (that's not VSCode)
                else if let Some(edit) = command.get(APPLY_EDIT_SELECTOR) {
                    self.apply_edit(&mut data.source.lock().unwrap(), edit);
                    ctx.set_handled();
                }
            }

            Event::Paste(clipboard) => {
                let clip_text = clipboard.get_string().unwrap_or_default();
                self.insert_str(&mut data.source.lock().unwrap(), &clip_text);
                ctx.set_handled();
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
        self.completion_popup.update(ctx, data, env);
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
        let width = max_chars as f64 * FONT_WIDTH.get().unwrap()
            + super::OUTER_PAD
            + super::GUTTER_WIDTH
            + super::TEXT_L_PAD
            + 40.0; // extra space for nesting blocks

        // height is just height of text
        let height = source.len_lines() as f64 * FONT_HEIGHT.get().unwrap()
            + super::OUTER_PAD
            + self.padding.iter().sum::<f64>()
            + 200.0; // extra space for over-scroll

        let desired = Size { width, height };

        // add diagnostic popup
        let point = self.diagnostic_popup.widget().calc_origin(&self.padding);
        self.diagnostic_popup.set_origin(ctx, point);
        self.diagnostic_popup.layout(ctx, bc, data, env);

        // add completion popup
        let point = self
            .completion_popup
            .widget()
            .calc_origin(&self.padding, self.selection.start);
        self.completion_popup.set_origin(ctx, point);
        self.completion_popup.layout(ctx, bc, data, env);

        bc.constrain(desired)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditorModel, env: &druid::Env) {
        let source = data.source.lock().unwrap();

        // recompute cached objects if text changed
        if self.text_changed {
            // get blocks
            let mut cursor = self.tree_manager.get_cursor();
            self.blocks = block_drawer::blocks_for_tree(&mut cursor, self.language);

            // get padding
            let line_count = source.len_lines();
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
        self.draw_pseudo_selection(&source, ctx);
        self.draw_selection(&source, ctx);

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

        // draw completion popup (if any)
        self.completion_popup.paint(ctx, data, env);

        // draw gutter
        gutter_drawer::draw_line_numbers(&self.padding, self.selection.end.row, ctx);

        // draw cursor
        if ctx.has_focus() {
            self.draw_cursor(ctx);
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        data: &EditorModel,
        env: &druid::Env,
    ) {
        match event {
            LifeCycle::WidgetAdded => {
                // register as text input
                ctx.register_text_input(self.ime.ime_handler());

                // find font dimensions if not already found
                if FONT_WIDTH.get().is_none() {
                    super::find_font_dimensions(ctx, env);
                }
            }
            LifeCycle::BuildFocusChain => {
                // make the view a focus target
                ctx.register_for_focus();
            }
            _ => {}
        }

        // pass lifecycle events to children
        self.diagnostic_popup.lifecycle(ctx, event, data, env);
        self.completion_popup.lifecycle(ctx, event, data, env);
    }
}
