use std::collections::HashMap;

use egui::{Align2, FontId, Rect, Response, ScrollArea, Sense, Stroke, Ui, Vec2, Widget};

use super::loose_block::LooseBlock;
use crate::block_editor::{DragSession, MonospaceFont};
use crate::theme::blocks_theme::BlocksTheme;
use crate::vscode;
use crate::{
    lang::{config::Snippet, Language},
    theme,
};

pub struct BlockPalette {
    shown: bool,
    selected_palette: usize,
    palette_names: Vec<&'static str>,
    items: Vec<Vec<PaletteItem>>,
}

struct PaletteItem {
    id: &'static str,
    block: LooseBlock,
}

impl PaletteItem {
    fn new(snippet: &Snippet, lang: &mut Language, font: &MonospaceFont) -> Self {
        Self {
            id: snippet.id,
            block: LooseBlock::new(snippet.source, 10.0, lang, font),
        }
    }
}

impl BlockPalette {
    pub fn new() -> Self {
        Self {
            shown: true,
            selected_palette: 0,
            palette_names: vec![],
            items: vec![],
        }
    }

    pub fn populate(&mut self, lang: &mut Language, font: &MonospaceFont) {
        self.items = lang
            .config
            .palettes
            .iter()
            .map(|palette| {
                palette
                    .snippets
                    .iter()
                    .map(|s| PaletteItem::new(s, lang, font))
                    .collect()
            })
            .collect();
        self.palette_names = lang.config.palettes.iter().map(|p| p.name).collect();
        self.selected_palette = 0;
    }

    pub fn is_populated(&self) -> bool {
        !self.items.is_empty()
    }
}

const H_PADDING: f32 = 10.0;
const V_PADDING: f32 = 8.0;
const HEADING_HEIGHT: f32 = 30.0;

impl BlockPalette {
    pub fn widget<'a>(
        &'a mut self,
        dragged_block: &'a mut Option<DragSession>,
        blocks_theme: BlocksTheme,
        font: &'a MonospaceFont,
    ) -> impl Widget + 'a {
        move |ui: &mut Ui| -> Response {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .id_salt("block_palette")
                .drag_to_scroll(false)
                .show(ui, |ui| {
                    let content_size = self.find_size();
                    let expanded_size = content_size.max(ui.available_size() - Vec2::new(0.0, 5.0));
                    let (id, rect) = ui.allocate_space(expanded_size);
                    let response = ui.interact(rect, id, Sense::click());

                    let painter = ui.painter();
                    painter.rect_filled(painter.clip_rect(), 0.0, theme::POPUP_BACKGROUND);

                    if self.shown {
                        ui.painter().text(
                            rect.min + Vec2::new(H_PADDING, V_PADDING),
                            Align2::LEFT_TOP,
                            "Palette:",
                            FontId::proportional(15.0),
                            theme::INTERFACE_TEXT,
                        );

                        ui.put(
                            Rect::from_min_size(
                                rect.min + Vec2::new(H_PADDING + 70.0, V_PADDING),
                                Vec2::new(100.0, 10.0),
                            ),
                            |ui: &mut Ui| -> Response {
                                egui::ComboBox::from_id_salt("palette_selector")
                                    .selected_text(
                                        *self
                                            .palette_names
                                            .get(self.selected_palette)
                                            .unwrap_or(&""),
                                    )
                                    .show_ui(ui, |ui| {
                                        for (i, palette) in self.palette_names.iter().enumerate() {
                                            ui.selectable_value(
                                                &mut self.selected_palette,
                                                i,
                                                *palette,
                                            );
                                        }
                                    })
                                    .response
                            },
                        );

                        self.add_arrow(ui, rect);

                        self.add_blocks(
                            (rect.min + Vec2::new(H_PADDING, HEADING_HEIGHT)).to_vec2(),
                            content_size.x,
                            ui,
                            dragged_block,
                            blocks_theme,
                            font,
                        );
                    } else {
                        self.add_arrow(ui, rect);
                    }

                    // if mouse released in the block palette, cancel the drag
                    if response.contains_pointer() {
                        let mouse_released = ui.input(|i| i.pointer.primary_released());
                        if mouse_released {
                            *dragged_block = None;
                        }
                    }

                    response
                })
                .inner
        }
    }

    fn add_arrow(&mut self, ui: &mut Ui, rect: Rect) {
        let direction = if self.shown { Vec2::RIGHT } else { Vec2::LEFT };
        let close_response = ui.put(
            Rect::from_min_size(
                rect.right_top() + Vec2::new(-(H_PADDING + 30.0), V_PADDING - 5.0),
                Vec2::splat(30.0),
            ),
            ArrowButton::new(direction),
        );
        if close_response.clicked() {
            self.shown = !self.shown;
        }
    }

    fn add_blocks(
        &mut self,
        mut offset: Vec2,
        width: f32,
        ui: &mut Ui,
        dragged_block: &mut Option<DragSession>,
        blocks_theme: BlocksTheme,
        font: &MonospaceFont,
    ) {
        for item in self.items.get(self.selected_palette).unwrap_or(&vec![]) {
            let block_rect = Rect::from_min_size(
                offset.to_pos2(),
                Vec2::new(width - (H_PADDING * 3.0), item.block.min_size().y),
            );
            let response = ui.put(block_rect, item.block.widget(blocks_theme, font));
            if dragged_block.is_none() {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    vscode::log_event("palette-blog-drag", HashMap::from([("type", item.id)]));

                    *dragged_block = Some(DragSession {
                        text: item.block.text().to_string(),
                        offset: pointer_pos - block_rect.min.to_vec2(),
                    });
                }
            }

            offset.y += item.block.min_size().y + V_PADDING;
        }
    }

    pub fn find_size(&self) -> Vec2 {
        if self.shown {
            let mut size = Vec2::ZERO;
            for item in self.items.get(self.selected_palette).unwrap_or(&vec![]) {
                size.x = f32::max(size.x, item.block.min_size().x);
                size.y += item.block.min_size().y + V_PADDING;
            }
            size.x += H_PADDING * 3.0;
            size.max(Vec2::new(220.0, 50.0))
        } else {
            Vec2::new(40.0, 30.0)
        }
    }
}

struct ArrowButton {
    direction: Vec2,
}

impl ArrowButton {
    fn new(direction: Vec2) -> Self {
        Self { direction }
    }
}

impl Widget for ArrowButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(30.0, 30.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            let arrow_points = [
                rect.center() + self.direction * rect.height() * 0.2,
                rect.center() - self.direction.rot90() * rect.height() * 0.2,
                rect.center() + self.direction.rot90() * rect.height() * 0.2,
            ];
            ui.painter().add(egui::Shape::convex_polygon(
                arrow_points.to_vec(),
                visuals.fg_stroke.color,
                Stroke::NONE,
            ));
        }

        response
    }
}
