use egui::{Color32, Painter, Pos2, Rect, Ui, Vec2};
use ropey::Rope;

use super::{Selections, CURSOR_OFF_DURATION, CURSOR_ON_DURATION};
use crate::{
    block_editor::{
        text_editor::TextPoint, text_range::TextRange, MonospaceFont, OUTER_PAD,
        TOTAL_TEXT_X_OFFSET,
    },
    theme,
};

impl Selections {
    /// Draws the cursor at the current position and returns the rect of the cursor.
    pub fn draw_cursor(
        &self,
        offset: Vec2,
        padding: &[f32],
        font: &MonospaceFont,
        ui: &Ui,
    ) -> Rect {
        // we want to draw the cursor where the mouse has last been (selection end)
        let total_pad: f32 = padding.iter().take(self.selection.end.line + 1).sum();
        let block = Rect::from_min_size(
            Pos2::new(
                TOTAL_TEXT_X_OFFSET + (self.selection.end.col as f32) * font.size.x,
                OUTER_PAD + (self.selection.end.line as f32) * font.size.y + total_pad,
            ) + offset,
            Vec2::new(2.0, font.size.y),
        );

        let time_since_last_selection = self.frame_start_time - self.last_selection_time;
        let total_duration = CURSOR_ON_DURATION + CURSOR_OFF_DURATION;
        let time_in_cycle = time_since_last_selection % total_duration;
        let wake_in = if time_in_cycle < CURSOR_ON_DURATION {
            // cursor is visible
            ui.painter().rect_filled(block, 0.0, theme::CURSOR);
            CURSOR_ON_DURATION - time_in_cycle
        } else {
            // cursor is not visible
            total_duration - time_in_cycle
        };
        ui.ctx().request_repaint_after_secs(wake_in as f32);

        block
    }

    pub fn draw_selection(
        &self,
        offset: Vec2,
        padding: &[f32],
        source: &Rope,
        font: &MonospaceFont,
        painter: &Painter,
    ) {
        if !self.selection.is_cursor() {
            self.draw_selection_blocks(
                self.selection,
                theme::SELECTION,
                offset,
                padding,
                source,
                font,
                painter,
            );
        }
    }

    pub fn draw_pseudo_selection(
        &self,
        offset: Vec2,
        padding: &[f32],
        source: &Rope,
        font: &MonospaceFont,
        painter: &Painter,
    ) {
        if let Some(selection) = self.pseudo_selection {
            self.draw_selection_blocks(
                selection,
                theme::PSEUDO_SELECTION,
                offset,
                padding,
                source,
                font,
                painter,
            );
        }
    }

    fn draw_selection_blocks(
        &self,
        selection: TextRange,
        color: Color32,
        offset: Vec2,
        padding: &[f32],
        source: &Rope,
        font: &MonospaceFont,
        painter: &Painter,
    ) {
        let selection = selection.ordered();
        let line_ranges = selection.individual_lines(source);

        // start the the total padding through the first line so the selection
        // block is placed on the text of the first line (instead of the padding above it)
        let mut total_padding: f32 = padding.iter().take(selection.start.line + 1).sum();

        for line_range in line_ranges {
            // one line per range so the line number is the start of the range
            let line_num = line_range.start.line;

            // find width of selection block in chars
            let width = line_range.end.col - line_range.start.col
                + if line_num != selection.end.line { 1 } else { 0 }; // 1 is added to the width to include the newline

            self.draw_selection_block(
                TextPoint::new(line_num, line_range.start.col),
                width,
                total_padding,
                line_num != selection.start.line,
                color,
                offset,
                padding,
                font,
                painter,
            );

            // the padding for the first line was adding before the loop
            if line_num != selection.start.line {
                total_padding += padding[line_num];
            }
        }
    }

    fn draw_selection_block(
        &self,
        start: TextPoint,
        width: usize,
        padding_above: f32,
        has_block_above: bool,
        color: Color32,
        offset: Vec2,
        padding: &[f32],
        font: &MonospaceFont,
        painter: &Painter,
    ) {
        let line_padding = if has_block_above {
            padding[start.line]
        } else {
            0.0
        };

        let block = Rect::from_min_size(
            Pos2::new(
                (start.col as f32 * font.size.x) + TOTAL_TEXT_X_OFFSET,
                (start.line as f32 * font.size.y) + OUTER_PAD + padding_above,
            ) + offset,
            Vec2::new(width as f32 * font.size.x, font.size.y + line_padding),
        );

        painter.rect_filled(block, 0.0, color);
    }
}
