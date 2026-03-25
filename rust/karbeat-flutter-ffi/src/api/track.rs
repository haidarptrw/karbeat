// rust\src\api\track.rs

use std::sync::Arc;

use crate::api::project::{ UiClip, UiTrack };
use crate::broadcast_state_change;
use karbeat_core::core::project::clip::ResizeEdge;
use karbeat_core::core::{
    history::ProjectAction,
    project::{
        clip::{ Clip, ClipId },
        track::{ audio_waveform::AudioSourceId, midi::{ Pattern, PatternId }, TrackId, TrackType },
        KarbeatSource,
    },
};
use karbeat_core::lock::{ get_app_read, get_app_write, get_history_lock };
use karbeat_core::utils::get_waveform_buffer;
use karbeat_utils::color::Color;

pub enum UiSourceType {
    Audio,
    Midi,
}

pub enum UiResizeEdge {
    Left,
    Right,
}

impl Into<UiResizeEdge> for ResizeEdge {
    fn into(self) -> UiResizeEdge {
        match self {
            ResizeEdge::Left => UiResizeEdge::Left,
            ResizeEdge::Right => UiResizeEdge::Right,
        }
    }
}

impl From<&UiResizeEdge> for ResizeEdge {
    fn from(value: &UiResizeEdge) -> Self {
        match value {
            UiResizeEdge::Left => ResizeEdge::Left,
            UiResizeEdge::Right => ResizeEdge::Right,
        }
    }
}

impl From<UiResizeEdge> for ResizeEdge {
    fn from(value: UiResizeEdge) -> Self {
        match value {
            UiResizeEdge::Left => ResizeEdge::Left,
            UiResizeEdge::Right => ResizeEdge::Right,
        }
    }
}

pub fn create_clip(
    source_id: Option<u32>,
    source_type: UiSourceType,
    track_id: u32,
    start_time: u32
) -> Result<(), String> {
    let track_id = TrackId::from(track_id);

    {
        let mut app = get_app_write();
        let mut history_manager = get_history_lock();

        match source_type {
            UiSourceType::Audio => {
                let source_id = source_id.ok_or(format!("Audio clip needs source id"))?;
                let source_id = AudioSourceId::from(source_id);
                // check the source
                let audio_source = app.asset_library.source_map
                    .get(&source_id)
                    .ok_or("The audio source is not available in the library".to_string())?
                    .clone();

                let project_sample_rate = app.audio_config.sample_rate as f64;
                let source_sample_rate = audio_source.sample_rate as f64;
                let buffer_len = get_waveform_buffer(&audio_source.buffer)
                    .map(|b| b.len())
                    .unwrap_or(0);
                let source_frames = (buffer_len as u32) / (audio_source.channels as u32);
                let timeline_length = if source_sample_rate > 0.0 {
                    ((source_frames as f64) * (project_sample_rate / source_sample_rate)) as u32
                } else {
                    source_frames // Fallback to avoid division by zero
                };

                let new_clip_id = ClipId::next(&mut app.clip_counter);

                let clip = Clip {
                    name: audio_source.name.clone(),
                    id: new_clip_id,
                    start_time,
                    source: karbeat_core::core::project::KarbeatSource::Audio(source_id),
                    offset_start: 0,
                    loop_length: timeline_length,
                };
                app.add_clip_to_track(track_id, clip.clone());

                history_manager.push(ProjectAction::AddClip { track_id, clip });
            }
            UiSourceType::Midi => {
                let sample_rate = app.audio_config.sample_rate;
                let bpm = if app.transport.bpm == 0.0 { 120.0 } else { app.transport.bpm };
                let samples_per_beat = ((sample_rate as f32) / (bpm / 60.0)) as u32;

                // Use existing pattern if source_id provided, otherwise create new
                let (pattern_id, timeline_length) = if let Some(id) = source_id {
                    let pattern_id = PatternId::from(id);
                    let pattern = app.pattern_pool
                        .get(&pattern_id)
                        .ok_or(format!("Pattern {} not found", id))?;

                    // Calculate length from pattern's ticks
                    let samples_per_tick = (samples_per_beat as f32) / 960.0;
                    let length = ((pattern.length_ticks as f32) * samples_per_tick) as u32;
                    (pattern_id, length)
                } else {
                    // Create new pattern
                    let new_pattern_id = PatternId::next(&mut app.pattern_counter);
                    let default_ticks = 4 * 960;
                    let timeline_length = 4 * samples_per_beat;

                    let pattern = Arc::new(Pattern {
                        id: new_pattern_id,
                        name: format!("Pattern {}", new_pattern_id.to_u32()),
                        length_ticks: default_ticks,
                        notes: Vec::new(),
                        next_note_id: 0,
                    });
                    app.pattern_pool.insert(new_pattern_id, pattern);
                    (new_pattern_id, timeline_length)
                };

                let pattern_name = app.pattern_pool
                    .get(&pattern_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| format!("Pattern {}", pattern_id.to_u32()));

                let new_clip_id = ClipId::next(&mut app.clip_counter);
                let clip = Clip {
                    name: pattern_name,
                    id: new_clip_id,
                    start_time,
                    source: KarbeatSource::Midi(pattern_id),
                    offset_start: 0,
                    loop_length: timeline_length,
                };

                app.add_clip_to_track(track_id, clip.clone());
                history_manager.push(ProjectAction::AddClip { track_id, clip });
            }
        }
    }
    broadcast_state_change();
    Ok(())
}

