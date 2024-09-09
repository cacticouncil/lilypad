use ropey::Rope;
use std::borrow::Cow;

use super::{TextEdit, TextRange};
use crate::parse::TreeManager;

pub struct UndoManager {
    undo_stack: Vec<UndoItem>,
    redo_stack: Vec<UndoItem>,
}

enum UndoItem {
    /// An edit to apply
    Edit(TextEdit<'static>),

    /// A stopper to separate undoable actions. Undoing applies edits until it hits a stop, allowing multiple edits to be undone at once.
    Stop,
}

#[derive(PartialEq)]
pub enum UndoStopCondition {
    Always,
    IfNotMerged,
    Never,
}

impl UndoManager {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn add_undo(
        &mut self,
        undo_edit: &TextEdit,
        orig_edit: &TextEdit,
        stop_before: UndoStopCondition,
        stop_after: bool,
    ) {
        use UndoStopCondition::*;

        if stop_before != Never {
            if stop_before == Always {
                self.add_undo_stop();
            }
            // spaces should add a stop unless they are merging with other spaces
            else if orig_edit.text() == " " {
                if let Some(UndoItem::Edit(prev_undo)) = self.undo_stack.last() {
                    if !prev_undo.text().ends_with(' ') {
                        self.add_undo_stop();
                    }
                }
            }
            // should add a stop before certain characters
            else if ['(', '[', '{', ':', '"', '\'', '.']
                .iter()
                .any(|c| orig_edit.text().starts_with(*c))
            {
                self.add_undo_stop();
            }
        }

        // the fewer edits we apply when undoing the better, so try to merge adjacent edits
        let merged = if let Some(UndoItem::Edit(prev_undo)) = self.undo_stack.last() {
            Self::merge_edits(prev_undo, undo_edit)
        } else {
            None
        };

        if let Some(merged) = merged {
            self.undo_stack.pop();
            self.undo_stack.push(UndoItem::Edit(merged));
        } else {
            if stop_before == IfNotMerged {
                self.add_undo_stop();
            }
            self.undo_stack.push(UndoItem::Edit(undo_edit.owned_text()));
        }

        if stop_after {
            self.add_undo_stop();
        }

        // remove the oldest undo if there are too many to prevent memory buildup
        // remove up to the first stop to prevent leaving a partial undo
        if self.undo_stack.len() > 30 {
            if let Some(first_stop) = self.undo_stack.iter().position(|x| match x {
                UndoItem::Edit(_) => false,
                UndoItem::Stop => true,
            }) {
                self.undo_stack.drain(0..=first_stop);
            }
        }
    }

    // Prevents whatever is added to undo next from being undone alongside what is already there
    pub fn add_undo_stop(&mut self) {
        // add a stop to the undo stack if there is not already one
        if let Some(UndoItem::Edit(_)) = self.undo_stack.last() {
            self.undo_stack.push(UndoItem::Stop)
        }
    }

    fn add_redo_stop(&mut self) {
        if let Some(UndoItem::Edit(_)) = self.redo_stack.last() {
            self.redo_stack.push(UndoItem::Stop)
        }
    }

    // Apply all the undos on the stack until it hits a stop. Adding their inverses to the redo stack.
    pub fn apply_undo(
        &mut self,
        source: &mut Rope,
        tree_manager: &mut TreeManager,
    ) -> Option<TextRange> {
        // remove a stop from the top of the stack if it is there
        if let Some(UndoItem::Stop) = self.undo_stack.last() {
            self.undo_stack.pop();
        }

        // apply text edits to source until it hits a stop or the end of the stack
        // add inverses of each of those edits to the redo stack
        self.add_redo_stop();
        let mut selection = None;
        while let Some(item) = self.undo_stack.pop() {
            match item {
                UndoItem::Stop => break,
                UndoItem::Edit(edit) => {
                    let redo = edit.apply(source, tree_manager).owned_text();
                    self.redo_stack.push(UndoItem::Edit(redo));
                    selection = Some(TextRange::new_cursor(edit.new_end()));
                }
            }
        }
        selection
    }

    // Apply all the redos on the stack until it hits a stop. Adding their inverses to the undo stack.
    pub fn apply_redo(
        &mut self,
        source: &mut Rope,
        tree_manager: &mut TreeManager,
    ) -> Option<TextRange> {
        // remove a stop from the top of the stack if it is there
        if let Some(UndoItem::Stop) = self.undo_stack.last() {
            self.undo_stack.pop();
        }

        // same as undo but reversed
        self.add_undo_stop();
        let mut selection = None;
        while let Some(item) = self.redo_stack.pop() {
            match item {
                UndoItem::Stop => break,
                UndoItem::Edit(edit) => {
                    let undo = edit.apply(source, tree_manager).owned_text();
                    self.undo_stack.push(UndoItem::Edit(undo));
                    selection = Some(TextRange::new_cursor(edit.new_end()));
                }
            }
        }
        selection
    }

    // Clear the redo stack. Should be called on any non undo or redo edit
    pub fn clear_redos(&mut self) {
        self.redo_stack.clear();
    }

    /// Combine two text edits if they are adjacent and can be combined (both insertions or deletions)
    fn merge_edits(a: &TextEdit, b: &TextEdit) -> Option<TextEdit<'static>> {
        // ad hoc ordering
        let (first, second) = if a.range().start < b.range().start {
            (a, b)
        } else {
            (b, a)
        };

        let both_insert = first.text() != "" && second.text() != "";
        let both_delete = first.text() == "" && second.text() == "";

        if both_delete && first.range().end == second.range().start {
            Some(TextEdit::delete(TextRange::new(
                first.range().start,
                second.range().end,
            )))
        } else if both_insert && first.new_end() == second.range().start {
            Some(TextEdit::new(
                Cow::Owned(format!("{}{}", first.text(), second.text())),
                first.range(),
            ))
        } else {
            None
        }
    }
}
