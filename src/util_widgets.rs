use egui::{Align2, Color32, FontId, Response, Sense, Ui, Widget};

use crate::theme;

/// A selectable row that can be used in a list, and responds to a click anywhere in the row.
pub struct SelectableRow<'a> {
    text: &'a str,
    color: Color32,
    selected: bool,
    font: FontId,
}

impl<'a> SelectableRow<'a> {
    pub fn new(text: &'a str, color: Color32, selected: bool, font: FontId) -> Self {
        Self {
            text,
            color,
            selected,
            font,
        }
    }
}

impl Widget for SelectableRow<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (id, rect) = ui.allocate_space(ui.available_size());
        // have this also respond to drag so that drags don't pass through to the
        // text editor behind it when this is up
        let response = ui.interact(rect, id, Sense::click_and_drag());

        // highlight background if selected or hovered
        let painter = ui.painter();
        if self.selected || response.hovered() {
            painter.rect_filled(rect, 0.0, theme::SELECTION);
        }

        painter.text(
            rect.left_center(),
            Align2::LEFT_CENTER,
            self.text,
            self.font,
            self.color,
        );

        response
    }
}
