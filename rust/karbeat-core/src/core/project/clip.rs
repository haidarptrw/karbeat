use karbeat_utils::define_id;
use std::{cmp::Ordering, sync::Arc};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeEdge {
    Left,
    Right,
}

use crate::core::project::track::audio_waveform::AudioSourceId;
use crate::core::project::track::midi::{Pattern, PatternId};
use crate::core::project::{track::TrackId, track::TrackType, ApplicationState, KarbeatSource};

define_id!(ClipId);

pub enum ClipSourceType {
    Midi,
    Audio,
}

/// Clip struct that holds data for clip in the timeline
/// # Example:
/// ```rust,ignore
/// let clip = Clip {
///     name: "My Clip".to_string(),
///     id: ClipId::new(0),
///     start_time: 0,
///     source: KarbeatSource::Audio,
///     source_id: 0,
///     offset_start: 0,
///     loop_length: 0,
/// };
/// ```
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Clip {
    /// Clip name
    pub name: String,
    /// Clip ID
    pub id: ClipId,
    /// Refer to where it sits on the global timeline
    pub start_time: u32,
    /// Source of the clip
    pub source: KarbeatSource,
    pub offset_start: u32, // currently this does nothing since we set it always to 0
    pub loop_length: u32,  // Refer to length of the entire clip when not shrinked
}

impl PartialEq for Clip {
    fn eq(&self, other: &Self) -> bool {
        self.start_time == other.start_time && self.id == other.id
    }
}

impl Eq for Clip {}

impl PartialOrd for Clip {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Primary ordering by start_time, then by id for tie-breaking
        match self.start_time.cmp(&other.start_time) {
            Ordering::Equal => Some(self.id.cmp(&other.id)),
            ordering => Some(ordering),
        }
    }
}

impl Ord for Clip {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl ApplicationState {
    pub fn add_clip_to_track(
        &mut self,
        track_id: TrackId,
        clip: Clip,
        update_max_sample_index: bool,
    ) -> anyhow::Result<()> {
        // Get the track
        match self.tracks.get_mut(&track_id) {
            Some(track_arc) => {
                // COW: Get mutable track
                let track = Arc::make_mut(track_arc);

                // Add Clip & Check bounds
                // We pass the Clip by value. The track takes ownership and wraps it in Arc.
                let _ = track.add_clip(clip)?;
                if update_max_sample_index {
                    self.update_max_sample_index();
                }
            }
            _ => return Err(anyhow::anyhow!("Track not found")),
        }
        Ok(())
    }

    pub fn delete_clip_from_track(
        &mut self,
        track_id: TrackId,
        clip_id: ClipId,
        update_max_sample_index: bool,
    ) -> anyhow::Result<Arc<Clip>> {
        let deleted_clip = if let Some(track_arc) = self.tracks.get_mut(&track_id) {
            let track = Arc::make_mut(track_arc);
            match track.remove_clip(&clip_id) {
                Ok(clip) => {
                    if update_max_sample_index {
                        self.update_max_sample_index();
                    }
                    Ok(clip)
                }
                Err(e) => Err(e),
            }
        } else {
            Err(anyhow::anyhow!("Track not found"))
        }?;

        Ok(deleted_clip)
    }

    /// Get a clip from a track by its ID.
    /// Returns an owned clone of the Clip if found.
    pub fn get_clip(&self, track_id: &TrackId, clip_id: &ClipId) -> Option<Clip> {
        self.tracks
            .get(track_id)
            .and_then(|track| track.clips.iter().find(|c| c.id == *clip_id))
            .map(|arc_clip| (**arc_clip).clone())
    }

    /// Move a clip from one track to another (or within the same track) with a new start time.
    /// This removes the clip from the source track and adds it to the target track.
    /// Returns an error if the track or clip is not found, or if types are incompatible.
    pub fn move_clip(
        &mut self,
        source_track_id: TrackId,
        target_track_id: TrackId,
        clip_id: ClipId,
        new_start_time: u32,
    ) -> Result<Clip, String> {
        // First, extract the clip from the source track
        let clip = {
            let track_arc = self
                .tracks
                .get_mut(&source_track_id)
                .ok_or("Source track not found")?;
            let track = Arc::make_mut(track_arc);

            let clip_arc = track
                .clips
                .iter()
                .find(|c| c.id == clip_id)
                .cloned()
                .ok_or("Clip not found in source track")?;

            track.clips.remove(&clip_arc);
            track.update_max_sample_index();

            (*clip_arc).clone()
        };

        // Update the clip's start time
        let mut modified_clip = clip;
        modified_clip.start_time = new_start_time;

        // Add the clip to the target track
        {
            let track_arc = self
                .tracks
                .get_mut(&target_track_id)
                .ok_or("Target track not found")?;
            let track = Arc::make_mut(track_arc);

            track
                .add_clip(modified_clip.clone())
                .map_err(|e| e.to_string())?;
        }

        self.update_max_sample_index();
        Ok(modified_clip)
    }

