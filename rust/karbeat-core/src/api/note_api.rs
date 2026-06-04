use crate::context::utils::broadcast_track_graph;
use crate::core::history::ProjectAction;
use crate::core::project::{Note, NoteId};
use crate::lock::{get_app_write, get_history_lock};
use crate::shared::id::*;

pub fn add_note(
    pattern_id: PatternId,
    key: u8,
    start_tick: u64,
    duration: Option<u64>,
) -> anyhow::Result<Note> {
    // 1. Mutate state
    let note = {
        let mut app = get_app_write();
        app.add_note_to_pattern(pattern_id, key, start_tick, duration)?
    };

    // 2. Update history
    {
        let mut history = get_history_lock();
        history.push(ProjectAction::AddNote {
            pattern_id,
            note: note.clone(),
        });
    }
    broadcast_track_graph();
    Ok(note)
}

pub fn delete_note(pattern_id: PatternId, note_id: NoteId) -> anyhow::Result<Note> {
    // 1. Mutate state
    let note = {
        let mut app = get_app_write();
        app.delete_note_from_pattern(pattern_id, note_id)?
    };

    // 2. Update history
    {
        let mut history = get_history_lock();
        history.push(ProjectAction::DeleteNote {
            pattern_id,
            note: note.clone(),
        });
    }

    broadcast_track_graph();

    Ok(note)
}

pub fn move_note(
    pattern_id: PatternId,
    note_id: NoteId,
    new_start_tick: u64,
    new_key: u8,
) -> anyhow::Result<Note> {
    // 1. Mutate state
    let (note, old_tick, old_key) = {
        let mut app = get_app_write();
        app.move_note_in_pattern(pattern_id, note_id, new_start_tick, new_key)?
    };

    // 2. Update history
    {
        let mut history = get_history_lock();
        history.push(ProjectAction::MoveNote {
            pattern_id,
            note_id,
            old_tick,
            old_key,
            new_tick: new_start_tick,
            new_key,
        });
    }

    broadcast_track_graph();
    Ok(note)
}

pub fn resize_note(
    pattern_id: PatternId,
    note_id: NoteId,
    new_duration: u64,
) -> anyhow::Result<Note> {
    // 1. Mutate state
    let (note, old_duration) = {
        let mut app = get_app_write();
        app.resize_note_in_pattern(pattern_id, note_id, new_duration)?
    };

    // 2. Update history
    {
        let mut history = get_history_lock();
        history.push(ProjectAction::ResizeNote {
            pattern_id,
            note_id,
            old_duration,
            new_duration,
        });
    }

    broadcast_track_graph();

    Ok(note)
}

pub fn change_note_params(
    pattern_id: PatternId,
    note_id: NoteId,
    velocity: Option<u8>,
    probability: Option<f32>,
    micro_offset: Option<i8>,
    mute: Option<bool>,
) -> anyhow::Result<Note> {
    let note = {
        let mut app = get_app_write();
        app.change_note_params_in_pattern(
            pattern_id,
            note_id,
            velocity,
            probability,
            micro_offset,
            mute,
        )?
    };

    broadcast_track_graph();
    Ok(note)
}

/// Add notes in batch
///
/// ## Parameters
/// * pattern_id: [PatternId]
/// * new_notes: Vector of tuples that contains (key, start_tick, duration)
pub fn add_notes_batch(
    pattern_id: PatternId,
    notes_data: Vec<(u8, u64, Option<u64>)>,
) -> anyhow::Result<Vec<Note>> {
    // 1. Mutate state
    let added_notes = {
        let mut app = get_app_write();
        app.add_notes_to_pattern_batch(pattern_id, &notes_data)?
    };

    // 2. Update history
    if !added_notes.is_empty() {
        let mut actions = Vec::with_capacity(added_notes.len());
        for note in &added_notes {
            actions.push(ProjectAction::AddNote {
                pattern_id,
                note: note.clone(),
            });
        }
        let mut history = get_history_lock();
        if actions.len() == 1 {
            history.push(actions.remove(0));
        } else {
            history.push(ProjectAction::Batch(actions));
        }
        broadcast_track_graph();
    }

    Ok(added_notes)
}

pub fn delete_notes_batch(pattern_id: PatternId, note_ids: Vec<NoteId>) -> anyhow::Result<()> {
    // 1. Mutate state
    let deleted_notes = {
        let mut app = get_app_write();
        app.delete_notes_from_pattern_batch(pattern_id, &note_ids)?
    };

    // 2. Update history
    if !deleted_notes.is_empty() {
        let mut actions = Vec::with_capacity(deleted_notes.len());
        for note in deleted_notes {
            actions.push(ProjectAction::DeleteNote { pattern_id, note });
        }
        let mut history = get_history_lock();
        if actions.len() == 1 {
            history.push(actions.remove(0));
        } else {
            history.push(ProjectAction::Batch(actions));
        }
        broadcast_track_graph();
    }

    Ok(())
}

/// Updates is Vec of (note_id, new_tick, new_key)
pub fn move_notes_batch(
    pattern_id: PatternId,
    updates: Vec<(NoteId, u64, u8)>, // (note_id, new_tick, new_key)
) -> anyhow::Result<Vec<Note>> {
    // 1. Mutate state
    let moved_data = {
        let mut app = get_app_write();
        app.move_notes_in_pattern_batch(pattern_id, &updates)?
    };

    // 2. Update history
    let mut final_notes = Vec::with_capacity(moved_data.len());
    if !moved_data.is_empty() {
        let mut actions = Vec::with_capacity(moved_data.len());
        for (note, old_tick, old_key) in moved_data {
            actions.push(ProjectAction::MoveNote {
                pattern_id,
                note_id: note.id,
                old_tick,
                old_key,
                new_tick: note.start_tick,
                new_key: note.key,
            });
            final_notes.push(note);
        }
        let mut history = get_history_lock();
        if actions.len() == 1 {
            history.push(actions.remove(0));
        } else {
            history.push(ProjectAction::Batch(actions));
        }
        broadcast_track_graph();
    }

    Ok(final_notes)
}

/// updates is Vec of (note id, new size)
pub fn resize_notes_batch(
    pattern_id: PatternId,
    updates: Vec<(NoteId, u64)>, // (note_id, new_duration)
) -> anyhow::Result<Vec<Note>> {
    // 1. Mutate state
    let resized_data = {
        let mut app = get_app_write();
        app.resize_notes_in_pattern_batch(pattern_id, &updates)?
    };

    // 2. Update history
    let mut final_notes = Vec::with_capacity(resized_data.len());
    if !resized_data.is_empty() {
        let mut actions = Vec::with_capacity(resized_data.len());
        for (note, old_duration) in resized_data {
            actions.push(ProjectAction::ResizeNote {
                pattern_id,
                note_id: note.id,
                old_duration,
                new_duration: note.duration,
            });
            final_notes.push(note);
        }
        let mut history = get_history_lock();
        if actions.len() == 1 {
            history.push(actions.remove(0));
        } else {
            history.push(ProjectAction::Batch(actions));
        }
        broadcast_track_graph();
    }

    Ok(final_notes)
}
