use std::collections::HashMap;

use crate::{
    core::project::{Note, Pattern},
    APP_STATE,
};

pub struct UiPattern {
    pub id: u32,
    pub name: String,
    pub length_ticks: u64,

    pub notes: Vec<UiNote>,
}
pub struct UiNote {
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
    let patterns = app.pattern_pool.iter().map(|(&id, pattern_arc)| {
        let pattern_ui = UiPattern::from(pattern_arc.as_ref());
        (id, pattern_ui)
    }).collect();
    Ok(patterns)
}