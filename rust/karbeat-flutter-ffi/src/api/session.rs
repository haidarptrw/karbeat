use std::sync::Arc;

use crate::api::track::UiResizeEdge;
use crate::api::{pattern::UiNote, project::UiClip};
use crate::broadcast_state_change;
use karbeat_core::core::project::PatternId;
use karbeat_core::core::{
    history::ProjectAction,
    project::{
        clip::{Clip, ClipId},
        clipboard::ClipboardContent,
        track::TrackId,
        Note, NoteId,
    },
};
use karbeat_core::lock::{get_app_read, get_app_write, get_history_lock};

// =======================================
// Data type definition
// =======================================

#[derive(Clone, Default)]

/// UI-compatible representation of [ClipboardContent](karbeat_core::core::project::clipboard::ClipboardContent)
pub enum UiClipboardContent {
    #[default]
    Empty,
    Notes(Vec<UiNote>), // A list of notes (for Pattern View)
    Clips(Vec<UiClip>), // A list of clips (for Track View)
}

impl From<&ClipboardContent> for UiClipboardContent {
    fn from(clipboard: &ClipboardContent) -> Self {
        match clipboard {
            ClipboardContent::Empty => UiClipboardContent::Empty,
            ClipboardContent::Notes(notes) => {
                let ui_notes = notes.iter().map(|note| UiNote::from(note)).collect();
                UiClipboardContent::Notes(ui_notes)
            }
            ClipboardContent::Clips(clips) => {
                let ui_clips = clips.iter().map(|note| UiClip::from(note)).collect();
                UiClipboardContent::Clips(ui_clips)
            }
        }
    }
}

// Note: Session state (clip selection, preview generator) is now managed
// entirely in the Flutter frontend. Only clipboard and editing APIs remain here.

/// Undo the last action.
pub fn undo() -> Result<(), String> {
    let mut history = get_history_lock();
    let mut app = get_app_write();
    history.undo(&mut app)?;
    drop(app);
    broadcast_state_change();
    Ok(())
}

/// Redo the last undone action.
pub fn redo() -> Result<(), String> {
    let mut history = get_history_lock();
    let mut app = get_app_write();
    history.redo(&mut app)?;
    drop(app);
    broadcast_state_change();
    Ok(())
}

// =============================================
// Pattern Note Actions
// =============================================

/// Copy selected pattern notes to the clipboard.
pub fn copy_pattern_notes(
    pattern_id: u32,
    note_ids: Vec<u32>,
) -> Result<UiClipboardContent, String> {
    let mut app = get_app_write();
    let pattern = app
        .pattern_pool
        .get(&PatternId::from(pattern_id))
        .ok_or("Pattern not found")?;

    // Filter notes
    let notes_to_copy: Vec<Note> = pattern
        .notes
        .iter()
        .filter(|n| note_ids.contains(&n.id.to_u32()))
        .cloned()
        .collect();

    if !notes_to_copy.is_empty() {
        app.clipboard = ClipboardContent::Notes(notes_to_copy);
        Ok(UiClipboardContent::from(&app.clipboard))
    } else {
        Ok(UiClipboardContent::Empty)
    }
}

