use egui::{Color32, Painter, Rect, Response, Stroke, TextEdit, Ui, Vec2, Widget};
use ropey::Rope;

use crate::theme;

use super::{source::Source, text_range::TextRange, MonospaceFont};

mod boyer_moore;

const MARGIN: Vec2 = Vec2::splat(2.5);

pub struct SearchPopup {
    pub results: Option<SearchResults>,
    search: String,
    show: bool,
    is_appearing: bool,
}

impl SearchPopup {
    pub fn new() -> Self {
        SearchPopup {
            results: None,
            search: "".to_string(),
            show: false,
            is_appearing: false,
        }
    }

    pub fn show(&mut self) {
        self.show = true;
        self.is_appearing = true;
    }

    pub fn widget<'a>(&'a mut self, source: &'a Source, source_changed: bool) -> impl Widget + 'a {
        move |ui: &mut Ui| -> Response {
            // don't show or compute anything if the search popup is not visible
            if !self.show {
                return ui.allocate_response(Vec2::ZERO, egui::Sense::hover());
            }

            let (id, rect) = ui.allocate_space(ui.available_size());
            let response = ui.interact(rect, id, egui::Sense::click_and_drag());

            // set background color
            ui.painter().rect_filled(rect, 0.0, theme::POPUP_BACKGROUND);

            self.draw_text_box(ui, source, source_changed, rect);
            self.draw_buttons(ui, rect);

            response
        }
    }

    fn draw_text_box(&mut self, ui: &mut Ui, source: &Source, source_changed: bool, rect: Rect) {
        let text_response = ui.put(
            Rect::from_min_size(
                rect.min + MARGIN,
                rect.size() - Vec2::new(60.0, 0.0) - (MARGIN * 2.0),
            ),
            TextEdit::singleline(&mut self.search)
                .hint_text("Find")
                .background_color(Color32::BLACK),
        );

        // handle changes to search text
        if !self.search.is_empty() && (text_response.changed() || source_changed) {
            self.results = SearchResults::search(source.text(), &self.search);
        } else if self.search.is_empty() {
            self.results = None;
        }

        // grab focus if the search popup is appearing
        if self.is_appearing {
            text_response.request_focus();
            self.is_appearing = false;
        }

        // handle pressing enter (move to next) or escape (close)
        if text_response.lost_focus() {
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                text_response.request_focus();
                self.results.as_mut().map(|r| r.select_next());
            } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.close();
            }
        }
    }

    fn draw_buttons(&mut self, ui: &mut Ui, rect: Rect) {
        // up button
        let up_response = ui.put(
            Rect::from_min_size(
                rect.right_top() - Vec2::new(60.0, 0.0) + MARGIN,
                Vec2::new(20.0, rect.height()) - (MARGIN * 2.0),
            ),
            egui::Button::new("^").fill(theme::POPUP_BACKGROUND),
        );
        if up_response.clicked() {
            self.results.as_mut().map(|r| r.select_prev());
        }

        // down button
        let down_response = ui.put(
            Rect::from_min_size(
                rect.right_top() - Vec2::new(40.0, 0.0) + MARGIN,
                Vec2::new(20.0, rect.height()) - (MARGIN * 2.0),
            ),
            egui::Button::new("v").fill(theme::POPUP_BACKGROUND),
        );
        if down_response.clicked() {
            self.results.as_mut().map(|r| r.select_next());
        }

        // close button
        let close_response = ui.put(
            Rect::from_min_size(
                rect.right_top() - Vec2::new(20.0, 0.0) + MARGIN,
                Vec2::new(20.0, rect.height()) - (MARGIN * 2.0),
            ),
            egui::Button::new("\u{1F5D9}").fill(theme::POPUP_BACKGROUND),
        );
        if close_response.clicked() {
            self.close();
        }
    }

    fn close(&mut self) {
        self.show = false;
        self.results
            .as_mut()
            .map(|r| r.will_clear_and_select = true);
    }
}

pub struct SearchResults {
    results: Vec<TextRange>,
    current: usize,
    will_clear_and_select: bool,
    will_scroll_to_current: bool,
}

impl SearchResults {
    pub fn search(source: &Rope, pat: &str) -> Option<Self> {
        let starts = boyer_moore::boyer_moore_search(source, pat);
        let ranges: Vec<TextRange> = starts
            .iter()
            .map(|start| TextRange::from_char_range_in(source, *start..(*start + pat.len())))
            .collect();
        if ranges.is_empty() {
            None
        } else {
            Some(SearchResults {
                results: ranges,
                current: 0,
                will_clear_and_select: false,
                will_scroll_to_current: true,
            })
        }
    }

    pub fn will_clear_and_select(&self) -> bool {
        self.will_clear_and_select
    }

    pub fn check_and_clear_scroll_to_current(&mut self) -> bool {
        let temp = self.will_scroll_to_current;
        self.will_scroll_to_current = false;
        temp
    }

    pub fn current(&self) -> TextRange {
        self.results[self.current]
    }

    pub fn select_next(&mut self) {
        self.current += 1;
        self.current %= self.results.len();
        self.will_scroll_to_current = true;
    }

    pub fn select_prev(&mut self) {
        if self.current == 0 {
            self.current = self.results.len() - 1;
        } else {
            self.current -= 1;
        }
        self.will_scroll_to_current = true;
    }

    pub fn draw(
        &self,
        offset: Vec2,
        padding: &Padding,
        source: &Rope,
        font: &MonospaceFont,
        painter: &Painter,
    ) {
        for (idx, range) in self.results.iter().enumerate() {
            let stroke = if idx == self.current {
                Stroke::new(2.0, theme::SEARCH_RESULT_SELECTED)
            } else {
                Stroke::NONE
            };
            range.draw_selection_blocks(
                theme::SEARCH_RESULT,
                stroke,
                offset,
                padding,
                source,
                font,
                painter,
            );
        }
    }
}
