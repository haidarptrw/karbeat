use std::sync::Arc;

use crate::core::project::{ApplicationState, Note};

/// Every action to the projects that are stored in history
#[derive(Debug, Clone)]
pub enum ProjectAction {
    AddNote {
        pattern_id: u32,
        note: Note,
    },
    DeleteNote {
        pattern_id: u32,
        note: Note, // Keep the data to restore it on Undo
    },
    MoveNote {
        pattern_id: u32,
        note_id: u32,
        old_tick: u64,
        old_key: u8,
        new_tick: u64,
        new_key: u8,
    },
    ResizeNote {
        pattern_id: u32,
        note_id: u32,
        old_duration: u64,
        new_duration: u64,
    },
    /// Groups multiple actions into one Undo/Redo step (e.g. Paste)
    Batch(Vec<ProjectAction>),
}

#[derive(Clone, Default)]
pub struct HistoryManager {
    pub undo_stack: Vec<ProjectAction>,
    pub redo_stack: Vec<ProjectAction>,
    pub max_history: usize,
}

impl HistoryManager {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 100,
        }
    }

    pub fn push(&mut self, action: ProjectAction) {
        self.undo_stack.push(action);
        self.redo_stack.clear();

        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self, app: &mut ApplicationState) -> Result<(), String> {
        let action = self.undo_stack.pop().ok_or("Nothing to undo")?;
        self.apply_inverse(&action, app)?;
        self.redo_stack.push(action);
        Ok(())
    }

    pub fn redo(&mut self, app: &mut ApplicationState) -> Result<(), String> {
        let action = self.redo_stack.pop().ok_or("Nothing to redo")?;
        self.apply_forward(&action, app)?;
        self.undo_stack.push(action);
        Ok(())
    }

    fn apply_inverse(
        &self,
        action: &ProjectAction,
        app: &mut ApplicationState,
    ) -> Result<(), String> {
        match action {
            ProjectAction::AddNote { pattern_id, note } => {
                // Inverse: Delete the note
                let pattern = app
                    .pattern_pool
                    .get_mut(pattern_id)
                    .ok_or("Pattern not found")?;
                let p = Arc::make_mut(pattern);
                let index = p.notes.iter().position(|n| n.id == note.id)
                    .ok_or("Note not found")?;
                
                p.delete_note(index).map_err(|e| e.to_string())?;
            }
            ProjectAction::DeleteNote { pattern_id, note } => {
                // Inverse: Add the note back
                let pattern = app
                    .pattern_pool
                    .get_mut(pattern_id)
                    .ok_or("Pattern not found")?;
                let p = Arc::make_mut(pattern);
                p.restore_note(note.clone()).map_err(|e| e.to_string())?;
            }
            ProjectAction::MoveNote {
                pattern_id,
                note_id,
                old_tick,
                old_key,
                ..
            } => {
                // Inverse: Move to old position
                let pattern = app
                    .pattern_pool
                    .get_mut(pattern_id)
                    .ok_or("Pattern not found")?;
                let p = Arc::make_mut(pattern);
                let index = p.notes.iter().position(|n| n.id == *note_id).ok_or("Note not found")?;
                p.move_note(index, *old_tick, *old_key).map_err(|e| e.to_string())?;
            }
            ProjectAction::ResizeNote {
                pattern_id,
                note_id,
                old_duration,
                ..
            } => {
                let pattern = app
                    .pattern_pool
                    .get_mut(pattern_id)
                    .ok_or("Pattern not found")?;
                let p = Arc::make_mut(pattern);
                let index = p.notes.iter().position(|n| n.id == *note_id).ok_or("Note not found")?;
                p.resize_note(index, *old_duration).map_err(|e| e.to_string())?;
            }
            ProjectAction::Batch(actions) => {
                // Inverse of Batch: Undo actions in REVERSE order
                for action in actions.iter().rev() {
                    self.apply_inverse(action, app)?;
                }
            }
        }

        Ok(())
    }

    fn apply_forward(
        &self,
        action: &ProjectAction,
        app: &mut super::project::ApplicationState,
    ) -> Result<(), String> {
        match action {
            ProjectAction::AddNote { pattern_id, note } => {
                let pattern = app
                    .pattern_pool
                    .get_mut(pattern_id)
                    .ok_or("Pattern not found")?;
                let p = Arc::make_mut(pattern);
                p.restore_note(note.clone()).map_err(|e| e.to_string())?;
            }
            ProjectAction::DeleteNote { pattern_id, note } => {
                let pattern = app
                    .pattern_pool
                    .get_mut(pattern_id)
                    .ok_or("Pattern not found")?;
                let p = Arc::make_mut(pattern);
                let index = p.notes.iter().position(|n| n.id == note.id).ok_or("Note not found")?;
                p.delete_note(index).map_err(|e| e.to_string())?;
            }
            ProjectAction::MoveNote {
                pattern_id,
                note_id,
                new_tick,
                new_key,
                ..
            } => {
                let pattern = app
                    .pattern_pool
                    .get_mut(pattern_id)
                    .ok_or("Pattern not found")?;
                let p = Arc::make_mut(pattern);
                let index = p.notes.iter().position(|n| n.id == *note_id).ok_or("Note not found")?;
                p.move_note(index, *new_tick, *new_key).map_err(|e| e.to_string())?;
            }
            ProjectAction::ResizeNote {
                pattern_id,
                note_id,
                new_duration,
                ..
            } => {
                let pattern = app
                    .pattern_pool
                    .get_mut(pattern_id)
                    .ok_or("Pattern not found")?;
                let p = Arc::make_mut(pattern);
                let index = p.notes.iter().position(|n| n.id == *note_id).ok_or("Note not found")?;
                p.resize_note(index, *new_duration).map_err(|e| e.to_string())?;
            }
            ProjectAction::Batch(actions) => {
                // Forward of Batch: Apply actions in NORMAL order
                for action in actions.iter() {
                    self.apply_forward(action, app)?;
                }
            }
        }
        Ok(())
    }
}