    /// Resize a clip by updating its start_time, offset_start, and loop_length.
    /// Supports both left (slip edit) and right edge resizing.
    /// - `edge`: Which edge is being dragged (Left or Right)
    /// - `new_time_val`: The new timeline position for the dragged edge
    pub fn resize_clip(
        &mut self,
        track_id: TrackId,
        clip_id: ClipId,
        edge: ResizeEdge,
        new_time_val: u32,
    ) -> Result<Clip, String> {
        let track_arc = self.tracks.get_mut(&track_id).ok_or("Track not found")?;
        let track = Arc::make_mut(track_arc);

        // Find and remove the old clip
        let clip_arc = track
            .clips
            .iter()
            .find(|c| c.id == clip_id)
            .cloned()
            .ok_or("Clip not found")?;

        track.clips.remove(&clip_arc);

        let mut modified_clip = (*clip_arc).clone();

        match edge {
            ResizeEdge::Right => {
                // Dragging Right Edge: Only change loop_length
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
                    let delta = new_start as i64 - old_start as i64;

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

        // Re-insert the clip
        track.clips.insert(Arc::new(modified_clip.clone()));
        track.update_max_sample_index();

        self.update_max_sample_index();
        Ok(modified_clip)
    }

    pub fn create_new_clip(
        &mut self,
        source_id: Option<u32>,
        source_type: ClipSourceType,
        track_id: TrackId,
        start_time: u32,
    ) -> anyhow::Result<Clip> {
        let clip = match source_type {
            ClipSourceType::Audio => {
                let source_id =
                    source_id.ok_or_else(|| anyhow::anyhow!("Audio clip needs source id"))?;
                let source_id = AudioSourceId::from(source_id);
                // check the source
                let audio_source = self
                    .asset_library
                    .source_map
                    .get(&source_id)
                    .ok_or_else(|| {
                        anyhow::anyhow!("The audio source is not available in the library")
                    })?
                    .clone();

                let project_sample_rate = self.audio_config.sample_rate as f64;
                let source_sample_rate = audio_source.sample_rate as f64;
                let buffer_len = crate::utils::get_waveform_buffer(&audio_source.buffer)
                    .map(|b| b.len())
                    .unwrap_or(0);
                let source_frames = (buffer_len as u32) / (audio_source.channels as u32);
                let timeline_length = if source_sample_rate > 0.0 {
                    ((source_frames as f64) * (project_sample_rate / source_sample_rate)) as u32
                } else {
                    source_frames // Fallback to avoid division by zero
                };

                let new_clip_id = ClipId::next(&mut self.clip_counter);

                let clip = Clip {
                    name: audio_source.name.clone(),
                    id: new_clip_id,
                    start_time,
                    source: KarbeatSource::Audio(source_id),
                    offset_start: 0,
                    loop_length: timeline_length,
                };
                self.add_clip_to_track(track_id, clip.clone(), true)?;

                clip
            }
            ClipSourceType::Midi => {
                let sample_rate = self.audio_config.sample_rate;
                let bpm = if self.transport.bpm == 0.0 {
                    120.0
                } else {
                    self.transport.bpm
                };
                let samples_per_beat = ((sample_rate as f32) / (bpm / 60.0)) as u32;

                // Use existing pattern if source_id provided, otherwise create new
                let (pattern_id, timeline_length) = if let Some(id) = source_id {
                    let pattern_id = PatternId::from(id);
                    let pattern = self
                        .pattern_pool
                        .get(&pattern_id)
                        .ok_or_else(|| anyhow::anyhow!("Pattern {} not found", id))?;

                    // Calculate length from pattern's ticks
                    let samples_per_tick = (samples_per_beat as f32) / 960.0;
                    let length = ((pattern.length_ticks as f32) * samples_per_tick) as u32;
                    (pattern_id, length)
                } else {
                    // Create new pattern
                    let new_pattern_id = PatternId::next(&mut self.pattern_counter);
                    let default_ticks = 4 * 960;
                    let timeline_length = 4 * samples_per_beat;

                    let pattern = Arc::new(Pattern {
                        id: new_pattern_id,
                        name: format!("Pattern {}", new_pattern_id.to_u32()),
                        length_ticks: default_ticks,
                        notes: Vec::new(),
                        next_note_id: 0,
                    });
                    self.pattern_pool.insert(new_pattern_id, pattern);
                    (new_pattern_id, timeline_length)
                };

                let pattern_name = self
                    .pattern_pool
                    .get(&pattern_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| format!("Pattern {}", pattern_id.to_u32()));

                let new_clip_id = ClipId::next(&mut self.clip_counter);
                let clip = Clip {
                    name: pattern_name,
                    id: new_clip_id,
                    start_time,
                    source: KarbeatSource::Midi(pattern_id),
                    offset_start: 0,
                    loop_length: timeline_length,
                };

                self.add_clip_to_track(track_id, clip.clone(), true)?;
                clip
            }
        };

        Ok(clip)
    }

    pub fn move_clip_batch(
        &mut self,
        source_track_id: TrackId,
        target_track_id: TrackId,
        clip_ids: Vec<ClipId>,
        delta_samples: i64,
    ) -> Result<Vec<Clip>, String> {
        let mut result_clips = Vec::new();
        let target_type = if let Some(target) = self.tracks.get(&target_track_id) {
            target.track_type.clone()
        } else {
            return Err("Target track not found".to_string());
        };

        if source_track_id == target_track_id {
            // Same track: just update start times
            let track_arc = self
                .tracks
                .get_mut(&source_track_id)
                .ok_or("Source track not found")?;
            let track = Arc::make_mut(track_arc);

            for clip_id in &clip_ids {
                if let Some(clip) = track.clips.iter().find(|c| c.id == *clip_id).cloned() {
                    track.clips.remove(&clip);
                    let mut modified_clip = (*clip).clone();
                    // Apply delta with clamping to 0
                    let new_start =
                        ((modified_clip.start_time as i64) + delta_samples).max(0) as u32;
                    modified_clip.start_time = new_start;
                    track.clips.insert(Arc::new(modified_clip.clone()));
                    result_clips.push(modified_clip);
                }
            }
            track.update_max_sample_index();
        } else {
            // Cross-track move
            let mut clips_to_move = Vec::new();
            {
                let source_track = Arc::make_mut(
                    self.tracks
                        .get_mut(&source_track_id)
                        .ok_or("Source track not found")?,
                );

                for clip_id in &clip_ids {
                    if let Some(clip) = source_track
                        .clips
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
            }

            // Add to target track
            let target_track = Arc::make_mut(
                self.tracks
                    .get_mut(&target_track_id)
                    .ok_or("Target track not found")?,
            );
            for clip in clips_to_move {
                let mut modified_clip = (*clip).clone();
                let new_start = ((modified_clip.start_time as i64) + delta_samples).max(0) as u32;
                modified_clip.start_time = new_start;
                let _ = target_track.add_clip(modified_clip.clone());
                result_clips.push(modified_clip);
            }
        }
        self.update_max_sample_index();
        Ok(result_clips)
    }

    pub fn resize_clip_batch(
        &mut self,
        track_id: TrackId,
        clip_ids: Vec<ClipId>,
        edge: ResizeEdge,
        delta_samples: i64,
    ) -> Result<Vec<Clip>, String> {
        let track_arc = self.tracks.get_mut(&track_id).ok_or("Track not found")?;
        let track = Arc::make_mut(track_arc);

        let mut result_clips = Vec::new();

        for clip_id in &clip_ids {
            if let Some(clip) = track.clips.iter().find(|c| c.id == *clip_id).cloned() {
                track.clips.remove(&clip);
                let mut modified_clip = (*clip).clone();

                match edge {
                    ResizeEdge::Right => {
                        let current_end = modified_clip.start_time + modified_clip.loop_length;
                        let new_end = ((current_end as i64) + delta_samples)
                            .max((modified_clip.start_time as i64) + 100)
                            as u32;
                        modified_clip.loop_length = new_end - modified_clip.start_time;
                    }
                    ResizeEdge::Left => {
                        let old_start = modified_clip.start_time;
                        let old_end = old_start + modified_clip.loop_length;
                        let new_start = ((old_start as i64) + delta_samples)
                            .clamp(0, (old_end as i64) - 100)
                            as u32;

                        let delta = (new_start as i64) - (old_start as i64);
                        let current_offset = modified_clip.offset_start as i64;
                        let new_offset = (current_offset + delta).max(0) as u32;

                        modified_clip.start_time = new_start;
                        modified_clip.loop_length = old_end - new_start;
                        modified_clip.offset_start = new_offset;
                    }
                }

                track.clips.insert(Arc::new(modified_clip.clone()));
                result_clips.push(modified_clip);
            }
        }
        track.update_max_sample_index();
        self.update_max_sample_index();
        Ok(result_clips)
    }
}
