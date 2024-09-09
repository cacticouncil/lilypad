use egui::Pos2;
use tree_sitter::TreeCursor;

use super::Selections;
use crate::block_editor::{
    rope_ext::RopeExt, source::Source, text_editor::coord_conversions::pt_to_text_coord,
    text_range::movement::TextMovement, MonospaceFont, TextRange,
};

impl Selections {
    pub fn set_selection(&mut self, new_selection: TextRange, source: &Source) {
        self.selection = new_selection;

        // find pseudo selection for new position
        self.find_pseudo_selection(source);

        // make cursor visible whenever moved
        self.last_selection_time = self.frame_start_time;
    }

    // Set the selection as a result of non-edit user input
    fn set_selection_user(&mut self, selection: TextRange, source: &mut Source) {
        self.set_selection(selection, source);
        source.external_cursor_move();
    }

    pub fn find_pseudo_selection(&mut self, source: &Source) {
        self.pseudo_selection = None;
        if self.selection.is_cursor() {
            // find if the cursor is after a quote
            let cursor_loc = self.selection.start;
            let cursor_offset = cursor_loc.char_idx_in(source.text());
            let (prev_char, _) = source.text().surrounding_chars(cursor_offset);

            if prev_char == '"' || prev_char == '\'' {
                self.pseudo_selection = self.string_pseudo_selection_range(
                    source.get_tree_cursor(),
                    cursor_loc.into(),
                    source,
                );
            }
        }
    }

    fn string_pseudo_selection_range(
        &self,
        mut cursor: TreeCursor,
        point: tree_sitter::Point,
        source: &Source,
    ) -> Option<TextRange> {
        // go to lowest node for point
        // don't set if error (bc that would make things go wonky when unpaired)
        while cursor.goto_first_child_for_point(point).is_some() {
            if cursor.node().is_error() {
                return None;
            }
        }

        // verify that our current point is the start or end of a string (not an escape sequence)
        let current_kind = cursor.node().kind_id();
        let kinds = source.language.string_node_ids;
        if !kinds.string_bounds.contains(&current_kind) {
            return None;
        }

        // go up until we hit the string (node of id 230)
        while cursor.goto_parent() {
            let node = cursor.node();
            if node.kind_id() == kinds.string {
                let range =
                    TextRange::new(node.start_position().into(), node.end_position().into());
                return Some(range);
            }
        }

        // we hit the top without finding a string, just return none
        None
    }

    /* ----------------------------- Cursor Movement ---------------------------- */
    pub fn move_cursor(&mut self, movement: TextMovement, source: &mut Source) {
        let new_cursor = self
            .selection
            .find_movement_result(movement, source.text(), false);
        self.set_selection_user(TextRange::new_cursor(new_cursor), source);
    }

    pub fn move_selecting(&mut self, movement: TextMovement, source: &mut Source) {
        let new_sel = self.selection.expanded_by(movement, source.text());
        self.set_selection_user(new_sel, source);
    }

    /* ------------------------------ Mouse Clicks ------------------------------ */
    pub fn mouse_clicked(
        &mut self,
        pos: Pos2,
        padding: &[f32],
        source: &mut Source,
        font: &MonospaceFont,
    ) {
        let text_pos = pt_to_text_coord(pos, padding, source.text(), font);
        self.set_selection_user(TextRange::new_cursor(text_pos), source);
    }

    pub fn expand_selection(
        &mut self,
        pos: Pos2,
        padding: &[f32],
        source: &Source,
        font: &MonospaceFont,
    ) {
        // set selection end to dragged position
        self.selection.end = pt_to_text_coord(pos, padding, source.text(), font);

        // clear pseudo selection if making a selection
        if !self.selection.is_cursor() {
            self.pseudo_selection = None;
        }

        // show cursor
        self.last_selection_time = self.frame_start_time;
    }
}
