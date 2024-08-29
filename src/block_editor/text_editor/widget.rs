use egui::{
    os::OperatingSystem, output::IMEOutput, scroll_area::ScrollBarVisibility, CursorIcon, Event,
    EventFilter, ImeEvent, Key, Modifiers, Rect, Response, ScrollArea, Sense, Ui, Vec2, Widget,
};
use ropey::Rope;
use std::{collections::HashSet, ops::RangeInclusive};

use super::{
    gutter_drawer,
    text_editing::{HDir, HUnit, TextMovement, VDir, VUnit},
    undo_manager::UndoStopCondition,
    TextEdit, TextEditor, TextPoint,
};
use crate::{
    block_editor::{
        block_drawer, text_range::TextRange, DragSession, ExternalCommand, MonospaceFont,
        GUTTER_WIDTH, OUTER_PAD, TEXT_L_PAD, TOTAL_TEXT_X_OFFSET,
    },
    lang::lang_for_file,
    theme::{self, blocks_theme::BlocksTheme},
};

const EVENT_FILTER: EventFilter = EventFilter {
    horizontal_arrows: true,
    vertical_arrows: true,
    tab: true,
    escape: true,
};

impl TextEditor {
    pub fn widget<'a>(
        &'a mut self,
        drag_block: &'a mut Option<DragSession>,
        external_commands: &'a [ExternalCommand],
        blocks_theme: BlocksTheme,
        font: &'a MonospaceFont,
    ) -> impl Widget + 'a {
        move |ui: &mut Ui| -> egui::Response {
            ScrollArea::both()
                .auto_shrink([false; 2])
                .scroll_bar_visibility(ScrollBarVisibility::VisibleWhenNeeded)
                .id_source("text_editor")
                .drag_to_scroll(false)
                .show_viewport(ui, |ui, viewport| {
                    // allocate space
                    let content_size = self.content_size(viewport, font);
                    let expanded_size = content_size.max(ui.available_size() - Vec2::new(0.0, 5.0));
                    let (auto_id, rect) = ui.allocate_space(expanded_size);

                    // setup interactivity
                    let sense = Sense::click_and_drag();
                    let mut response = ui.interact(rect, auto_id, sense);
                    response.fake_primary_click = false;
                    ui.memory_mut(|mem| mem.set_focus_lock_filter(auto_id, EVENT_FILTER));

                    // find the offset to the content
                    let offset = rect.min.to_vec2();

                    // note the start of the frame
                    self.frame_start_time = ui.input(|i| i.time);

                    // handle interactions
                    let drop_point =
                        self.handle_pointer(ui, offset, &mut response, drag_block, font);
                    self.handle_external_commands(external_commands);
                    self.event(ui);
                    self.update_text_if_needed();
                    // TODO: if the selection moved out of view, scroll to it

                    // draw the text editor
                    let cursor_rect = self.draw(
                        offset,
                        content_size.x,
                        viewport,
                        response.has_focus(),
                        drop_point,
                        blocks_theme,
                        font,
                        ui,
                    );

                    // draw completion popup
                    // TODO: have the draw function return the cursor rect and use that here
                    if self.completion_popup.has_completions() {
                        let mut completion_edit: Option<TextEdit> = None;
                        ui.put(
                            Rect::from_min_size(
                                self.completion_popup.calc_origin(
                                    self.selection.start,
                                    &self.padding,
                                    font,
                                ) + offset,
                                self.completion_popup.calc_size(font),
                            ),
                            self.completion_popup
                                .widget(&mut completion_edit, &self.source, font),
                        );
                        if let Some(edit) = completion_edit {
                            self.apply_edit(&edit, UndoStopCondition::Always, true);
                            self.completion_popup.clear();
                            response.request_focus();
                        }
                    }

                    // draw diagnostic popup
                    if let Some(diagnostic_selection) = self.diagnostic_selection {
                        let diagnostic = &self.diagnostics[diagnostic_selection];
                        ui.put(
                            Rect::from_min_size(
                                self.diagnostic_popup.calc_origin(
                                    diagnostic,
                                    offset,
                                    &self.padding,
                                    font,
                                ),
                                self.diagnostic_popup.calc_size(diagnostic, font),
                            ),
                            self.diagnostic_popup.widget(diagnostic, font),
                        );
                    }

                    // Set IME output (in screen coords)
                    if let Some(cursor_rect) = cursor_rect {
                        let transform = ui
                            .memory(|m| m.layer_transforms.get(&ui.layer_id()).copied())
                            .unwrap_or_default();

                        ui.ctx().output_mut(|o| {
                            o.ime = Some(IMEOutput {
                                rect: transform * rect,
                                cursor_rect: transform * cursor_rect,
                            });
                        });
                    }

                    response
                })
                .inner
        }
    }

    /* --------------------------------- drawing -------------------------------- */
    /// Draws the contents of the editor and returns the rect of the cursor
    fn draw(
        &self,
        offset: Vec2,
        content_width: f32,
        viewport: Rect,
        has_focus: bool,
        block_drop_point: Option<TextPoint>,
        blocks_theme: BlocksTheme,
        font: &MonospaceFont,
        ui: &Ui,
    ) -> Option<Rect> {
        // draw background
        let painter = ui.painter();
        painter.rect_filled(painter.clip_rect(), 0.0, theme::BACKGROUND);

        // draw selection under text and blocks
        self.draw_pseudo_selection(offset, font, painter);
        self.draw_selection(offset, font, painter);

        // find which lines are visible in the viewport
        let visible_lines = self.visible_lines(viewport, font);

        // draw text and blocks
        let block_padding = Vec2::new(OUTER_PAD + GUTTER_WIDTH, OUTER_PAD);
        let block_offset = block_padding + offset;
        block_drawer::draw_blocks(
            &self.blocks,
            block_offset,
            content_width - (block_padding.x + OUTER_PAD),
            Some(visible_lines.clone()),
            blocks_theme,
            font,
            painter,
        );
        let text_padding = Vec2::new(TOTAL_TEXT_X_OFFSET, OUTER_PAD);
        let text_offset = text_padding + offset;
        self.text_drawer.draw(
            &self.padding,
            text_offset,
            Some(visible_lines),
            font,
            painter,
        );

        // draw drag & drop insertion line
        if let Some(drop_point) = block_drop_point {
            self.draw_dropping_line(drop_point, content_width, offset, font, painter);
        }

        // draw diagnostic underlines
        // TODO: draw higher priorities on top
        for diagnostic in &self.diagnostics {
            diagnostic.draw(&self.padding, &self.source, offset, font, painter);
        }

        // draw gutter if in frame
        if offset.x > -1.0 * GUTTER_WIDTH {
            let gutter_offset = Vec2::new(0.0, OUTER_PAD) + offset;
            gutter_drawer::draw_line_numbers(
                &self.padding,
                gutter_offset,
                self.selection.end.line,
                &self.breakpoints,
                self.stack_frame,
                font,
                painter,
            );
        }

        // draw cursor
        if has_focus {
            Some(self.draw_cursor(offset, font, ui))
        } else {
            None
        }
    }

    fn update_text_if_needed(&mut self) {
        if self.text_changed {
            // get blocks
            let mut cursor = self.tree_manager.get_cursor();
            self.blocks = block_drawer::blocks_for_tree(&mut cursor, &self.source, self.language);

            // get padding
            let line_count = self.source.len_lines();
            self.padding = block_drawer::make_padding(&self.blocks, line_count);

            // highlight text
            self.text_drawer
                .highlight(self.tree_manager.get_cursor().node(), &self.source);

            self.text_changed = false;
        }
    }

    /// Find the range of any lines that are visible (even partially) in the viewport
    fn visible_lines(&self, viewport: Rect, font: &MonospaceFont) -> RangeInclusive<usize> {
        let mut top_line = 0;
        let mut bottom_line: Option<usize> = None;

        let mut current_height = OUTER_PAD;
        for (i, padding) in self.padding.iter().enumerate() {
            if current_height < viewport.min.y {
                top_line = i;
            }

            current_height += padding + font.size.y;

            if current_height > viewport.max.y {
                bottom_line = Some(i);
                break;
            }
        }

        // if the text ends within the viewport, just go until the end of the text
        let bottom_line = bottom_line.unwrap_or_else(|| self.padding.len() - 1);

        top_line..=bottom_line
    }

    /* --------------------------------- events --------------------------------- */
    /// Returns the drop point if a block is being dragged
    fn handle_pointer(
        &mut self,
        ui: &Ui,
        offset: Vec2,
        response: &mut Response,
        dragged_block: &mut Option<DragSession>,
        font: &MonospaceFont,
    ) -> Option<TextPoint> {
        let mods = ui.input(|i| i.modifiers);

        if response.hovered() {
            if mods.alt {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            } else {
                ui.ctx().set_cursor_icon(CursorIcon::Text);
            }

            ui.output_mut(|o| o.mutable_text_under_cursor = true);

            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let pos = pointer_pos - offset;

                // handle mouse down
                let is_being_dragged = ui.ctx().is_being_dragged(response.id);
                if is_being_dragged {
                    if dragged_block.is_none() {
                        self.expand_selection(pos, font);
                    }
                } else if ui.input(|i| i.pointer.primary_pressed()) {
                    self.mouse_clicked(pos, mods, dragged_block, font);
                    self.completion_popup.clear();
                    response.request_focus();
                }

                // handle mouse up
                if dragged_block.is_some() {
                    let mouse_released = ui.input(|i| i.pointer.primary_released());
                    if mouse_released {
                        let drop_point = self.find_drop_point(pos, font);
                        self.drop_block(dragged_block, drop_point);
                        response.request_focus();
                    }
                }
            }
        }

        // find diagnostic under cursor
        // TODO: delay to pop up
        // TODO: multiple diagnostics displayed at once
        // TODO: r-tree??
        if response.hovered() {
            if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                let coord = self.mouse_to_raw_coord(pointer_pos - offset, font);
                self.diagnostic_selection = self
                    .diagnostics
                    .iter()
                    .position(|d| d.range.contains(coord, &self.source));
            }
        };

        // figure out the block drop points
        if response.contains_pointer() && dragged_block.is_some() {
            ui.input(|i| i.pointer.latest_pos())
                .map(|pointer_pos| self.find_drop_point(pointer_pos - offset, font))
        } else {
            None
        }
    }

    fn event(&mut self, ui: &Ui) {
        let mut events = ui.input(|i| i.filtered_events(&EVENT_FILTER));

        if self.ime_enabled {
            Self::remove_ime_incompatible_events(&mut events);
            // Process IME events first:
            events.sort_by_key(|e| !matches!(e, Event::Ime(_)));
        }

        let os = ui.ctx().os();
        for event in &events {
            match event {
                Event::Copy => {
                    let selection = self.selection.ordered().char_range_in(&self.source);
                    let selected_text = self.source.slice(selection).to_string();
                    ui.ctx().copy_text(selected_text);
                }

                Event::Cut => {}

                Event::Paste(new_text) => self.insert_str(new_text),

                Event::Text(new_text) => {
                    self.insert_char(new_text);
                    self.completion_popup
                        .request_completions(&self.source, self.selection)
                }

                Event::Key {
                    modifiers,
                    key,
                    pressed: true,
                    ..
                } => {
                    if !self.handle_selection_modifying_keypress(modifiers, *key) {
                        self.handle_text_modifying_keypress(os, modifiers, *key);
                    }
                }

                Event::Ime(ime_event) => self.handle_ime(ime_event),

                _ => {}
            };
        }
    }

    fn handle_external_commands(&mut self, commands: &[ExternalCommand]) {
        for command in commands {
            match command {
                ExternalCommand::SetText(new_text) => {
                    // update state and tree
                    let rope = Rope::from_str(new_text);
                    self.tree_manager.replace(&rope);
                    self.source = rope;

                    // reset view properties
                    self.selection = TextRange::ZERO;
                    self.pseudo_selection = None;
                    self.input_ignore_stack.clear();
                    self.paired_delete_stack.clear();

                    // mark new text layout
                    self.text_changed = true;
                }
                ExternalCommand::SetFileName(new_name) => {
                    let new_lang = lang_for_file(new_name);
                    if self.language.name != new_lang.name {
                        self.language = new_lang;
                        self.text_drawer.change_language(new_lang);
                        self.tree_manager.change_language(new_lang);
                    }
                }
                ExternalCommand::ApplyEdit(edit) => {
                    self.apply_edit(edit, UndoStopCondition::Always, true)
                }
                ExternalCommand::InsertText(new_text) => {
                    self.insert_str(new_text);
                }
                ExternalCommand::SetDiagnostics(new_diagnostics) => {
                    self.diagnostics = new_diagnostics.clone();
                    self.diagnostic_popup.clear_fixes();
                    self.diagnostic_selection = None;
                }
                ExternalCommand::SetQuickFix(id, fixes) => {
                    self.diagnostic_popup.set_fixes(*id, fixes.clone());
                }
                ExternalCommand::SetCompletions(new_completions) => {
                    self.completion_popup
                        .set_completions(new_completions, &self.source);
                }
                ExternalCommand::SetBreakpoints(new_breakpoints) => {
                    let mut set = HashSet::new();
                    for bp in new_breakpoints {
                        set.insert(*bp);
                    }
                    self.breakpoints = set;
                }
                ExternalCommand::SetStackFrame(new_stack_frame) => {
                    self.stack_frame = *new_stack_frame;
                }
                ExternalCommand::Undo => {
                    self.undo();
                }
                ExternalCommand::Redo => {
                    self.redo();
                }
                _ => {}
            }
        }
    }

    fn remove_ime_incompatible_events(events: &mut Vec<Event>) {
        // Remove key events which cause problems while 'IME' is being used.
        // See https://github.com/emilk/egui/pull/4509
        events.retain(|event| {
            !matches!(
                event,
                Event::Key { repeat: true, .. }
                    | Event::Key {
                        key: Key::Backspace
                            | Key::ArrowUp
                            | Key::ArrowDown
                            | Key::ArrowLeft
                            | Key::ArrowRight,
                        ..
                    }
            )
        });
    }

    fn handle_text_modifying_keypress(
        &mut self,
        os: OperatingSystem,
        modifiers: &Modifiers,
        key: Key,
    ) {
        match key {
            // Basic actions
            Key::Enter => {
                if self.completion_popup.has_completions() {
                    self.completion_popup.trigger_completion();
                } else {
                    self.insert_newline();
                    self.completion_popup.clear();
                }
            }
            Key::Tab => {
                if self.completion_popup.has_completions() {
                    self.completion_popup.trigger_completion();
                } else {
                    if modifiers.shift {
                        self.unindent();
                    } else {
                        self.indent();
                    }
                    self.completion_popup.clear();
                }
            }
            Key::Backspace => {
                if modifiers.mac_cmd {
                    self.backspace(TextMovement::horizontal(HUnit::Line, HDir::Left));
                } else if modifiers.alt || modifiers.ctrl {
                    // alt on mac, ctrl on windows
                    self.backspace(TextMovement::horizontal(HUnit::Word, HDir::Left));
                } else {
                    self.backspace(TextMovement::horizontal(HUnit::Grapheme, HDir::Left));
                };
                self.completion_popup
                    .request_completions(&self.source, self.selection)
            }
            Key::Delete if !modifiers.shift || os != OperatingSystem::Windows => {
                if modifiers.mac_cmd {
                    self.backspace(TextMovement::horizontal(HUnit::Line, HDir::Right));
                } else if modifiers.alt || modifiers.ctrl {
                    // alt on mac, ctrl on windows
                    self.backspace(TextMovement::horizontal(HUnit::Word, HDir::Right));
                } else {
                    self.backspace(TextMovement::horizontal(HUnit::Grapheme, HDir::Right));
                };
                self.completion_popup
                    .request_completions(&self.source, self.selection)
            }

            // Undo/Redo
            Key::Z if modifiers.matches_logically(Modifiers::COMMAND) => {
                if modifiers.shift {
                    self.redo();
                } else {
                    self.undo();
                }
                self.completion_popup.clear();
            }
            Key::Y if modifiers.matches_logically(Modifiers::COMMAND) => {
                self.redo();
                self.completion_popup.clear();
            }

            // Control hotkeys
            Key::H if modifiers.ctrl => {
                self.backspace(TextMovement::horizontal(HUnit::Grapheme, HDir::Left));
                self.completion_popup.clear();
            }

            Key::K if modifiers.ctrl => {
                self.backspace(TextMovement::horizontal(HUnit::Line, HDir::Right));
                self.completion_popup.clear();
            }

            Key::U if modifiers.ctrl => {
                self.backspace(TextMovement::horizontal(HUnit::Line, HDir::Left));
                self.completion_popup.clear();
            }

            Key::W if modifiers.ctrl => {
                if self.selection.is_cursor() {
                    self.backspace(TextMovement::horizontal(HUnit::Word, HDir::Left));
                } else {
                    self.delete_selection();
                };
                self.completion_popup.clear();
            }
            _ => {}
        }
    }

    fn handle_ime(&mut self, ime_event: &ImeEvent) {
        match ime_event {
            ImeEvent::Enabled => {
                self.ime_enabled = true;
                self.ime_selection = self.selection;
            }
            ImeEvent::Preedit(text_mark) => {
                if text_mark != "\n" && text_mark != "\r" {
                    // Empty prediction can be produced when user press backspace
                    // or escape during IME, so we clear current text.
                    self.delete_selection();
                    let start_cursor = self.selection.start;
                    if !text_mark.is_empty() {
                        self.insert_str(text_mark);
                    }
                    self.ime_selection = self.selection;
                    self.selection.start = start_cursor;
                }
            }
            ImeEvent::Commit(prediction) => {
                if prediction != "\n" && prediction != "\r" {
                    self.ime_enabled = false;

                    if !prediction.is_empty() && self.selection.end == self.ime_selection.end {
                        self.delete_selection();
                        self.insert_str(prediction);
                    } else {
                        self.selection = TextRange::new_cursor(self.selection.start);
                    }
                }
            }
            ImeEvent::Disabled => {
                self.ime_enabled = false;
            }
        }
    }

    pub fn handle_selection_modifying_keypress(&mut self, modifiers: &Modifiers, key: Key) -> bool {
        match key {
            Key::A if modifiers.command => {
                self.selection = TextRange::ZERO.expanded_by(
                    TextMovement::vertical(VUnit::Document, VDir::Down),
                    &self.source,
                );
                self.completion_popup.clear();
                true
            }

            Key::ArrowLeft | Key::ArrowRight => {
                let direction = match key {
                    Key::ArrowLeft => HDir::Left,
                    Key::ArrowRight => HDir::Right,
                    _ => unreachable!(),
                };

                let unit = if modifiers.command {
                    HUnit::Line
                } else if modifiers.alt {
                    HUnit::Word
                } else {
                    HUnit::Grapheme
                };

                let movement = TextMovement::horizontal(unit, direction);

                if modifiers.shift {
                    self.move_selecting(movement);
                } else {
                    self.move_cursor(movement);
                }

                self.completion_popup.clear();

                true
            }

            Key::ArrowUp | Key::ArrowDown | Key::Home | Key::End => {
                if self.completion_popup.has_completions() {
                    if key == Key::ArrowUp {
                        self.completion_popup.select_prev();
                    } else if key == Key::ArrowDown {
                        self.completion_popup.select_next();
                    }
                } else {
                    let direction = match key {
                        Key::ArrowUp => VDir::Up,
                        Key::ArrowDown => VDir::Down,
                        Key::Home => VDir::Up,
                        Key::End => VDir::Down,
                        _ => unreachable!(),
                    };

                    let unit = if modifiers.command || Key::Home == key || Key::End == key {
                        VUnit::Document
                    } else {
                        VUnit::Line
                    };

                    let movement = TextMovement::vertical(unit, direction);

                    if modifiers.shift {
                        self.move_selecting(movement);
                    } else {
                        self.move_cursor(movement);
                    }

                    self.completion_popup.clear();
                }

                true
            }

            Key::Escape => {
                self.completion_popup.clear();
                true
            }

            _ => false,
        }
    }

    /* --------------------------------- helpers -------------------------------- */
    fn content_size(&self, viewport: Rect, font: &MonospaceFont) -> Vec2 {
        // width is max between text and window
        let max_chars = self
            .source
            .lines()
            .map(|l| l.len_chars())
            .max()
            .unwrap_or(0);
        let max_line_len =
            max_chars as f32 * font.size.x + (OUTER_PAD * 2.0) + GUTTER_WIDTH + TEXT_L_PAD + 40.0; // extra space for nesting blocks
        let width = f32::max(viewport.width(), max_line_len);

        // height is just height of text
        let height = self.source.len_lines() as f32 * font.size.y
            + OUTER_PAD
            + self.padding.iter().sum::<f32>()
            + 200.0; // extra space for over-scroll

        Vec2::new(width, height)
    }
}
