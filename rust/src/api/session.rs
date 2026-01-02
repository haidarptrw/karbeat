use std::sync::Arc;

use crate::{broadcast_state_change, core::{history::ProjectAction, project::{Note, clipboard::ClipboardContent}}, utils::lock::{get_app_write, get_history_lock}};

pub fn update_selected_clip(track_id: u32, clip_id: u32) -> Result<(), String> {
    {
        let mut app = get_app_write();

        app.session.selected_track_id = Some(track_id.into());
        app.session.selected_clip_id = Some(clip_id.into());
    }
    Ok(())
}

pub fn deselect_clip() -> Result<(), String> {
    {
        let mut app = get_app_write();
        app.session.selected_track_id = None;
        app.session.selected_clip_id = None;
    }
    Ok(())
}

pub fn undo() -> Result<(), String> {
    let mut history = get_history_lock();
    let mut app = get_app_write();
    history.undo(&mut app)?;
    drop(app);
    broadcast_state_change();
    Ok(())
}

pub fn redo() -> Result<(), String> {
    let mut history = get_history_lock();
    let mut app = get_app_write();
    history.redo(&mut app)?;
    drop(app);
    broadcast_state_change();
    Ok(())
}

pub fn copy_pattern_notes(pattern_id: u32, note_ids: Vec<u32>) -> Result<(), String> {
    let mut app = get_app_write();
    let pattern = app.pattern_pool.get(&pattern_id.into()).ok_or("Pattern not found")?;

    // Filter notes
    let notes_to_copy: Vec<Note> = pattern.notes.iter()
        .filter(|n| note_ids.contains(&n.id.to_u32()))
        .cloned()
        .collect();

    if !notes_to_copy.is_empty() {
        app.clipboard = ClipboardContent::Notes(notes_to_copy);
    }
    Ok(())
}

pub fn cut_pattern_notes(pattern_id: u32, note_ids: Vec<u32>) -> Result<(), String> {
    copy_pattern_notes(pattern_id, note_ids.clone())?;

    let mut app = get_app_write();
    let pattern_arc = app.pattern_pool.get_mut(&pattern_id.into()).ok_or("Pattern not found")?;
    let pattern = Arc::make_mut(pattern_arc);

    let mut actions = Vec::new();

    let notes_to_delete: Vec<Note> = pattern.notes.iter()
        .filter(|n| note_ids.contains(&n.id.to_u32()))
        .cloned()
        .collect();

        pattern.notes.retain(|n| !note_ids.contains(&n.id.to_u32()));
    
    for note in notes_to_delete {
        actions.push(ProjectAction::DeleteNote { pattern_id: pattern_id.into(), note });
    }

    if !actions.is_empty() {
        let mut history = get_history_lock();
        history.push(ProjectAction::Batch(actions));
    }
    drop(app);
    broadcast_state_change();
    Ok(())
}

/// Paste: Reads clipboard, creates new notes, creates Batch Add action
pub fn paste_pattern_notes(target_pattern_id: u32, playhead_tick: u64) -> Result<(), String> {
    let mut app = get_app_write();
    
    // Read Clipboard
    let notes_to_paste = match &app.clipboard {
        ClipboardContent::Notes(notes) => notes.clone(),
        _ => return Ok(()),
    };

    if notes_to_paste.is_empty() { return Ok(()); }

    // Shift notes relative to the first note's position vs the playhead
    let min_tick = notes_to_paste.iter().map(|n| n.start_tick).min().unwrap_or(0);
    let offset = (playhead_tick as i64) - (min_tick as i64);

    let pattern_arc = app.pattern_pool.get_mut(&target_pattern_id.into()).ok_or("Pattern not found")?;
    let pattern = Arc::make_mut(pattern_arc);

    let mut actions = Vec::new();

    for mut note in notes_to_paste {
        let new_start = (note.start_tick as i64 + offset).max(0) as u64;
        note.start_tick = new_start;

        match pattern.insert_note(note) {
            Ok(inserted_note) => {
                // Add to History Batch using the confirmed note data
                actions.push(ProjectAction::AddNote { 
                    pattern_id: target_pattern_id.into(), 
                    note: inserted_note 
                });
            }
            Err(e) => {
                log::error!("Failed to paste note: {}", e);
                // Continue trying to paste other notes
            }
        }
    }

    // Push History
    if !actions.is_empty() {
        let mut history = get_history_lock();
        history.push(ProjectAction::Batch(actions));
    }

    drop(app);
    broadcast_state_change();
    Ok(())
}