/// Cut pattern notes: copies them to clipboard then deletes with history.
pub fn cut_pattern_notes(pattern_id: u32, note_ids: Vec<u32>) -> Result<(), String> {
    copy_pattern_notes(pattern_id, note_ids.clone())?;
    delete_pattern_notes(pattern_id, note_ids)?;
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

    if notes_to_paste.is_empty() {
        return Ok(());
    }

    // Shift notes relative to the first note's position vs the playhead
    let min_tick = notes_to_paste
        .iter()
        .map(|n| n.start_tick)
        .min()
        .unwrap_or(0);
    let offset = (playhead_tick as i64) - (min_tick as i64);

    let pattern_arc = app
        .pattern_pool
        .get_mut(&PatternId::from(target_pattern_id))
        .ok_or("Pattern not found")?;
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
                    note: inserted_note,
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

/// Delete notes in group. useful for range and group deletion
pub fn delete_pattern_notes(pattern_id: u32, note_ids: Vec<u32>) -> Result<(), String> {
    let mut app = get_app_write();
    let pattern_arc = app
        .pattern_pool
        .get_mut(&PatternId::from(pattern_id))
        .ok_or("Pattern not found")?;
    let pattern = Arc::make_mut(pattern_arc);

    let mut actions = Vec::new();

    let notes_to_delete: Vec<Note> = pattern
        .notes
        .iter()
        .filter(|n| note_ids.contains(&n.id.to_u32()))
        .cloned()
        .collect();

    let deleted_count =
        pattern.delete_notes_by_id(note_ids.iter().map(|id| NoteId::from(*id)).collect());

    log::info!(
        "deleted {} notes in pattern {}",
        deleted_count,
        pattern.id.to_u32()
    );

    for note in notes_to_delete {
        actions.push(ProjectAction::DeleteNote {
            pattern_id: pattern_id.into(),
            note,
        });
    }

    if !actions.is_empty() {
        let mut history = get_history_lock();
        history.push(ProjectAction::Batch(actions));
    }
    drop(app);
    broadcast_state_change();
    Ok(())
}

// =============================================
// Clip Actions
// =============================================

/// Copy selected clips to the clipboard.
/// Each (track_id, clip_id) pair identifies a clip to copy.
pub fn copy_clips(track_id: u32, clip_ids: Vec<u32>) -> Result<UiClipboardContent, String> {
    let mut app = get_app_write();
    let mut clips_to_copy = Vec::new();

    let track_id_typed: TrackId = track_id.into();
    let track = app.tracks.get(&track_id_typed).ok_or("Track not found")?;

    for clip_id in &clip_ids {
        let clip_arc = track
            .clips
            .iter()
            .find(|c| c.id == ClipId::from(*clip_id))
            .ok_or("Clip not found")?;
        clips_to_copy.push((**clip_arc).clone());
    }

    if !clips_to_copy.is_empty() {
        app.clipboard = ClipboardContent::Clips(clips_to_copy);
        Ok(UiClipboardContent::from(&app.clipboard))
    } else {
        Ok(UiClipboardContent::Empty)
    }
}

/// Cut selected clips: copies them to clipboard then deletes with history.
pub fn cut_clips(track_id: u32, clip_ids: Vec<u32>) -> Result<(), String> {
    // First copy to clipboard
    copy_clips(track_id, clip_ids.clone())?;

    // Then delete with history
    delete_clips(track_id, clip_ids)?;

    Ok(())
}

/// Paste clips from clipboard to a target track at a specified start time.
/// Clips are offset relative to the earliest clip's start time.
pub fn paste_clips(target_track_id: u32, paste_start_time: u32) -> Result<(), String> {
    let mut app = get_app_write();

    // Read Clipboard
    let clips_to_paste = match &app.clipboard {
        ClipboardContent::Clips(clips) => clips.clone(),
        _ => return Ok(()), // Nothing to paste
    };

    if clips_to_paste.is_empty() {
        return Ok(());
    }

    // Calculate offset: shift all clips relative to the earliest one
    let min_start = clips_to_paste
        .iter()
        .map(|c| c.start_time)
        .min()
        .unwrap_or(0);
    let offset = paste_start_time as i64 - min_start as i64;

    let track_id: TrackId = target_track_id.into();

    // Pre-generate new ClipIds for all clips BEFORE borrowing tracks mutably
    let new_clip_ids: Vec<ClipId> = clips_to_paste
        .iter()
        .map(|_| ClipId::next(&mut app.clip_counter))
        .collect();

    // Prepare all new clips with their new IDs and offsets
    let new_clips: Vec<Clip> = clips_to_paste
        .iter()
        .zip(new_clip_ids.iter())
        .map(|(clip, &new_clip_id)| {
            let new_start = (clip.start_time as i64 + offset).max(0) as u32;
            Clip {
                id: new_clip_id,
                name: clip.name.clone(),
                start_time: new_start,
                source: clip.source.clone(),
                offset_start: clip.offset_start,
                loop_length: clip.loop_length,
            }
        })
        .collect();

    // Now get mutable access to the track
    let track_arc = app
        .tracks
        .get_mut(&track_id)
        .ok_or("Target track not found")?;
    let track = Arc::make_mut(track_arc);

    let mut actions = Vec::new();

    for new_clip in new_clips {
        match track.add_clip(new_clip.clone()) {
            Ok(_) => {
                actions.push(ProjectAction::AddClip {
                    track_id,
                    clip: new_clip,
                });
            }
            Err(e) => {
                log::error!("Failed to paste clip: {}", e);
            }
        }
    }

    // Push to history
    if !actions.is_empty() {
        let mut history = get_history_lock();
        history.push(ProjectAction::Batch(actions));
    }

    drop(app);
    broadcast_state_change();
    Ok(())
}

/// Delete specified clips from a track with history support.
pub fn delete_clips(track_id: u32, clip_ids: Vec<u32>) -> Result<(), String> {
    let mut app = get_app_write();

    let track_id_typed: TrackId = track_id.into();
    let track_arc = app
        .tracks
        .get_mut(&track_id_typed)
        .ok_or("Track not found")?;
    let track = Arc::make_mut(track_arc);

    let mut actions = Vec::new();

    for clip_id in clip_ids {
        let clip_id_typed = ClipId::from(clip_id);

        // Find and clone the clip data before removing
        if let Some(clip_arc) = track.clips.iter().find(|c| c.id == clip_id_typed).cloned() {
            let clip_data = (*clip_arc).clone();
            track
                .remove_clip(&clip_id_typed)
                .map_err(|e| e.to_string())?;

            actions.push(ProjectAction::DeleteClip {
                track_id: track_id_typed,
                clip: clip_data,
            });
        }
    }

    track.update_max_sample_index();

    if !actions.is_empty() {
        let mut history = get_history_lock();
        history.push(ProjectAction::Batch(actions));
    }

    drop(app);
    broadcast_state_change();
    Ok(())
}

/// Move a clip from one track to another (or within the same track) with a new start time.
pub fn move_clip(
    old_track_id: u32,
    new_track_id: u32,
    clip_id: u32,
    new_start_time: u32,
) -> Result<(), String> {
    let mut app = get_app_write();

    let old_track_id_typed: TrackId = old_track_id.into();
    let new_track_id_typed: TrackId = new_track_id.into();
    let clip_id_typed = ClipId::from(clip_id);

    // Get the old start time before moving
    let old_start_time = {
        let track = app
            .tracks
            .get(&old_track_id_typed)
            .ok_or("Source track not found")?;
        let clip = track
            .clips
            .iter()
            .find(|c| c.id == clip_id_typed)
            .ok_or("Clip not found in source track")?;
        clip.start_time
    };

    // Perform the move
    app.move_clip(
        old_track_id_typed,
        new_track_id_typed,
        clip_id_typed,
        new_start_time,
    )?;

    // Record in history
    let mut history = get_history_lock();
    history.push(ProjectAction::MoveClip {
        old_track_id: old_track_id_typed,
        new_track_id: new_track_id_typed,
        clip_id: clip_id_typed,
        old_start_time,
        new_start_time,
    });

    drop(app);
    broadcast_state_change();
    Ok(())
}

/// Resize a clip by updating its start_time, offset_start, and/or loop_length.
/// Supports both left (slip edit) and right edge resizing with history support.
pub fn resize_clip(
    track_id: u32,
    clip_id: u32,
    edge: UiResizeEdge,
    new_time_val: u32,
) -> Result<(), String> {
    let mut app = get_app_write();

    let track_id_typed: TrackId = track_id.into();
    let clip_id_typed = ClipId::from(clip_id);

    // Get the old clip state before resizing
    let old_clip = {
        let track = app.tracks.get(&track_id_typed).ok_or("Track not found")?;
        let clip_arc = track
            .clips
            .iter()
            .find(|c| c.id == clip_id_typed)
            .ok_or("Clip not found")?;
        (**clip_arc).clone()
    };

    // Perform the resize
    app.resize_clip(track_id_typed, clip_id_typed, edge.into(), new_time_val)?;

    // Get the new clip state after resizing
    let new_clip = {
        let track = app.tracks.get(&track_id_typed).ok_or("Track not found")?;
        let clip_arc = track
            .clips
            .iter()
            .find(|c| c.id == clip_id_typed)
            .ok_or("Clip not found after resize")?;
        (**clip_arc).clone()
    };

    // Record in history
    let mut history = get_history_lock();
    history.push(ProjectAction::ResizeClip {
        track_id: track_id_typed,
        old_clip,
        new_clip,
    });

    drop(app);
    broadcast_state_change();
    Ok(())
}

// ==================================
// Getters
// ==================================

pub fn get_clipboard_contents() -> UiClipboardContent {
    let app = get_app_read();
    let clipboard_content = &app.clipboard;
    UiClipboardContent::from(clipboard_content)
}
