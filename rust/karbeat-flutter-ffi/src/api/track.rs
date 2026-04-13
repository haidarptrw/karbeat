// rust\src\api\track.rs

use std::collections::HashMap;

use crate::api::project::{ AudioWaveformUiForClip, UiClip, UiTrack };
use karbeat_core::core::project::AudioSourceId;
use karbeat_core::core::project::clip::ResizeEdge;
use karbeat_core::shared::id::*;
use karbeat_core::api::{
    clip_api as clip_api,
    audio_waveform_api as audio_waveform_api,
    track_api as track_api,
};

pub enum UiSourceType {
    Audio,
    Midi,
}

pub enum UiResizeEdge {
    Left,
    Right,
}

impl From<ResizeEdge> for UiResizeEdge {
    fn from(value: ResizeEdge) -> Self {
        match value {
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
    audio_waveform_api
        ::get_audio_waveform_clips_data(|id, waveform| (
            id.to_u32(),
            AudioWaveformUiForClip::from(waveform),
        ))
        .map_err(|e| e.to_string())
}

pub fn get_audio_waveform_for_clip(audio_source_id: u32) -> Result<AudioWaveformUiForClip, String> {
    let audio_waveform = audio_waveform_api
        ::get_audio_waveform_for_clip(&AudioSourceId::from(audio_source_id))
        .map_err(|e| e.to_string())?;
    let audio_waveform_dto = AudioWaveformUiForClip::from(audio_waveform.as_ref());
    Ok(audio_waveform_dto)
}

/// Getter for all audio waveform data for audio only for this specific track
pub fn get_audio_waveform_for_clip_only_in_specific_track(
    track_id: u32
) -> Result<HashMap<u32, AudioWaveformUiForClip>, String> {
    let return_map = audio_waveform_api
        ::get_audio_waveform_for_clip_only_in_specific_track(
            &TrackId::from(track_id),
            |id, waveform| (id.to_u32(), AudioWaveformUiForClip::from(waveform))
        )
        .ok_or("Can't find any clip")?;

    Ok(return_map)
}

/// Getter for all audio waveform data for audio in all audio tracks
pub fn get_audio_waveform_for_clip_all_available_in_tracks() -> Result<
    HashMap<u32, AudioWaveformUiForClip>,
    String
> {
    audio_waveform_api
        ::get_audio_waveform_for_clip_all_available_in_tracks(|id, waveform| {
            (id, AudioWaveformUiForClip::from(waveform))
        })
        .map_err(|e| e.to_string())
}

pub fn create_clip(
    source_id: Option<u32>,
    source_type: UiSourceType,
    track_id: u32,
    start_time: u32
) -> Result<UiClip, String> {
    let track_id = TrackId::from(track_id);
    let core_source_type = match source_type {
        UiSourceType::Audio => karbeat_core::core::project::clip::ClipSourceType::Audio,
        UiSourceType::Midi => karbeat_core::core::project::clip::ClipSourceType::Midi,
    };

    let clip = clip_api
        ::add_clip(source_id, core_source_type, track_id, start_time)
        .map_err(|e| format!("{}", e))?;

    let ui_clip = UiClip::from(&clip);
    Ok(ui_clip)
}

pub fn delete_clip(track_id: u32, clip_id: u32) -> Result<(), String> {
    let track_id = TrackId::from(track_id);
    let clip_id = ClipId::from(clip_id);

    clip_api::delete_clip(track_id, clip_id).map_err(|e| format!("Failed to delete clip: {}", e))?;

    Ok(())
}

pub fn resize_clip(
    track_id: u32,
    clip_id: u32,
    edge: UiResizeEdge,
    new_time_val: u32
) -> Result<UiClip, String> {
    let track_id = TrackId::from(track_id);
    let clip_id = ClipId::from(clip_id);
    let core_edge: ResizeEdge = edge.into();

    let res = clip_api
        ::resize_clip(track_id, clip_id, core_edge, new_time_val)
        .map_err(|e| format!("{}", e))?;

    Ok(UiClip::from(&res))
}

pub fn move_clip(
    source_track_id: u32,
    clip_id: u32,
    new_start_time: u32,
    new_track_id: Option<u32>
) -> Result<UiClip, String> {
    let source_track_id = TrackId::from(source_track_id);
    let clip_id = ClipId::from(clip_id);
    let target_track_id = new_track_id.map(TrackId::from).unwrap_or(source_track_id);

    let res = clip_api
        ::move_clip(source_track_id, target_track_id, clip_id, new_start_time)
        .map_err(|e| format!("{}", e))?;

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
    cut_point_sample: u32
) -> Result<Vec<UiClip>, String> {
    let source_track_id_typed = TrackId::from(source_track_id);
    let clip_id_typed = ClipId::from(clip_id);

    let (c1, c2) = clip_api
        ::cut_clip(source_track_id_typed, clip_id_typed, cut_point_sample)
        .map_err(|e| format!("{}", e))?;

    Ok(vec![UiClip::from(&c1), UiClip::from(&c2)])
}

/// Add a MIDI track with a generator by its registry ID (preferred method).
pub fn add_midi_track_with_generator_id(registry_id: u32) -> Result<UiTrack, String> {
    let res = track_api::add_midi_track_with_generator_id(registry_id).map_err(|e| e.to_string())?;
    Ok(UiTrack::from(res.as_ref()))
}

pub fn get_clip(track_id: u32, clip_id: u32) -> Result<UiClip, String> {
    clip_api
        ::get_clip(TrackId::from(track_id), ClipId::from(clip_id), |c| UiClip::from(c))
        .map_err(|e| e.to_string())
}

// Alternatively, fetching the whole Track is often useful too and still cheaper than all tracks
pub fn get_track(track_id: u32) -> Result<UiTrack, String> {
    track_api::get_track(TrackId::from(track_id), |t| UiTrack::from(t)).map_err(|e| e.to_string())
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
) -> Result<Vec<UiClip>, String> {
    let source_track_id = TrackId::from(source_track_id);
    let target_track_id = new_track_id.map(TrackId::from).unwrap_or(source_track_id);
    let clip_ids: Vec<ClipId> = clip_ids.into_iter().map(ClipId::from).collect();

    let res = clip_api
        ::batch_move_clips(source_track_id, target_track_id, clip_ids, delta_samples)
        .map_err(|e| format!("{}", e))?;

    Ok(
        res
            .iter()
            .map(UiClip::from)
            .collect()
    )
}

/// Resize clips in batch by a delta amount
pub fn resize_clip_batch(
    track_id: u32,
    clip_ids: Vec<u32>,
    edge: UiResizeEdge,
    delta_samples: i64
) -> Result<Vec<UiClip>, String> {
    let track_id = TrackId::from(track_id);
    let clip_ids: Vec<ClipId> = clip_ids.into_iter().map(ClipId::from).collect();
    let core_edge: ResizeEdge = edge.into();
    let res = clip_api
        ::batch_resize_clips(track_id, clip_ids, core_edge, delta_samples)
        .map_err(|e| format!("{}", e))?;

    Ok(
        res
            .iter()
            .map(UiClip::from)
            .collect()
    )
}

/// Delete clips in batch
pub fn delete_clip_batch(track_id: u32, clip_ids: Vec<u32>) -> Result<(), String> {
    let track_id = TrackId::from(track_id);
    let clip_ids: Vec<ClipId> = clip_ids.into_iter().map(ClipId::from).collect();
    clip_api
        ::batch_delete_clips(track_id, clip_ids)
        .map_err(|e| format!("Failed to delete clips: {}", e))?;

    Ok(())
}

pub fn change_track_name(track_id: u32, new_name: &str) -> Result<(), String> {
    track_api::change_track_name(TrackId::from(track_id), new_name).map_err(|e| e.to_string())?;
    Ok(())
}

/// Change the track header's color to a new color specified by a hex string (e.g. "#RRGGBB" or "#RRGGBBAA").
pub fn change_track_color(track_id: u32, new_color: &str) -> Result<(), String> {
    track_api::change_track_color(TrackId::from(track_id), new_color).map_err(|e| e.to_string())?;
    Ok(())
}
