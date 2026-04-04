// rust\src\api\track.rs

use std::collections::HashMap;
use std::sync::Arc;

use crate::api::project::{AudioWaveformUiForClip, UiClip, UiTrack};
use crate::broadcast_state_change;
use karbeat_core::core::file_manager::audio_loader::AudioLoader;
use karbeat_core::core::project::clip::ResizeEdge;
use karbeat_core::core::{
    project::{
        clip::ClipId,
        track::{TrackId, TrackType},
        KarbeatSource,
    },
};
use karbeat_core::lock::{get_app_read, get_app_write};
use karbeat_core::api::clip as clip_api;
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

pub fn get_audio_waveform_clips_data() -> Result<HashMap<u32, AudioWaveformUiForClip>, String> {
    let app = get_app_read();

    let map = app
        .get_audio_sources()
        .iter()
        .map(|(&id, arc_waveform)| {
            let ui = AudioWaveformUiForClip::from(arc_waveform.as_ref());
            (id.to_u32(), ui)
        })
        .collect();

    Ok(map)
}

pub fn get_audio_waveform_for_clip(audio_source_id: u32) -> Result<AudioWaveformUiForClip, String> {
    let app = get_app_read();
    let audio_waveform = app.get_audio_source(audio_source_id).ok_or(format!(
        "Cannot get the audio source with id {}",
        audio_source_id
    ))?;
    let audio_waveform_dto = AudioWaveformUiForClip::from(audio_waveform.as_ref());
    Ok(audio_waveform_dto)
}

/// Getter for all audio waveform data for audio only for this specific track
pub fn get_audio_waveform_for_clip_only_in_specific_track(
    track_id: u32,
) -> Result<HashMap<u32, AudioWaveformUiForClip>, String> {
    let app = get_app_read();

    // get specific track
    let track = app
        .tracks
        .get(&TrackId::from(track_id))
        .ok_or(format!("Track not found"))?
        .as_ref();

    // ** Iterate through tracks and fetch audio waveform for every audio clip **
    let TrackType::Audio = track.track_type else {
        return Ok(HashMap::new()); // Return empty since it is not a audio track
    };

    let return_map: HashMap<u32, AudioWaveformUiForClip> = track
        .clips()
        .iter()
        .filter_map(|c| {
            // Get source Id from clip
            let KarbeatSource::Audio(id) = c.source else {
                return None;
            };

            let id_u32 = id.to_u32();
            let waveform_dto =
                AudioWaveformUiForClip::try_from_audio_waveform_with_target_sample_bin_internal(
                    &app,
                    id.to_u32(),
                )
                .ok()?;

            Some((id_u32, waveform_dto))
        })
        .collect();

    Ok(return_map)
}

/// Getter for all audio waveform data for audio in all audio tracks
pub fn get_audio_waveform_for_clip_all_available_in_tracks(
) -> Result<HashMap<u32, AudioWaveformUiForClip>, String> {
    let app = get_app_read();

    let mut return_map: HashMap<u32, AudioWaveformUiForClip> = HashMap::new();

    for track in app.tracks.values() {
        let track = track.as_ref();

        // Only process audio tracks
        let TrackType::Audio = track.track_type else {
            continue;
        };

        for clip in track.clips().iter() {
            let KarbeatSource::Audio(id) = clip.source else {
                continue;
            };

            let id_u32 = id.to_u32();

            // Avoid duplicate processing
            if return_map.contains_key(&id_u32) {
                continue;
            }

            let Some(audio_source) = app.get_audio_source(id_u32) else {
                continue;
            };

            let waveform_dto = AudioWaveformUiForClip::from(audio_source.as_ref());

            return_map.insert(id_u32, waveform_dto);
        }
    }

    Ok(return_map)
}

pub fn create_clip(
    source_id: Option<u32>,
    source_type: UiSourceType,
    track_id: u32,
    start_time: u32,
) -> Result<UiClip, String> {
    let track_id = TrackId::from(track_id);

    let core_source_type = match source_type {
        UiSourceType::Audio => karbeat_core::core::project::clip::ClipSourceType::Audio,
        UiSourceType::Midi => karbeat_core::core::project::clip::ClipSourceType::Midi,
    };

    let clip = clip_api::add_clip(source_id, core_source_type, track_id, start_time)
        .map_err(|e| format!("{}", e))?;

    let ui_clip = UiClip::from(&clip);
    broadcast_state_change();
    Ok(ui_clip)
}

pub fn delete_clip(track_id: u32, clip_id: u32) -> Result<(), String> {
    let track_id = TrackId::from(track_id);
    let clip_id = ClipId::from(clip_id);

    clip_api::delete_clip(track_id, clip_id)
        .map_err(|e| format!("Failed to delete clip: {}", e))?;

    broadcast_state_change();
    Ok(())
}

pub fn resize_clip(
    track_id: u32,
    clip_id: u32,
    edge: UiResizeEdge,
    new_time_val: u32,
) -> Result<UiClip, String> {
    let track_id = TrackId::from(track_id);
    let clip_id = ClipId::from(clip_id);
    let core_edge: ResizeEdge = edge.into();

    let res = clip_api::resize_clip(track_id, clip_id, core_edge, new_time_val)
        .map_err(|e| format!("{}", e))?;

    broadcast_state_change();
    Ok(UiClip::from(&res))
}

