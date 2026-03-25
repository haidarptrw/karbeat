use std::{ collections::HashMap, sync::Arc };

use crate::broadcast_state_change;
use karbeat_core::{
    audio::engine::PlaybackMode,
    commands::AudioCommand,
    context::{ ctx, utils::try_send_audio_command_chain },
    core::{
        history::ProjectAction,
        project::{ GeneratorId, Note, NoteId, track::midi::{ Pattern, PatternId } },
    },
};
use karbeat_core::lock::{ get_app_read, get_app_write, get_history_lock };

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
        let ui_notes: Vec<UiNote> = value.notes
            .iter()
            .map(|note| UiNote::from(note))
            .collect();

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
    let pattern_arc = app.pattern_pool
        .get(&pattern_id)
        .ok_or(format!("Pattern {:?} not found", pattern_id))?;

    let pattern_ui = UiPattern::from(pattern_arc.as_ref());
    Ok(pattern_ui)
}

pub fn get_patterns() -> Result<HashMap<u32, UiPattern>, String> {
    let app = get_app_read();
    let patterns = app.pattern_pool
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
    duration: Option<u64>
) -> Result<UiNote, String> {
    // check key input if it is in the range between 0 - 127
    if key > 127 {
        return Err("Invalid key input: it must not exceed 127".to_string());
    }

    let mut history = get_history_lock();
    let note: Option<Note> = {
        let mut app = get_app_write();
        let pattern_arc = app.pattern_pool
            .get_mut(&PatternId::from(pattern_id))
            .ok_or("Cannot find the pattern".to_string())?;
        let pattern = Arc::make_mut(pattern_arc);

        let duration = duration.unwrap_or(960);
        let note_end = start_tick + duration;
        if note_end > pattern.length_ticks {
            pattern.length_ticks = note_end;
        }

        let note = pattern
            .add_note(key as u8, start_tick, Some(duration))
            .map_err(|e| format!("{}", e))?;

        Some(note)
    };

    let note_unwrapped = note.ok_or(
        "Add note failed previously. 
                This error shouldn't happen as all error cases handle gracefully".to_owned()
    )?;

    let note_ui = UiNote::from(&note_unwrapped);

    history.push(ProjectAction::AddNote {
        pattern_id: PatternId::from(pattern_id),
        note: note_unwrapped,
    });
    broadcast_state_change();
    Ok(note_ui)
}

pub fn delete_note(pattern_id: u32, note_id: u32) -> Result<UiNote, String> {
    let mut history = get_history_lock();
    let note: Note = {
        let mut app = get_app_write();
        let pattern_arc = app.pattern_pool
            .get_mut(&PatternId::from(pattern_id))
            .ok_or("Cannot find the pattern".to_string())?;
        let pattern = Arc::make_mut(pattern_arc);

        let note_id = NoteId::from(note_id);
        let index = pattern.notes
            .iter()
            .position(|n| n.id == note_id)
            .ok_or(format!("Note with ID {:?} not found", note_id))?;
        pattern.delete_note(index).map_err(|e| format!("{}", e))?
    };

    let note_ui = UiNote::from(&note);

    // Add to history
    history.push(ProjectAction::DeleteNote {
        pattern_id: PatternId::from(pattern_id),
        note,
    });
    broadcast_state_change();
    Ok(note_ui)
}

pub fn resize_note(pattern_id: u32, note_id: u32, new_duration: u64) -> Result<UiNote, String> {
    let mut history = get_history_lock();
    let mut app = get_app_write();
    let pattern_arc = app.pattern_pool
        .get_mut(&PatternId::from(pattern_id))
        .ok_or("Cannot find the pattern".to_string())?;
    let pattern = Arc::make_mut(pattern_arc);

    let note_id = NoteId::from(note_id);
    let index = pattern.notes
        .iter()
        .position(|n| n.id == note_id)
        .ok_or(format!("Note with ID {:?} not found", note_id))?;

    let old_duration = pattern.notes[index].duration;

    let note = pattern.resize_note(index, new_duration).map_err(|e| format!("{}", e))?;

    let note_ui = UiNote::from(note);

    // add to history
    history.push(ProjectAction::ResizeNote {
        pattern_id: PatternId::from(pattern_id),
        note_id,
        old_duration,
        new_duration,
    });

    // drop lock here so that broadcast state change can access the APP_STATE
    drop(app);

    broadcast_state_change();
    Ok(note_ui)
}

pub fn move_note(
    pattern_id: u32,
    note_id: u32,
    new_start_tick: u64,
    new_key: u32
) -> Result<UiNote, String> {
    let pattern_id = PatternId::from(pattern_id);
    let note_id = NoteId::from(note_id);

    let mut history = get_history_lock();
    if new_key > 127 {
        return Err("Invalid key".to_string());
    }

    let mut app = get_app_write();
    let pattern_arc = app.pattern_pool.get_mut(&pattern_id).ok_or("Pattern not found")?;
    let pattern = Arc::make_mut(pattern_arc);

    let index = pattern.notes
        .iter()
        .position(|n| n.id == note_id)
        .ok_or(format!("Note with ID {:?} not found", note_id))?;

    let old_tick = pattern.notes[index].start_tick;
    let old_key = pattern.notes[index].key;

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

    // push history
    history.push(ProjectAction::MoveNote {
        pattern_id,
        note_id,
        old_tick,
        old_key,
        new_tick: new_start_tick,
        new_key: new_key as u8,
    });

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
    mute: Option<bool>
) -> Result<UiNote, String> {
    let pattern_id = PatternId::from(pattern_id);
    let note_id = NoteId::from(note_id);

    // validate inputs
    let velocity = velocity.and_then(|v| u8::try_from(v).ok());
    let micro_offset = micro_offset.and_then(|m| i8::try_from(m).ok());

    let mut app = get_app_write();
    let pattern_arc = app.pattern_pool
        .get_mut(&pattern_id)
        .ok_or("Cannot find the pattern".to_string())?;
    let pattern = Arc::make_mut(pattern_arc);

    let index = pattern.notes
        .iter()
        .position(|n| n.id == note_id)
        .ok_or(format!("Note with ID {:?} not found", note_id))?;

    let note = pattern
        .set_note_params(index, velocity, probability, micro_offset, mute)
        .map_err(|e| format!("{}", e))?;

    let note_ui = UiNote::from(note);

    // drop lock here so that broadcast state change can access the APP_STATE
    drop(app);

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
    try_send_audio_command_chain(
        smallvec::smallvec![
            AudioCommand::SetPlaybackMode(
                PlaybackMode::Pattern {
                    pattern_id,
                    generator_id,
                }
            ),
            AudioCommand::SetPlaying(true)
        ]
    ).map_err(|e| format!("{}", e))?;

    // if let Some(cmd_producer) = ctx().command_sender.lock().as_mut() {
    //     let _ = cmd_producer.push(
    //         AudioCommand::SetPlaybackMode(PlaybackMode::Pattern {
    //             pattern_id,
    //             generator_id,
    //         })
    //     );
    //     let _ = cmd_producer.push(AudioCommand::SetPlaying(true));
    // }

    Ok(())
}

/// Stop pattern preview and return to Song mode.
pub fn stop_pattern_preview() -> Result<(), String> {
    // Send commands to stop playing and switch back to Song mode
    if let Some(cmd_producer) = ctx().command_sender.lock().as_mut() {
        let _ = cmd_producer.push(AudioCommand::SetPlaying(false));
        let _ = cmd_producer.push(AudioCommand::SetPlaybackMode(PlaybackMode::Song));
    }

    Ok(())
}
