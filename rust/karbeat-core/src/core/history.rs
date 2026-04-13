use std::sync::Arc;

use crate::{core::project::{
     ApplicationState, Clip, ClipId, Note, NoteId, TrackId,
}, shared::PatternId};

/// Every action to the projects that are stored in history
#[derive(Debug, Clone)]
pub enum ProjectAction {
    AddNote {
        pattern_id: PatternId,
        note: Note,
    },
    DeleteNote {
        pattern_id: PatternId,
        note: Note,
    },
    MoveNote {
        pattern_id: PatternId,
        note_id: NoteId,
        old_tick: u64,
        old_key: u8,
        new_tick: u64,
        new_key: u8,
    },
    ResizeNote {
        pattern_id: PatternId,
        note_id: NoteId,
        old_duration: u64,
        new_duration: u64,
    },
    AddClip {
        track_id: TrackId,
        clip: Clip,
    },
    DeleteClip {
        track_id: TrackId,
        clip: Clip, // Store full clip data to enable undo
    },
    MoveClip {
        old_track_id: TrackId,
        new_track_id: TrackId,
        clip_id: ClipId,
        old_start_time: u32,
        new_start_time: u32,
    },
    ResizeClip {
        track_id: TrackId,
        old_clip: Clip, // Store full clip state before resize
        new_clip: Clip, // Store full clip state after resize
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
                let index = p
                    .notes
                    .iter()
                    .position(|n| n.id == note.id)
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
                let index = p
                    .notes
                    .iter()
                    .position(|n| n.id == *note_id)
                    .ok_or("Note not found")?;
                p.move_note(index, *old_tick, *old_key)
                    .map_err(|e| e.to_string())?;
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
                let index = p
                    .notes
                    .iter()
                    .position(|n| n.id == *note_id)
                    .ok_or("Note not found")?;
                p.resize_note(index, *old_duration)
                    .map_err(|e| e.to_string())?;
            }
            ProjectAction::Batch(actions) => {
                // Inverse of Batch: Undo actions in REVERSE order
                for action in actions.iter().rev() {
                    self.apply_inverse(action, app)?;
                }
            }
            ProjectAction::AddClip { track_id, clip } => {
                // Inverse of AddClip: Delete the clip
                app.delete_clip_from_track(*track_id, clip.id, true)
                    .map_err(|e| e.to_string())?;
            }
            ProjectAction::DeleteClip { track_id, clip } => {
                // Inverse of DeleteClip: Restore the clip to the track
                app.add_clip_to_track(*track_id, clip.clone(), true).map_err(|e| format!("{}", e))?;
            }
            ProjectAction::MoveClip {
                old_track_id,
                new_track_id,
                clip_id,
                old_start_time,
                ..
            } => {
                // Inverse: Move clip back to old_track_id with old_start_time
                app.move_clip(*new_track_id, *old_track_id, *clip_id, *old_start_time)?;
            }
            ProjectAction::ResizeClip {
                track_id, old_clip, ..
            } => {
                // Inverse: Restore the old clip state
                let track_arc = app.tracks.get_mut(track_id).ok_or("Track not found")?;
                let track = Arc::make_mut(track_arc);

                // Remove current clip and insert old clip
                track.clips.retain(|c| c.id != old_clip.id);
                track.clips.insert(Arc::new(old_clip.clone()));
                track.update_max_sample_index();
                app.update_max_sample_index();
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
                let index = p
                    .notes
                    .iter()
                    .position(|n| n.id == note.id)
                    .ok_or("Note not found")?;
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
                let index = p
                    .notes
                    .iter()
                    .position(|n| n.id == *note_id)
                    .ok_or("Note not found")?;
                p.move_note(index, *new_tick, *new_key)
                    .map_err(|e| e.to_string())?;
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
                let index = p
                    .notes
                    .iter()
                    .position(|n| n.id == *note_id)
                    .ok_or("Note not found")?;
                p.resize_note(index, *new_duration)
                    .map_err(|e| e.to_string())?;
            }
            ProjectAction::Batch(actions) => {
                // Forward of Batch: Apply actions in NORMAL order
                for action in actions.iter() {
                    self.apply_forward(action, app)?;
                }
            }
            ProjectAction::AddClip { track_id, clip } => {
                let track = app.tracks.get_mut(track_id).ok_or("Track not found")?;
                let t = Arc::make_mut(track);
                t.add_clip(clip.clone()).map_err(|e| e.to_string())?;
            }
            ProjectAction::DeleteClip { track_id, clip } => {
                // Forward: Delete the clip from the track
                app.delete_clip_from_track(*track_id, clip.id, true)
                    .map_err(|e| e.to_string())?;
            }
            ProjectAction::MoveClip {
                old_track_id,
                new_track_id,
                clip_id,
                new_start_time,
                ..
            } => {
                // Forward: Move clip from old_track_id to new_track_id with new_start_time
                app.move_clip(*old_track_id, *new_track_id, *clip_id, *new_start_time)?;
            }
            ProjectAction::ResizeClip {
                track_id, new_clip, ..
            } => {
                // Forward: Apply the new clip state
                let track_arc = app.tracks.get_mut(track_id).ok_or("Track not found")?;
                let track = Arc::make_mut(track_arc);

                // Remove old clip and insert new clip
                track.clips.retain(|c| c.id != new_clip.id);
                track.clips.insert(Arc::new(new_clip.clone()));
                track.update_max_sample_index();
                app.update_max_sample_index();
            }
        }
        Ok(())
    }
}
