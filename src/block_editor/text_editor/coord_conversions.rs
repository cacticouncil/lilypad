use egui::Pos2;
use ropey::Rope;

use super::TextPoint;
use crate::block_editor::{
    rope_ext::RopeExt, MonospaceFont, GUTTER_WIDTH, OUTER_PAD, TEXT_L_PAD, TOTAL_TEXT_X_OFFSET,
};

pub fn pt_to_text_coord(
    point: Pos2,
    padding: &[f32],
    source: &Rope,
    font: &MonospaceFont,
) -> TextPoint {
    // find the line clicked on by finding the next one and then going back one
    let raw_y = point.y - OUTER_PAD;
    let mut line: usize = 0;
    let mut total_pad = 0.0;
    for line_pad in padding {
        total_pad += line_pad;
        let curr_line_start = total_pad + (line as f32 * font.size.y);
        if raw_y <= curr_line_start {
            break;
        }
        line += 1;
    }
    line = line.saturating_sub(1);

    // double check that we are in bounds
    // (clicking and deleting at the same time can cause the padding to not be updated yet)
    let line_count = source.len_lines();
    if line >= line_count {
        line = line_count - 1;
    }

    // TODO: if past last line, move to end of last line

    let col_raw =
        ((point.x - OUTER_PAD - GUTTER_WIDTH - TEXT_L_PAD) / font.size.x).round() as usize;
    let col_bound = source.clamp_col(line, col_raw);

    TextPoint::new(line, col_bound)
}

/// Finds the text coordinate that the mouse is over, without clamping to a valid position within the text
pub fn pt_to_unbounded_text_coord(point: Pos2, padding: &[f32], font: &MonospaceFont) -> TextPoint {
    // find the line clicked on by finding the next one and then going back one
    let mut line: usize = 0;
    let mut total_pad = 0.0;
    for line_pad in padding {
        total_pad += line_pad;
        let curr_line_start = total_pad + (line as f32 * font.size.y);
        let raw_y = point.y - OUTER_PAD;
        if raw_y <= curr_line_start {
            break;
        }
        line += 1;
    }

    // add any remaining lines past the last line
    line += ((point.y - (total_pad + (line as f32 * font.size.y))) / font.size.y) as usize;

    line = line.saturating_sub(1);

    let col = ((point.x - OUTER_PAD - GUTTER_WIDTH - TEXT_L_PAD) / font.size.x).round() as usize;

    TextPoint::new(line, col)
}

pub fn text_coord_to_pt(coord: TextPoint, padding: &[f32], font: &MonospaceFont) -> Pos2 {
    let y = OUTER_PAD
        + (coord.line as f32 * font.size.y)
        + padding.iter().take(coord.line).sum::<f32>();
    let x = TOTAL_TEXT_X_OFFSET + (coord.col as f32 * font.size.x);

    Pos2::new(x, y)
}
