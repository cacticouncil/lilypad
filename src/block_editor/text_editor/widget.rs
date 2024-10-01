use egui::{
    output::IMEOutput, scroll_area::ScrollBarVisibility, CursorIcon, Event, EventFilter, ImeEvent,
    Key, Modifiers, Pos2, Rect, Response, ScrollArea, Sense, Ui, Vec2, Widget,
};
use std::{collections::HashSet, ops::RangeInclusive};

use super::{
    coord_conversions::pt_to_unbounded_text_coord, gutter::Gutter, TextEdit, TextEditor, TextPoint,
};
use crate::{
    block_editor::{
        block_drawer,
        source::{Source, UndoStopCondition},
        text_range::{
            movement::{HDir, HUnit, TextMovement, VDir, VUnit},
            TextRange,
        },
        DragSession, ExternalCommand, MonospaceFont, GUTTER_WIDTH, OUTER_PAD, TEXT_L_PAD,
        TOTAL_TEXT_X_OFFSET,
    },
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
        source: &'a mut Source,
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
                    let content_size = self.content_size(source, viewport, font);
                    let expanded_size = content_size.max(ui.available_size() - Vec2::new(0.0, 5.0));
                    let (auto_id, rect) = ui.allocate_space(expanded_size);

                    // setup interactivity
                    let sense = Sense::click_and_drag();
                    let mut response = ui.interact(rect, auto_id, sense);
                    response.fake_primary_click = false;
                    ui.memory_mut(|mem| mem.set_focus_lock_filter(auto_id, EVENT_FILTER));

                    // find the offset to the content
                    let offset = rect.min.to_vec2();

                    // set the frame start time in the completion popup (for the cursor blink)
                    self.selections.set_frame_start_time(ui.input(|i| i.time));

                    // handle interactions
                    let drop_point =
                        self.handle_pointer(ui, offset, &mut response, drag_block, source, font);
                    self.handle_external_commands(external_commands, source);
                    self.event(source, ui);
                    self.update_text_if_needed(source);
                    // TODO: if the selection moved out of view, scroll to it

                    // draw the text editor
                    let cursor_rect = self.draw(
                        offset,
                        content_size.x,
                        viewport,
                        response.has_focus(),
                        drop_point,
                        source,
                        blocks_theme,
                        font,
                        ui,
                    );

                    // draw gutter if in frame
                    if offset.x > -1.0 * GUTTER_WIDTH {
                        ui.put(
                            Rect::from_min_size(
                                Pos2::new(0.0, OUTER_PAD) + offset,
                                Vec2::new(GUTTER_WIDTH, content_size.y),
                            ),
                            Gutter::new(
                                self.selections.selection().end.line,
                                &mut self.breakpoints,
                                self.stack_frame,
                                &self.padding,
                                source.text(),
                                font,
                            ),
                        );
                    }

                    // draw completion popup
                    // TODO: use the cursor rect from the draw function as the origin
                    if self.completion_popup.has_completions() {
                        let mut completion_edit: Option<TextEdit> = None;
                        ui.put(
                            Rect::from_min_size(
                                self.completion_popup.calc_origin(
                                    self.selections.selection().start,
                                    &self.padding,
                                    font,
                                ) + offset,
                                self.completion_popup.calc_size(font),
                            ),
                            self.completion_popup
                                .widget(&mut completion_edit, source.text(), font),
                        );
                        if let Some(edit) = completion_edit {
                            source.apply_edit(
                                &edit,
                                UndoStopCondition::Always,
                                true,
                                &mut self.selections,
                            );
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
        source: &Source,
        blocks_theme: BlocksTheme,
        font: &MonospaceFont,
        ui: &Ui,
    ) -> Option<Rect> {
        // draw background
        let painter = ui.painter();
        painter.rect_filled(painter.clip_rect(), 0.0, theme::BACKGROUND);

        // draw selection under text and blocks
        self.selections
            .draw_pseudo_selection(offset, &self.padding, source.text(), font, painter);
        self.selections
            .draw_selection(offset, &self.padding, source.text(), font, painter);

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
            diagnostic.draw(&self.padding, source.text(), offset, font, painter);
        }

        // draw cursor
        if has_focus {
            Some(self.selections.draw_cursor(offset, &self.padding, font, ui))
        } else {
            None
        }
    }

    fn update_text_if_needed(&mut self, source: &mut Source) {
        if source.has_text_changed_since_last_check() {
            // get blocks
            {
                let mut cursor = source.get_tree_cursor();
                self.blocks =
                    block_drawer::blocks_for_tree(&mut cursor, source.text(), source.lang.config);
            }

            // get padding
            let line_count = source.text().len_lines();
            self.padding = block_drawer::make_padding(&self.blocks, line_count);

            // highlight text
            self.text_drawer.highlight_source(source);
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
        source: &mut Source,
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
                        self.selections
                            .expand_selection(pos, &self.padding, source, font);
                    }
                } else if ui.input(|i| i.pointer.primary_pressed()) && pos.x >= GUTTER_WIDTH {
                    if mods.shift {
                        self.selections
                            .expand_selection(pos, &self.padding, source, font);
                    } else {
                        // if option is held, remove the current block from the source and place it in drag_block
                        if mods.alt {
                            if dragged_block.is_none() {
                                self.start_block_drag(pos, dragged_block, source, font);
                            }
                        } else {
                            self.selections
                                .mouse_clicked(pos, &self.padding, source, font);
                        }
                    }
                    self.completion_popup.clear();
                    response.request_focus();
                }

                // handle mouse up
                if dragged_block.is_some() {
                    let mouse_released = ui.input(|i| i.pointer.primary_released());
                    if mouse_released {
                        let drop_point = self.find_drop_point(pos, source, font);
                        self.drop_block(dragged_block, drop_point, source);
                        response.request_focus();
                    }
                }
            }
        }

        // find diagnostic under cursor
        // TODO: multiple diagnostics displayed at once
        if response.hovered() {
            if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                if let Some(diagnostic_selection) = self.diagnostic_selection {
                    // if still in the current diagnostic range, keep the popup open.
                    // otherwise, clear the selection
                    if let Some(diagnostic) = self.diagnostics.get(diagnostic_selection) {
                        let coord =
                            pt_to_unbounded_text_coord(pointer_pos - offset, &self.padding, font);
                        if !diagnostic.range.contains(coord, source.text()) {
                            self.diagnostic_selection = None;
                        }
                    }
                }

                // if the mouse has been still for a bit with no selection, find the diagnostic under the cursor
                if self.diagnostic_selection.is_none() && Self::mouse_still_for(0.25, ui) {
                    let coord =
                        pt_to_unbounded_text_coord(pointer_pos - offset, &self.padding, font);
                    self.diagnostic_selection = self
                        .diagnostics
                        .iter()
                        .position(|d| d.range.contains(coord, source.text()));
                }
                if Self::mouse_still_for(0.5, ui) {
                    let coord =
                        pt_to_unbounded_text_coord(pointer_pos - offset, &self.padding, font);
                    self.documentation_popup
                        .request_hover(coord.line, coord.col);
                }
            }
        };

        // figure out the block drop points
        if response.contains_pointer() && dragged_block.is_some() {
            ui.input(|i| i.pointer.latest_pos())
                .map(|pointer_pos| self.find_drop_point(pointer_pos - offset, source, font))
        } else {
            None
        }
    }

    fn event(&mut self, source: &mut Source, ui: &Ui) {
        let mut events = ui.input(|i| i.filtered_events(&EVENT_FILTER));

        if self.ime_enabled {
            Self::remove_ime_incompatible_events(&mut events);
            // Process IME events first:
            events.sort_by_key(|e| !matches!(e, Event::Ime(_)));
        }

        for event in &events {
            match event {
                Event::Copy => {
                    let char_range = self
                        .selections
                        .selection()
                        .ordered()
                        .char_range_in(source.text());
                    let selected_text = source.text().slice(char_range).to_string();
                    ui.ctx().copy_text(selected_text);
                }

                Event::Cut => {
                    let char_range = self
                        .selections
                        .selection()
                        .ordered()
                        .char_range_in(source.text());
                    let selected_text = source.text().slice(char_range).to_string();
                    ui.ctx().copy_text(selected_text);

                    source.insert_str("", &mut self.selections);
                }

                Event::Paste(new_text) => {
                    source.insert_str(new_text, &mut self.selections);
                }

                Event::Text(new_text) => {
                    source.insert_char(new_text, &mut self.selections);
                    self.completion_popup
                        .request_completions(source.text(), self.selections.selection())
                }

                Event::Key {
                    modifiers,
                    key,
                    pressed: true,
                    ..
                } => {
                    if !self.handle_selection_modifying_keypress(modifiers, *key, source) {
                        self.handle_text_modifying_keypress(modifiers, *key, source);
                    }
                }

                Event::Ime(ime_event) => self.handle_ime(ime_event, source),

                _ => {}
            };
        }
    }

    fn handle_external_commands(&mut self, commands: &[ExternalCommand], source: &mut Source) {
        for command in commands {
            match command {
                ExternalCommand::SetText(_) => {
                    self.selections.set_selection(TextRange::ZERO, source);
                }
                ExternalCommand::SetFile { .. } => {
                    self.selections.set_selection(TextRange::ZERO, source);
                }
                ExternalCommand::ApplyEdit(edit) => {
                    source.apply_edit(edit, UndoStopCondition::Always, true, &mut self.selections);
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
                        .set_completions(new_completions, source.text());
                }
                ExternalCommand::SetHover(hover) => {
                    self.documentation_popup.set_hover(hover.to_vec());
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
                    source.undo(&mut self.selections);
                    self.completion_popup.clear();
                }
                ExternalCommand::Redo => {
                    source.redo(&mut self.selections);
                    self.completion_popup.clear();
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
        modifiers: &Modifiers,
        key: Key,
        source: &mut Source,
    ) {
        match key {
            // Basic actions
            Key::Enter => {
                if self.completion_popup.has_completions() {
                    self.completion_popup.trigger_completion();
                } else {
                    source.insert_newline(&mut self.selections);
                    self.completion_popup.clear();
                }
            }
            Key::Tab => {
                if self.completion_popup.has_completions() {
                    self.completion_popup.trigger_completion();
                } else {
                    if modifiers.shift {
                        source.unindent(&mut self.selections);
                    } else {
                        source.indent(&mut self.selections)
                    };
                    self.completion_popup.clear();
                }
            }
            Key::Backspace => {
                let movement = if modifiers.mac_cmd {
                    TextMovement::horizontal(HUnit::Line, HDir::Left)
                } else if modifiers.alt || modifiers.ctrl {
                    // alt on mac, ctrl on windows
                    TextMovement::horizontal(HUnit::Word, HDir::Left)
                } else {
                    TextMovement::horizontal(HUnit::Grapheme, HDir::Left)
                };
                source.delete(movement, &mut self.selections);

                self.completion_popup
                    .request_completions(source.text(), self.selections.selection())
            }
            Key::Delete => {
                let movement = if modifiers.mac_cmd {
                    TextMovement::horizontal(HUnit::Line, HDir::Right)
                } else if modifiers.alt || modifiers.ctrl {
                    // alt on mac, ctrl on windows
                    TextMovement::horizontal(HUnit::Word, HDir::Right)
                } else {
                    TextMovement::horizontal(HUnit::Grapheme, HDir::Right)
                };
                source.delete(movement, &mut self.selections);
                self.completion_popup
                    .request_completions(source.text(), self.selections.selection())
            }

            // Undo/Redo
            Key::Z if modifiers.matches_logically(Modifiers::COMMAND) => {
                if modifiers.shift {
                    source.redo(&mut self.selections);
                } else {
                    source.undo(&mut self.selections);
                }
                self.completion_popup.clear();
            }
            Key::Y if modifiers.matches_logically(Modifiers::COMMAND) => {
                source.redo(&mut self.selections);
                self.completion_popup.clear();
            }

            // Control hotkeys
            Key::H if modifiers.ctrl => {
                source.delete(
                    TextMovement::horizontal(HUnit::Grapheme, HDir::Left),
                    &mut self.selections,
                );
                self.completion_popup.clear();
            }
            Key::D if modifiers.ctrl => {
                source.delete(
                    TextMovement::horizontal(HUnit::Grapheme, HDir::Right),
                    &mut self.selections,
                );
                self.completion_popup.clear();
            }
            Key::K if modifiers.ctrl => {
                source.delete(
                    TextMovement::horizontal(HUnit::Line, HDir::Right),
                    &mut self.selections,
                );
                self.completion_popup.clear();
            }

            _ => {}
        }
    }

    fn handle_ime(&mut self, ime_event: &ImeEvent, source: &mut Source) {
        match ime_event {
            ImeEvent::Enabled => {
                self.ime_enabled = true;
                self.ime_selection = self.selections.selection();
            }
            ImeEvent::Preedit(text_mark) => {
                if text_mark != "\n" && text_mark != "\r" {
                    // Empty prediction can be produced when user press backspace
                    // or escape during IME, so we clear current text.
                    source.insert_str("", &mut self.selections);
                    let start_cursor = self.selections.selection().start;
                    if !text_mark.is_empty() {
                        source.insert_str(text_mark, &mut self.selections);
                    }
                    self.ime_selection = self.selections.selection();

                    let new_selection =
                        TextRange::new(start_cursor, self.selections.selection().end);
                    self.selections.set_selection(new_selection, source);
                }
            }
            ImeEvent::Commit(prediction) => {
                if prediction != "\n" && prediction != "\r" {
                    self.ime_enabled = false;

                    if !prediction.is_empty()
                        && self.selections.selection().start == self.ime_selection.start
                    {
                        source.insert_str(prediction, &mut self.selections);
                    } else {
                        self.selections.set_selection(
                            TextRange::new_cursor(self.selections.selection().start),
                            source,
                        );
                    }
                }
            }
            ImeEvent::Disabled => {
                self.ime_enabled = false;
            }
        }
    }

    pub fn handle_selection_modifying_keypress(
        &mut self,
        modifiers: &Modifiers,
        key: Key,
        source: &mut Source,
    ) -> bool {
        match key {
            Key::A if modifiers.command => {
                self.selections.set_selection(
                    TextRange::ZERO.expanded_by(
                        TextMovement::vertical(VUnit::Document, VDir::Down),
                        source.text(),
                    ),
                    source,
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
                    self.selections.move_selecting(movement, source);
                } else {
                    self.selections.move_cursor(movement, source);
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
                        self.selections.move_selecting(movement, source);
                    } else {
                        self.selections.move_cursor(movement, source);
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
    fn content_size(&self, source: &Source, viewport: Rect, font: &MonospaceFont) -> Vec2 {
        // width is max between text and window
        let max_chars = source
            .text()
            .lines()
            .map(|l| l.len_chars())
            .max()
            .unwrap_or(0);
        let max_line_len =
            max_chars as f32 * font.size.x + (OUTER_PAD * 2.0) + GUTTER_WIDTH + TEXT_L_PAD + 40.0; // extra space for nesting blocks
        let width = f32::max(viewport.width(), max_line_len);

        // height is just height of text
        let height = source.text().len_lines() as f32 * font.size.y
            + OUTER_PAD
            + self.padding.iter().sum::<f32>()
            + 200.0; // extra space for over-scroll

        Vec2::new(width, height)
    }

    /// Detects if the mouse has been in the same spot for a certain amount of time (in seconds)
    fn mouse_still_for(duration: f32, ui: &Ui) -> bool {
        let time_since_last_move = ui.input(|i| i.pointer.time_since_last_movement());
        if time_since_last_move < duration {
            ui.ctx()
                .request_repaint_after_secs(duration - time_since_last_move);
            false
        } else {
            true
        }
    }
}
