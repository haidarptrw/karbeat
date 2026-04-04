use std::collections::HashMap;

use crate::broadcast_state_change;
use karbeat_core::lock::{get_app_read};
use karbeat_core::{
    api::note as note_api,
    audio::engine::PlaybackMode,
    commands::AudioCommand,
    context::utils::try_send_audio_command_chain,
    core::
        project::{
            track::midi::{Pattern, PatternId},
            GeneratorId, Note, NoteId,
        }
    ,
};

#[derive(Clone)]
pub struct UiPattern {
    pub id: u32,
    pub name: String,
    pub length_ticks: u64,

    pub notes: Vec<UiNote>,
}

#[derive(Clone)]
pub struct UiNote {
    pub id: u32,
    pub start_tick: u64,
    pub duration: u64,
    pub key: u8, // 0 - 127 MIDI key
    pub velocity: u8,

    pub probability: f32,
    pub micro_offset: i8,
    pub mute: bool,
}
// Helper to convert internal Note to UiNote
impl From<&Note> for UiNote {
    fn from(n: &Note) -> Self {
        Self {
            id: n.id.into(), // Convert NoteId to u32
            start_tick: n.start_tick,
            duration: n.duration,
            key: n.key,
            velocity: n.velocity,
            probability: n.probability,
            micro_offset: n.micro_offset,
            mute: n.mute,
        }
    }
}

impl From<&Pattern> for UiPattern {
    fn from(value: &Pattern) -> Self {
        let ui_notes: Vec<UiNote> = value.notes.iter().map(|note| UiNote::from(note)).collect();

        Self {
            id: value.id.into(), // Convert PatternId to u32
            name: value.name.clone(),
            length_ticks: value.length_ticks,
            notes: ui_notes,
        }
    }
}

pub fn get_pattern(pattern_id: u32) -> Result<UiPattern, String> {
    let pattern_id = PatternId::from(pattern_id);
    let app = get_app_read();
    let pattern_arc = app
        .pattern_pool
        .get(&pattern_id)
        .ok_or(format!("Pattern {:?} not found", pattern_id))?;

    let pattern_ui = UiPattern::from(pattern_arc.as_ref());
    Ok(pattern_ui)
}

pub fn get_patterns() -> Result<HashMap<u32, UiPattern>, String> {
    let app = get_app_read();
    let patterns = app
        .pattern_pool
        .iter()
        .map(|(&id, pattern_arc)| {
            let pattern_ui = UiPattern::from(pattern_arc.as_ref());
            (id.into(), pattern_ui)
        })
        .collect();
    Ok(patterns)
}

pub fn add_note(
    pattern_id: u32,
    key: u32,
    start_tick: u64,
    duration: Option<u64>,
) -> Result<UiNote, String> {
    let note = note_api::add_note(PatternId::from(pattern_id), key as u8, start_tick, duration)
        .map_err(|e| format!("{}", e))?;
        
    let note_ui = UiNote::from(&note);
    
    broadcast_state_change();
    Ok(note_ui)
}

pub fn delete_note(pattern_id: u32, note_id: u32) -> Result<UiNote, String> {
    let note = note_api::delete_note(PatternId::from(pattern_id), NoteId::from(note_id))
        .map_err(|e| format!("{}", e))?;
    
    let note_ui = UiNote::from(&note);
    
    broadcast_state_change();
    Ok(note_ui)
}

pub fn resize_note(pattern_id: u32, note_id: u32, new_duration: u64) -> Result<UiNote, String> {
    let note = note_api::resize_note(
        PatternId::from(pattern_id),
        NoteId::from(note_id),
        new_duration,
    )
    .map_err(|e| format!("{}", e))?;
    
    let note_ui = UiNote::from(&note);

    broadcast_state_change();
    Ok(note_ui)
}

pub fn move_note(
    pattern_id: u32,
    note_id: u32,
    new_start_tick: u64,
    new_key: u32,
) -> Result<UiNote, String> {
    let note = note_api::move_note(
        PatternId::from(pattern_id),
        NoteId::from(note_id),
        new_start_tick,
        new_key as u8,
    )
    .map_err(|e| format!("{}", e))?;

    let ui_note = UiNote::from(&note);

    broadcast_state_change();
    Ok(ui_note)
}

pub fn change_note_params(
    pattern_id: u32,
    note_id: u32,
    velocity: Option<i64>,
    probability: Option<f32>,
    micro_offset: Option<i64>,
    mute: Option<bool>,
) -> Result<UiNote, String> {
    // validate inputs
    let velocity = velocity.and_then(|v| u8::try_from(v).ok());
    let micro_offset = micro_offset.and_then(|m| i8::try_from(m).ok());

    let note = note_api::change_note_params(
        PatternId::from(pattern_id),
        NoteId::from(note_id),
        velocity,
        probability,
        micro_offset,
        mute,
    )
    .map_err(|e| format!("{}", e))?;

    let note_ui = UiNote::from(&note);

    broadcast_state_change();
    Ok(note_ui)
}

// ========================= PATTERN PREVIEW TRANSPORT ============================

/// Play a pattern in isolation with a specific generator (looping automatically).
/// This temporarily switches the engine to Pattern playback mode.
pub fn play_pattern_preview(pattern_id: u32, generator_id: u32) -> Result<(), String> {
    let pattern_id = PatternId::from(pattern_id);
    let generator_id = GeneratorId::from(generator_id);

    // Verify pattern exists
    {
        let app = get_app_read();
        if !app.pattern_pool.contains_key(&pattern_id) {
            return Err(format!("Pattern {:?} not found", pattern_id));
        }
    }

    // Send commands to switch to Pattern mode and start playing
    try_send_audio_command_chain(vec![
        AudioCommand::SetPlaybackMode(PlaybackMode::Pattern {
            pattern_id,
            generator_id,
        }),
        AudioCommand::SetPlaying(true)
    ])
    .map_err(|e| format!("{}", e))?;

    Ok(())
}

/// Stop pattern preview and return to Song mode.
pub fn stop_pattern_preview() -> Result<(), String> {
    // Send commands to stop playing and switch back to Song mode
    try_send_audio_command_chain(vec![
        AudioCommand::SetPlaying(false),
        AudioCommand::SetPlaybackMode(PlaybackMode::Song)
    ])
    .map_err(|e| format!("{}", e))?;

    Ok(())
}
