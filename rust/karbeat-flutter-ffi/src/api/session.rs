use crate::api::track::UiResizeEdge;
use crate::api::{ pattern::UiNote, project::UiClip };
use karbeat_core::api::{
    self,
    clip_api as clip_api,
    clipboard_api,
    note_api as note_api,
    pattern_api,
};
use karbeat_core::core::project::{ NoteId, PatternId };
use karbeat_core::core::project::{
    clip::ClipId,
    clipboard::ClipboardContent,
    track::TrackId,
};

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
                let ui_notes = notes
                    .iter()
                    .map(|note| UiNote::from(note))
                    .collect();
                UiClipboardContent::Notes(ui_notes)
            }
            ClipboardContent::Clips(clips) => {
                let ui_clips = clips
                    .iter()
                    .map(|note| UiClip::from(note))
                    .collect();
                UiClipboardContent::Clips(ui_clips)
            }
        }
    }
}

// Note: Session state (clip selection, preview generator) is now managed
// entirely in the Flutter frontend. Only clipboard and editing APIs remain here.

/// Undo the last action.
pub fn undo() -> Result<(), String> {
    api::undo()?;
    Ok(())
}

/// Redo the last undone action.
pub fn redo() -> Result<(), String> {
    api::redo()?;
    Ok(())
}

// =============================================
// Pattern Note Actions
// =============================================

/// Copy selected pattern notes to the clipboard.
pub fn copy_pattern_notes(
    pattern_id: u32,
    note_ids: Vec<u32>
) -> Result<UiClipboardContent, String> {
    let pattern_id = PatternId::from(pattern_id);
    let note_ids: Vec<NoteId> = note_ids.into_iter().map(NoteId::from).collect();

    pattern_api
        ::copy_pattern_notes(pattern_id, note_ids, |clipboard| {
            UiClipboardContent::from(clipboard)
        })
        .map_err(|e| e.to_string())
}

/// Cut pattern notes: copies them to clipboard then deletes with history.
pub fn cut_pattern_notes(pattern_id: u32, note_ids: Vec<u32>) -> Result<(), String> {
    copy_pattern_notes(pattern_id, note_ids.clone())?;
    delete_pattern_notes(pattern_id, note_ids)?;
    Ok(())
}

/// Paste: Reads clipboard, creates new notes, creates Batch Add action
pub fn paste_pattern_notes(target_pattern_id: u32, playhead_tick: u64) -> Result<(), String> {
    note_api
        ::paste_notes(
            karbeat_core::core::project::track::midi::PatternId::from(target_pattern_id),
            playhead_tick
        )
        .map_err(|e| format!("{}", e))?;
    Ok(())
}

/// Delete notes in group. useful for range and group deletion
pub fn delete_pattern_notes(pattern_id: u32, note_ids: Vec<u32>) -> Result<(), String> {
    let note_ids_typed = note_ids
        .into_iter()
        .map(karbeat_core::core::project::NoteId::from)
        .collect();
    note_api
        ::delete_notes_batch(
            karbeat_core::core::project::track::midi::PatternId::from(pattern_id),
            note_ids_typed
        )
        .map_err(|e| format!("{}", e))?;

    Ok(())
}

// =============================================
// Clip Actions
// =============================================

/// Copy selected clips to the clipboard.
/// Each (track_id, clip_id) pair identifies a clip to copy.
pub fn copy_clips(track_id: u32, clip_ids: Vec<u32>) -> Result<UiClipboardContent, String> {
    let track_id = TrackId::from(track_id);
    let clip_ids: Vec<ClipId> = clip_ids.into_iter().map(ClipId::from).collect();

    clip_api
        ::copy_clips(track_id, clip_ids, |clipboard| { UiClipboardContent::from(clipboard) })
        .map_err(|e| e.to_string())
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
    clip_api
        ::paste_clips(
            karbeat_core::core::project::track::TrackId::from(target_track_id),
            paste_start_time
        )
        .map_err(|e| format!("{}", e))?;

    Ok(())
}

/// Delete specified clips from a track with history support.
pub fn delete_clips(track_id: u32, clip_ids: Vec<u32>) -> Result<(), String> {
    let clip_ids_typed = clip_ids
        .into_iter()
        .map(karbeat_core::core::project::clip::ClipId::from)
        .collect();
    clip_api
        ::batch_delete_clips(
            karbeat_core::core::project::track::TrackId::from(track_id),
            clip_ids_typed
        )
        .map_err(|e| format!("{}", e))?;

    Ok(())
}

/// Move a clip from one track to another (or within the same track) with a new start time.
pub fn move_clip(
    old_track_id: u32,
    new_track_id: u32,
    clip_id: u32,
    new_start_time: u32
) -> Result<(), String> {
    clip_api
        ::move_clip(
            karbeat_core::core::project::track::TrackId::from(old_track_id),
            karbeat_core::core::project::track::TrackId::from(new_track_id),
            karbeat_core::core::project::clip::ClipId::from(clip_id),
            new_start_time
        )
        .map_err(|e| format!("{}", e))?;

    Ok(())
}

/// Resize a clip by updating its start_time, offset_start, and/or loop_length.
/// Supports both left (slip edit) and right edge resizing with history support.
pub fn resize_clip(
    track_id: u32,
    clip_id: u32,
    edge: UiResizeEdge,
    new_time_val: u32
) -> Result<(), String> {
    clip_api
        ::resize_clip(
            karbeat_core::core::project::track::TrackId::from(track_id),
            karbeat_core::core::project::clip::ClipId::from(clip_id),
            edge.into(),
            new_time_val
        )
        .map_err(|e| format!("{}", e))?;

    Ok(())
}

// ==================================
// Getters
// ==================================

pub fn get_clipboard_contents() -> UiClipboardContent {
    clipboard_api::get_clipboard_contents(|clipboard_ref| {
        UiClipboardContent::from(clipboard_ref)
    })
}