pub fn move_clip(
    source_track_id: u32,
    clip_id: u32,
    new_start_time: u32,
    new_track_id: Option<u32>,
) -> Result<UiClip, String> {
    let source_track_id = TrackId::from(source_track_id);
    let clip_id = ClipId::from(clip_id);
    let target_track_id = new_track_id.map(TrackId::from).unwrap_or(source_track_id);

    let res = clip_api::move_clip(source_track_id, target_track_id, clip_id, new_start_time)
        .map_err(|e| format!("{}", e))?;

    broadcast_state_change();
    Ok(UiClip::from(&res))
}

/// Cut a clip in half.
/// This will retain the original clip at the left cut region,
/// while the right cut region will clone a new clip with the same source,
/// but with the offset at the cut point
///
/// # Parameters
///
/// - source_track_id: Track where clip resides
/// - clip_id: The cut clip id inside the track
/// - cut_point_sample: Absolute sample point of cut location
pub fn cut_clip(
    source_track_id: u32,
    clip_id: u32,
    cut_point_sample: u32,
) -> Result<Vec<UiClip>, String> {
    let source_track_id_typed = TrackId::from(source_track_id);
    let clip_id_typed = ClipId::from(clip_id);

    let (c1, c2) = clip_api::cut_clip(source_track_id_typed, clip_id_typed, cut_point_sample)
            .map_err(|e| format!("{}", e))?;

    broadcast_state_change();
    Ok(vec![UiClip::from(&c1), UiClip::from(&c2)])
}

/// Add a MIDI track with a generator by its registry ID (preferred method).
pub fn add_midi_track_with_generator_id(registry_id: u32) -> Result<UiTrack, String> {
    let res = {
        let mut app = get_app_write();
        app.add_new_midi_track_with_generator_id(registry_id)
            .map_err(|e| format!("{}", e))?
    };
    broadcast_state_change();
    Ok(UiTrack::from(res.as_ref()))
}

/// Add a MIDI track with a generator by name (backwards compatible).
pub fn add_midi_track_with_generator(generator_name: String) -> Result<UiTrack, String> {
    let res = {
        let mut app = get_app_write();
        app.add_new_midi_track_with_generator(&generator_name)
            .map_err(|e| format!("{}", e))?
    };
    broadcast_state_change();
    Ok(UiTrack::from(res.as_ref()))
}

pub fn get_clip(track_id: u32, clip_id: u32) -> Result<UiClip, String> {
    let track_id = TrackId::from(track_id);
    let clip_id = ClipId::from(clip_id);

    let app = get_app_read();

    let track = app
        .tracks
        .get(&track_id)
        .ok_or(format!("Track {:?} not found", track_id))?;

    let clip = track.clips.iter().find(|c| c.id == clip_id).ok_or(format!(
        "Clip {:?} not found in track {:?}",
        clip_id, track_id
    ))?;

    Ok(UiClip::from(clip.as_ref()))
}

// Alternatively, fetching the whole Track is often useful too and still cheaper than all tracks
pub fn get_track(track_id: u32) -> Result<UiTrack, String> {
    let track_id = TrackId::from(track_id);

    let app = get_app_read();
    let track = app
        .tracks
        .get(&track_id)
        .ok_or(format!("Track {:?} not found", track_id))?;

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
    new_track_id: Option<u32>,
) -> Result<Vec<UiClip>, String> {
    let source_track_id = TrackId::from(source_track_id);
    let target_track_id = new_track_id.map(TrackId::from).unwrap_or(source_track_id);
    let clip_ids: Vec<ClipId> = clip_ids.into_iter().map(ClipId::from).collect();

    let res = clip_api::batch_move_clips(source_track_id, target_track_id, clip_ids, delta_samples)
        .map_err(|e| format!("{}", e))?;

    broadcast_state_change();
    Ok(res.iter().map(|c| UiClip::from(c)).collect())
}

/// Resize clips in batch by a delta amount
pub fn resize_clip_batch(
    track_id: u32,
    clip_ids: Vec<u32>,
    edge: UiResizeEdge,
    delta_samples: i64,
) -> Result<Vec<UiClip>, String> {
    let track_id = TrackId::from(track_id);
    let clip_ids: Vec<ClipId> = clip_ids.into_iter().map(ClipId::from).collect();
    let core_edge: ResizeEdge = edge.into();

    let res = clip_api::batch_resize_clips(track_id, clip_ids, core_edge, delta_samples)
        .map_err(|e| format!("{}", e))?;

    broadcast_state_change();
    Ok(res.iter().map(|c| UiClip::from(c)).collect())
}

/// Delete clips in batch
pub fn delete_clip_batch(track_id: u32, clip_ids: Vec<u32>) -> Result<(), String> {
    let track_id = TrackId::from(track_id);
    let clip_ids: Vec<ClipId> = clip_ids.into_iter().map(ClipId::from).collect();

    clip_api::batch_delete_clips(track_id, clip_ids)
        .map_err(|e| format!("Failed to delete clips: {}", e))?;

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
        track.color = Color::new_from_string(new_color)
            .ok_or("Invalid color format. Use hex string like #RRGGBB or #RRGGBBAA")?;
    }

    broadcast_state_change();
    Ok(())
}