pub fn delete_clip(track_id: u32, clip_id: u32) -> Result<(), String> {
    let track_id = TrackId::from(track_id);
    let clip_id = ClipId::from(clip_id);

    {
        let mut app = get_app_write();
        let mut history_manager = get_history_lock();

        let deleted_clip_arc = app
            .delete_clip_from_track(track_id, clip_id)
            .map_err(|e| format!("Failed to delete clip: {}", e))?;

        let deleted_clip = deleted_clip_arc.as_ref().to_owned();

        history_manager.push(ProjectAction::DeleteClip {
            track_id,
            clip: deleted_clip,
        });
    }
    broadcast_state_change();
    Ok(())
}

/// Resize the clip. for default mode, it will only adjust
/// the start time and loop length of the clip
pub fn resize_clip(
    track_id: u32,
    clip_id: u32,
    edge: UiResizeEdge,
    new_time_val: u32
) -> Result<(), String> {
    let track_id = TrackId::from(track_id);
    let clip_id = ClipId::from(clip_id);

    {
        let mut app = get_app_write();
        let track_arc = app.tracks.get_mut(&track_id).ok_or("Track not found")?;

        let track = Arc::make_mut(track_arc);

        let clips = &mut track.clips;

        if
            let Some(clip) = clips
                .iter()
                .find(|c| c.id == clip_id)
                .cloned()
        {
            clips.remove(&clip);

            let mut modified_clip = (*clip).clone();

            match ResizeEdge::from(&edge) {
                ResizeEdge::Right => {
                    if new_time_val > modified_clip.start_time {
                        let new_length = new_time_val - modified_clip.start_time;
                        modified_clip.loop_length = new_length;
                    }
                }
                ResizeEdge::Left => {
                    // Dragging Left Edge: Slip Edit
                    let old_start = modified_clip.start_time;
                    let old_end = old_start + modified_clip.loop_length;

                    // Bound check: New Start cannot be past the old End
                    if new_time_val < old_end {
                        let new_start = new_time_val;

                        // Calculate delta (positive = trimmed right, negative = expanded left)
                        let delta = (new_start as i64) - (old_start as i64);

                        let current_offset = modified_clip.offset_start as i64;
                        let new_offset = current_offset + delta;

                        // Constraint: offset cannot be negative (can't start before 0 of source)
                        if new_offset >= 0 {
                            modified_clip.start_time = new_start;
                            // Length shrinks as start moves right (or grows as it moves left)
                            modified_clip.loop_length = old_end - new_start;
                            modified_clip.offset_start = new_offset as u32;
                        }
                    }
                }
            }

            clips.insert(Arc::new(modified_clip));
            track.update_max_sample_index();
        } else {
            return Err("Clip not found".to_string());
        }

        // update since there is a modification of a clip
        app.update_max_sample_index();
    }

    broadcast_state_change();
    Ok(())
}

