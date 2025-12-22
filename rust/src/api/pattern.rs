use std::{collections::HashMap, sync::Arc};

use crate::{
    broadcast_state_change,
    core::project::{Note, Pattern},
    sync_audio_graph, APP_STATE,
};

pub struct UiPattern {
    pub id: u32,
    pub name: String,
    pub length_ticks: u64,

    pub notes: Vec<UiNote>,
}
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
            id: n.id,
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
        // Convert the HashMap<u32, Vec<Note>> to HashMap<u32, Vec<UiNote>>
        let ui_notes: Vec<UiNote> = value
            .notes
            .iter()
            .map(|note| {
                let ui_note = UiNote::from(note);
                ui_note
            })
            .collect();

        Self {
            id: value.id,
            name: value.name.clone(),
            length_ticks: value.length_ticks,
            notes: ui_notes,
        }
    }
}

pub fn get_pattern(pattern_id: u32) -> Result<UiPattern, String> {
    let app = APP_STATE.read().map_err(|e| format!("{}", e))?;
    let pattern_arc = app
        .pattern_pool
        .get(&pattern_id)
        .ok_or(format!("Pattern {} not found", pattern_id))?;

    let pattern_ui = UiPattern::from(pattern_arc.as_ref());
    Ok(pattern_ui)
}

pub fn get_patterns() -> Result<HashMap<u32, UiPattern>, String> {
    let app = APP_STATE.read().map_err(|e| format!("{}", e))?;
    let patterns = app
        .pattern_pool
        .iter()
        .map(|(&id, pattern_arc)| {
            let pattern_ui = UiPattern::from(pattern_arc.as_ref());
            (id, pattern_ui)
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
    // check key input if it is in the range between 0 - 127
    if key > 127 {
        return Err("Invalid key input: it must not exceed 127".to_string());
    }

    let note: Option<Note> = {
        let mut app = APP_STATE.write().map_err(|e| format!("{}", e))?;
        let pattern_arc = app
            .pattern_pool
            .get_mut(&pattern_id)
            .ok_or("Cannot find the pattern".to_string())?;
        let pattern = Arc::make_mut(pattern_arc);

        let duration = duration.unwrap_or(960);
        let note_end = start_tick + duration;
        if note_end > pattern.length_ticks {
            pattern.length_ticks = note_end;
        }

        Some(
            pattern
                .add_note(key as u8, start_tick, Some(duration))
                .map_err(|e| format!("{}", e))?,
        )
    };

    let note_ui = UiNote::from(
        &note.ok_or(
            "Add note failed previously. 
    This error shouldn't happen as all error cases handle gracefully"
                .to_owned(),
        )?,
    );
    broadcast_state_change();
    Ok(note_ui)
}

pub fn delete_note(pattern_id: u32, note_id: u32) -> Result<UiNote, String> {
    let note: Note = {
        let mut app = APP_STATE.write().map_err(|e| format!("{}", e))?;
        let pattern_arc = app
            .pattern_pool
            .get_mut(&pattern_id)
            .ok_or("Cannot find the pattern".to_string())?;
        let pattern = Arc::make_mut(pattern_arc);

        let index = pattern
            .notes
            .iter()
            .position(|n| n.id == note_id)
            .ok_or(format!("Note with ID {} not found", note_id))?;
        pattern.delete_note(index).map_err(|e| format!("{}", e))?
    };

    let note_ui = UiNote::from(&note);
    broadcast_state_change();
    Ok(note_ui)
}

pub fn resize_note(pattern_id: u32, note_id: u32, new_duration: u64) -> Result<UiNote, String> {
    let mut app = APP_STATE.write().map_err(|e| format!("{}", e))?;
    let pattern_arc = app
        .pattern_pool
        .get_mut(&pattern_id)
        .ok_or("Cannot find the pattern".to_string())?;
    let pattern = Arc::make_mut(pattern_arc);

    let index = pattern
        .notes
        .iter()
        .position(|n| n.id == note_id)
        .ok_or(format!("Note with ID {} not found", note_id))?;

    let note = pattern
        .resize_note(index, new_duration)
        .map_err(|e| format!("{}", e))?;

    let note_ui = UiNote::from(note);

    // drop lock here so that broadcast state change can access the APP_STATE
    drop(app);

    broadcast_state_change();
    Ok(note_ui)
}

pub fn move_note(
    pattern_id: u32,
    note_id: u32,
    new_start_tick: u64,
    new_key: u32,
) -> Result<UiNote, String> {
    if new_key > 127 {
        return Err("Invalid key".to_string());
    }

    let mut app = APP_STATE.write().map_err(|e| format!("{}", e))?;
    let pattern_arc = app
        .pattern_pool
        .get_mut(&pattern_id)
        .ok_or("Pattern not found")?;
    let pattern = Arc::make_mut(pattern_arc);

    let index = pattern
        .notes
        .iter()
        .position(|n| n.id == note_id)
        .ok_or(format!("Note with ID {} not found", note_id))?;

    let duration = pattern.notes[index].duration;

    // Auto-Expand Pattern Length ---
    let note_end = new_start_tick + duration;
    if note_end > pattern.length_ticks {
        pattern.length_ticks = note_end;
    }

    let note = pattern
        .move_note(index, new_start_tick, new_key as u8)
        .map_err(|e| format!("{}", e))?;
    let ui_note = UiNote::from(note);
    drop(app);
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

    let mut app = APP_STATE.write().map_err(|e| format!("{}", e))?;
    let pattern_arc = app
        .pattern_pool
        .get_mut(&pattern_id)
        .ok_or("Cannot find the pattern".to_string())?;
    let pattern = Arc::make_mut(pattern_arc);

    let index = pattern
        .notes
        .iter()
        .position(|n| n.id == note_id)
        .ok_or(format!("Note with ID {} not found", note_id))?;

    let note = pattern
        .set_note_params(index, velocity, probability, micro_offset, mute)
        .map_err(|e| format!("{}", e))?;

    let note_ui = UiNote::from(note);

    // drop lock here so that broadcast state change can access the APP_STATE
    drop(app);

    broadcast_state_change();
    Ok(note_ui)
}

// TODO: add more APIs for piano roll feature
