use crate::core::history::ProjectAction;
use crate::core::project::track::midi::PatternId;
use crate::core::project::{clipboard::ClipboardContent, Note, NoteId};
use crate::lock::{get_app_read, get_app_write, get_history_lock};
use std::sync::Arc;

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
    let mut app = get_app_write();
    app.change_note_params_in_pattern(
        pattern_id,
        note_id,
        velocity,
        probability,
        micro_offset,
        mute,
    )
}

pub fn delete_notes_batch(pattern_id: PatternId, note_ids: Vec<NoteId>) -> anyhow::Result<()> {
    let mut actions = Vec::new();

    // 1. Mutate state and collect actions
    {
        let mut app = get_app_write();
        let pattern_arc = app
            .pattern_pool
            .get_mut(&pattern_id)
            .ok_or_else(|| anyhow::anyhow!("Pattern not found"))?;
        let pattern = Arc::make_mut(pattern_arc);

        let notes_to_delete: Vec<Note> = pattern
            .notes
            .iter()
            .filter(|n| note_ids.contains(&n.id))
            .cloned()
            .collect();

        pattern.delete_notes_by_id(note_ids.into());

        for note in notes_to_delete {
            actions.push(ProjectAction::DeleteNote { pattern_id, note });
        }
    }

    // 2. Update history
    if !actions.is_empty() {
        let mut history = get_history_lock();
        if actions.len() == 1 {
            history.push(actions.remove(0));
        } else {
            history.push(ProjectAction::Batch(actions));
        }
    }

    Ok(())
}

pub fn paste_notes(target_pattern_id: PatternId, playhead_tick: u64) -> anyhow::Result<()> {
    let mut actions = Vec::new();

    // 1. Mutate state
    {
        let mut app = get_app_write();

        let notes_to_paste = match &app.clipboard {
            ClipboardContent::Notes(notes) => notes.clone(),
            _ => return Ok(()),
        };

        if notes_to_paste.is_empty() {
            return Ok(());
        }

        let min_tick = notes_to_paste
            .iter()
            .map(|n| n.start_tick)
            .min()
            .unwrap_or(0);
        let offset = (playhead_tick as i64) - (min_tick as i64);

        let pattern_arc = app
            .pattern_pool
            .get_mut(&target_pattern_id)
            .ok_or_else(|| anyhow::anyhow!("Pattern not found"))?;
        let pattern = Arc::make_mut(pattern_arc);

        for mut note in notes_to_paste {
            let new_start = (note.start_tick as i64 + offset).max(0) as u64;
            note.start_tick = new_start;

            if let Ok(inserted_note) = pattern.insert_note(note) {
                actions.push(ProjectAction::AddNote {
                    pattern_id: target_pattern_id,
                    note: inserted_note,
                });
            }
        }
    }

    // 2. Update history
    if !actions.is_empty() {
        let mut history = get_history_lock();
        if actions.len() == 1 {
            history.push(actions.remove(0));
        } else {
            history.push(ProjectAction::Batch(actions));
        }
    }

    Ok(())
}