pub fn move_clip(
    source_track_id: u32,
    clip_id: u32,
    new_start_time: u32,
    new_track_id: Option<u32>
) -> Result<(), String> {
    let source_track_id = TrackId::from(source_track_id);
    let clip_id = ClipId::from(clip_id);
    let new_track_id_opt = new_track_id.map(TrackId::from);

    {
        let mut app = get_app_write();
        let target_track_id = new_track_id_opt.unwrap_or(source_track_id);
        let target_type = if let Some(target) = app.tracks.get(&target_track_id) {
            target.track_type.clone()
        } else {
            return Err("Target track not found".to_string());
        };

        let track_arc = app.tracks.get_mut(&source_track_id).ok_or("Track not found")?;

        if source_track_id == target_track_id {
            let track = Arc::make_mut(track_arc);
            let clips = &mut track.clips;
            if
                let Some(clip) = clips
                    .iter()
                    .find(|c| c.id == clip_id)
                    .cloned()
            {
                // remove old clip
                clips.remove(&clip);
                let mut modified_clip = (*clip).clone();
                modified_clip.start_time = new_start_time;
                clips.insert(Arc::new(modified_clip));
                track.update_max_sample_index();
            } else {
                return Err("[move_clip] Clip not found".to_string());
            }
        } else {
            let track = Arc::make_mut(track_arc);
            let clips = &mut track.clips;

            let clip = clips
                .iter()
                .find(|c| c.id == clip_id)
                .ok_or("[move_clip] clip not found".to_string())?
                .clone();

            let is_compatible = match (&target_type, &clip.source) {
                (TrackType::Audio, KarbeatSource::Audio(_)) => true,
                (TrackType::Midi, KarbeatSource::Midi(_)) => true,
                _ => false,
            };

            if !is_compatible {
                return Err(
                    format!(
                        "Incompatible track type. Cannot move {:?} clip to {:?} track.",
                        clip.source,
                        target_type
                    )
                );
            }

            clips.remove(&clip);
            track.update_max_sample_index();

            let mut new_clip = (*clip).clone();
            new_clip.start_time = new_start_time;

            // get target track. this is already checked at the beginning, so it will never throws error
            let target_track = Arc::make_mut(app.tracks.get_mut(&target_track_id).unwrap());
            let _ = target_track.add_clip(new_clip).map_err(|e| format!("{}", e));
        }
        app.update_max_sample_index();
    }

    broadcast_state_change();
    Ok(())
}

/// Add a MIDI track with a generator by its registry ID (preferred method).
pub fn add_midi_track_with_generator_id(registry_id: u32) -> Result<(), String> {
    {
        let mut app = get_app_write();
        app.add_new_midi_track_with_generator_id(registry_id).map_err(|e| format!("{}", e))?;
    }
    broadcast_state_change();
    Ok(())
}

/// Add a MIDI track with a generator by name (backwards compatible).
pub fn add_midi_track_with_generator(generator_name: String) -> Result<(), String> {
    {
        let mut app = get_app_write();
        app.add_new_midi_track_with_generator(&generator_name).map_err(|e| format!("{}", e))?;
    }
    broadcast_state_change();
    Ok(())
}

pub fn get_clip(track_id: u32, clip_id: u32) -> Result<UiClip, String> {
    let track_id = TrackId::from(track_id);
    let clip_id = ClipId::from(clip_id);

    let app = get_app_read();

    let track = app.tracks.get(&track_id).ok_or(format!("Track {:?} not found", track_id))?;

    let clip = track.clips
        .iter()
        .find(|c| c.id == clip_id)
        .ok_or(format!("Clip {:?} not found in track {:?}", clip_id, track_id))?;

    Ok(UiClip::from(clip.as_ref()))
}

// Alternatively, fetching the whole Track is often useful too and still cheaper than all tracks
pub fn get_track(track_id: u32) -> Result<UiTrack, String> {
    let track_id = TrackId::from(track_id);

    let app = get_app_read();
    let track = app.tracks.get(&track_id).ok_or(format!("Track {:?} not found", track_id))?;

    Ok(UiTrack::from(track.as_ref()))
}

// =====================================
// API for multiple actions at once
// =====================================

/// move clips in batch
pub fn move_clip_batch(
    source_track_id: u32,
    clip_ids: Vec<u32>,
    delta_samples: i64,
    new_track_id: Option<u32>
) -> Result<(), String> {
    let source_track_id = TrackId::from(source_track_id);
    let new_track_id_opt = new_track_id.map(TrackId::from);

    {
        let mut app = get_app_write();
        let target_track_id = new_track_id_opt.unwrap_or(source_track_id);

        // Validate target track exists and get its type
        let target_type = if let Some(target) = app.tracks.get(&target_track_id) {
            target.track_type.clone()
        } else {
            return Err("Target track not found".to_string());
        };

        let clip_ids: Vec<ClipId> = clip_ids.into_iter().map(ClipId::from).collect();

        if source_track_id == target_track_id {
            // Same track: just update start times
            let track_arc = app.tracks.get_mut(&source_track_id).ok_or("Source track not found")?;
            let track = Arc::make_mut(track_arc);

            for clip_id in &clip_ids {
                if
                    let Some(clip) = track.clips
                        .iter()
                        .find(|c| c.id == *clip_id)
                        .cloned()
                {
                    track.clips.remove(&clip);
                    let mut modified_clip = (*clip).clone();
                    // Apply delta with clamping to 0
                    let new_start = ((modified_clip.start_time as i64) + delta_samples).max(
                        0
                    ) as u32;
                    modified_clip.start_time = new_start;
                    track.clips.insert(Arc::new(modified_clip));
                }
            }
            track.update_max_sample_index();
        } else {
            // Cross-track move
            let source_track = Arc::make_mut(
                app.tracks.get_mut(&source_track_id).ok_or("Source track not found")?
            );

            let mut clips_to_move = Vec::new();
            for clip_id in &clip_ids {
                if
                    let Some(clip) = source_track.clips
                        .iter()
                        .find(|c| c.id == *clip_id)
                        .cloned()
                {
                    // Check compatibility
                    let is_compatible = match (&target_type, &clip.source) {
                        (TrackType::Audio, KarbeatSource::Audio(_)) => true,
                        (TrackType::Midi, KarbeatSource::Midi(_)) => true,
                        _ => false,
                    };
                    if !is_compatible {
                        continue; // Skip incompatible clips
                    }
                    source_track.clips.remove(&clip);
                    clips_to_move.push(clip);
                }
            }
            source_track.update_max_sample_index();

            // Add to target track
            let target_track = Arc::make_mut(
                app.tracks.get_mut(&target_track_id).ok_or("Target track not found")?
            );
            for clip in clips_to_move {
                let mut modified_clip = (*clip).clone();
                let new_start = ((modified_clip.start_time as i64) + delta_samples).max(0) as u32;
                modified_clip.start_time = new_start;
                let _ = target_track.add_clip(modified_clip);
            }
        }
        app.update_max_sample_index();
    }

    broadcast_state_change();
    Ok(())
}

/// Resize clips in batch by a delta amount
pub fn resize_clip_batch(
    track_id: u32,
    clip_ids: Vec<u32>,
    edge: UiResizeEdge,
    delta_samples: i64
) -> Result<(), String> {
    let track_id = TrackId::from(track_id);
    let clip_ids: Vec<ClipId> = clip_ids.into_iter().map(ClipId::from).collect();

    {
        let mut app = get_app_write();
        let track_arc = app.tracks.get_mut(&track_id).ok_or("Track not found")?;
        let track = Arc::make_mut(track_arc);

        for clip_id in &clip_ids {
            if
                let Some(clip) = track.clips
                    .iter()
                    .find(|c| c.id == *clip_id)
                    .cloned()
            {
                track.clips.remove(&clip);
                let mut modified_clip = (*clip).clone();

                match ResizeEdge::from(&edge) {
                    ResizeEdge::Right => {
                        // Extend/shrink the right edge by delta
                        let current_end = modified_clip.start_time + modified_clip.loop_length;
                        let new_end = ((current_end as i64) + delta_samples).max(
                            (modified_clip.start_time as i64) + 100
                        ) as u32;
                        modified_clip.loop_length = new_end - modified_clip.start_time;
                    }
                    ResizeEdge::Left => {
                        // Slip edit: move start time and adjust offset
                        let old_start = modified_clip.start_time;
                        let old_end = old_start + modified_clip.loop_length;
                        let new_start = ((old_start as i64) + delta_samples).clamp(
                            0,
                            (old_end as i64) - 100
                        ) as u32;

                        let delta = (new_start as i64) - (old_start as i64);
                        let current_offset = modified_clip.offset_start as i64;
                        let new_offset = (current_offset + delta).max(0) as u32;

                        modified_clip.start_time = new_start;
                        modified_clip.loop_length = old_end - new_start;
                        modified_clip.offset_start = new_offset;
                    }
                }

                track.clips.insert(Arc::new(modified_clip));
            }
        }
        track.update_max_sample_index();
        app.update_max_sample_index();
    }

    broadcast_state_change();
    Ok(())
}

/// Delete clips in batch
pub fn delete_clip_batch(track_id: u32, clip_ids: Vec<u32>) -> Result<(), String> {
    let track_id = TrackId::from(track_id);
    let clip_ids: Vec<ClipId> = clip_ids.into_iter().map(ClipId::from).collect();

    {
        let mut app = get_app_write();
        let mut history_manager = get_history_lock();

        for clip_id in clip_ids {
            if let Ok(deleted_clip_arc) = app.delete_clip_from_track(track_id, clip_id) {
                let deleted_clip = deleted_clip_arc.as_ref().to_owned();
                history_manager.push(ProjectAction::DeleteClip {
                    track_id,
                    clip: deleted_clip,
                });
            }
        }
    }

    broadcast_state_change();
    Ok(())
}

pub fn change_track_name(track_id: u32, new_name: &str) -> Result<(), String> {
    // check the name's length
    if new_name.len() > 20 {
        return Err("Track name cannot exceed 20 characters".to_string());
    }

    let track_id = TrackId::from(track_id);

    {
        let mut app = get_app_write();
        let track_arc = app.tracks.get_mut(&track_id).ok_or("Track not found")?;
        let track = Arc::make_mut(track_arc);
        track.name = new_name.to_string();
    }

    broadcast_state_change();
    Ok(())
}

/// Change the track header's color to a new color specified by a hex string (e.g. "#RRGGBB" or "#RRGGBBAA").
pub fn change_track_color(track_id: u32, new_color: &str) -> Result<(), String> {
    let track_id = TrackId::from(track_id);

    {
        let mut app = get_app_write();
        let track_arc = app.tracks.get_mut(&track_id).ok_or("Track not found")?;
        let track = Arc::make_mut(track_arc);
        track.color = Color::new_from_string(new_color).ok_or(
            "Invalid color format. Use hex string like #RRGGBB or #RRGGBBAA"
        )?;
    }

    broadcast_state_change();
    Ok(())
}
