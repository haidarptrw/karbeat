use std::{cmp::Ordering, sync::Arc};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeEdge {
    Left,
    Right,
}

use crate::core::project::track::midi::Pattern;
use crate::core::project::ClipboardContent;
use crate::core::project::{track::TrackType, ApplicationState, DawSource};
use crate::shared::id::{ClipId, TrackId};
use crate::shared::{AudioSourceId, PatternId};

pub enum ClipSourceType {
    Midi,
    Audio,
}

/// ======================================
/// ClipTimeUnit
/// Encapsulates clip timeline positioning with explicit units.
/// Audio clips use raw samples (BPM-independent).
/// MIDI/Automation clips use ticks (960 PPQN, BPM-dependent).
/// ======================================
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClipTimeUnit {
    /// Values in raw audio samples (BPM-independent)
    Samples {
        start_time: u64,
        loop_length: u64,
        offset_start: u64,
    },
    /// Values in ticks (960 PPQN, BPM-dependent)
    Ticks {
        start_time: u32,
        loop_length: u32,
        offset_start: u32,
    },
}

impl Default for ClipTimeUnit {
    fn default() -> Self {
        Self::Samples {
            start_time: 0,
            loop_length: 0,
            offset_start: 0,
        }
    }
}

impl ClipTimeUnit {
    /// Get the start time in samples, converting ticks if necessary
    pub fn start_time_samples(&self, bpm: f32, sample_rate: u32) -> u64 {
        match self {
            ClipTimeUnit::Samples { start_time, .. } => *start_time,
            ClipTimeUnit::Ticks { start_time, .. } => {
                let samples_per_tick = Self::samples_per_tick(bpm, sample_rate);
                ((*start_time as f64) * samples_per_tick) as u64
            }
        }
    }

    /// Get the loop length in samples, converting ticks if necessary
    pub fn loop_length_samples(&self, bpm: f32, sample_rate: u32) -> u64 {
        match self {
            ClipTimeUnit::Samples { loop_length, .. } => *loop_length,
            ClipTimeUnit::Ticks { loop_length, .. } => {
                let samples_per_tick = Self::samples_per_tick(bpm, sample_rate);
                ((*loop_length as f64) * samples_per_tick) as u64
            }
        }
    }

    /// Get the offset start in samples, converting ticks if necessary
    pub fn offset_start_samples(&self, bpm: f32, sample_rate: u32) -> u64 {
        match self {
            ClipTimeUnit::Samples { offset_start, .. } => *offset_start,
            ClipTimeUnit::Ticks { offset_start, .. } => {
                let samples_per_tick = Self::samples_per_tick(bpm, sample_rate);
                ((*offset_start as f64) * samples_per_tick) as u64
            }
        }
    }

    /// Get end position (start_time + loop_length) in samples
    pub fn end_samples(&self, bpm: f32, sample_rate: u32) -> u64 {
        self.start_time_samples(bpm, sample_rate) + self.loop_length_samples(bpm, sample_rate)
    }

    /// Get start_time in the native unit (u64 for either variant)
    pub fn start_time_raw(&self) -> u64 {
        match self {
            ClipTimeUnit::Samples { start_time, .. } => *start_time,
            ClipTimeUnit::Ticks { start_time, .. } => *start_time as u64,
        }
    }

    /// Get loop_length in the native unit (u64 for either variant)
    pub fn loop_length_raw(&self) -> u64 {
        match self {
            ClipTimeUnit::Samples { loop_length, .. } => *loop_length,
            ClipTimeUnit::Ticks { loop_length, .. } => *loop_length as u64,
        }
    }

    /// Get offset_start in the native unit (u64 for either variant)
    pub fn offset_start_raw(&self) -> u64 {
        match self {
            ClipTimeUnit::Samples { offset_start, .. } => *offset_start,
            ClipTimeUnit::Ticks { offset_start, .. } => *offset_start as u64,
        }
    }

    /// Returns true if this is SamplesBased
    pub fn is_samples(&self) -> bool {
        matches!(self, ClipTimeUnit::Samples { .. })
    }

    /// Helper: compute samples per tick from BPM and sample_rate
    pub fn samples_per_tick(bpm: f32, sample_rate: u32) -> f64 {
        let bpm = if bpm <= 0.0 { 120.0 } else { bpm };
        let samples_per_beat = (60.0 / (bpm as f64)) * (sample_rate as f64);
        samples_per_beat / 960.0
    }
}

/// Clip struct that holds data for clip in the timeline.
/// Positioning uses ClipTimeUnit to distinguish between sample-based (audio)
/// and tick-based (MIDI/automation) clips.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Clip {
    /// Clip name
    pub name: String,
    /// Clip ID
    pub id: ClipId,
    /// Source of the clip
    pub source: Option<DawSource>,
    /// Timeline position and length — explicit units via enum
    pub time: ClipTimeUnit,
}

impl PartialEq for Clip {
    fn eq(&self, other: &Self) -> bool {
        self.time.start_time_raw() == other.time.start_time_raw() && self.id == other.id
    }
}

impl Eq for Clip {}

impl PartialOrd for Clip {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Primary ordering by start_time, then by id for tie-breaking
        match self.time.start_time_raw().cmp(&other.time.start_time_raw()) {
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
    pub fn add_clip_to_track(&mut self, track_id: TrackId, clip: Clip) -> anyhow::Result<()> {
        // Get the track
        match self.tracks.get_mut(&track_id) {
            Some(track_arc) => {
                // COW: Get mutable track
                let track: &mut super::AudioTrack = Arc::make_mut(track_arc);

                // Add Clip & Check bounds
                // We pass the Clip by value. The track takes ownership and wraps it in Arc.
                let _ = track.add_clip(clip)?;
            }
            _ => {
                return Err(anyhow::anyhow!("Track not found"));
            }
        }
        Ok(())
    }

    pub fn delete_clip_from_track(
        &mut self,
        track_id: TrackId,
        clip_id: ClipId,
    ) -> anyhow::Result<Arc<Clip>> {
        let deleted_clip = (if let Some(track_arc) = self.tracks.get_mut(&track_id) {
            let track = Arc::make_mut(track_arc);
            match track.remove_clip(&clip_id) {
                Ok(clip) => Ok(clip),
                Err(e) => Err(e),
            }
        } else {
            Err(anyhow::anyhow!("Track not found"))
        })?;

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
    /// The new_start_time is in the clip's native unit (samples for audio, ticks for MIDI).
    /// Returns an error if the track or clip is not found, or if types are incompatible.
    pub fn move_clip(
        &mut self,
        source_track_id: TrackId,
        target_track_id: TrackId,
        clip_id: ClipId,
        new_start_time: u64,
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

            (*clip_arc).clone()
        };

        // Update the clip's start time (preserving the time unit variant)
        let mut modified_clip = clip;
        match &mut modified_clip.time {
            ClipTimeUnit::Samples { start_time, .. } => {
                *start_time = new_start_time;
            }
            ClipTimeUnit::Ticks { start_time, .. } => {
                *start_time = new_start_time as u32;
            }
        }

        // Add the clip to the target track
        {
            let track_arc = self
                .tracks
                .get_mut(&target_track_id)
                .ok_or("Target track not found")?;
            let track: &mut super::AudioTrack = Arc::make_mut(track_arc);

            track
                .add_clip(modified_clip.clone())
                .map_err(|e| e.to_string())?;
        }

        Ok(modified_clip)
    }

    /// Cut (Slice) a clip at a given point (in the clip's native time unit).
    /// Returns the two resulting clips.
    pub fn slice_clip(
        &mut self,
        track_id: &TrackId,
        clip_id: &ClipId,
        cut_point: u64,
    ) -> Result<(Clip, Clip), String> {
        let track_arc = self.tracks.get_mut(track_id).ok_or("Track not found")?;
        let track = Arc::make_mut(track_arc);

        // Find and remove the old clip
        let clip_arc = track
            .clips
            .iter()
            .find(|c| c.id == *clip_id)
            .cloned()
            .ok_or("Clip not found")?;

        let source_clip = (*clip_arc).clone();

        let start = source_clip.time.start_time_raw();
        let length = source_clip.time.loop_length_raw();
        let offset = source_clip.time.offset_start_raw();

        if cut_point <= start || cut_point >= start + length {
            return Err("Cut point is out of range of clip boundaries".to_string());
        }

        let first_clip_length = cut_point - start;

        // Create first clip (same start, shortened length)
        let mut first_clip = source_clip.clone();
        match &mut first_clip.time {
            ClipTimeUnit::Samples { loop_length, .. } => {
                *loop_length = first_clip_length;
            }
            ClipTimeUnit::Ticks { loop_length, .. } => {
                *loop_length = first_clip_length as u32;
            }
        }

        // Create second clip (starts at cut point, offset adjusted)
        let second_clip_id = ClipId::next(&mut self.clip_counter);
        let mut second_clip = source_clip.clone();
        second_clip.id = second_clip_id;
        let second_clip_length = length - first_clip_length;
        let second_clip_offset = offset + first_clip_length;
        match &mut second_clip.time {
            ClipTimeUnit::Samples {
                start_time,
                loop_length,
                offset_start,
            } => {
                *start_time = cut_point;
                *loop_length = second_clip_length;
                *offset_start = second_clip_offset;
            }
            ClipTimeUnit::Ticks {
                start_time,
                loop_length,
                offset_start,
            } => {
                *start_time = cut_point as u32;
                *loop_length = second_clip_length as u32;
                *offset_start = second_clip_offset as u32;
            }
        }

        track.clips.remove(&clip_arc);
        track.clips.insert(Arc::new(first_clip.clone()));
        track.clips.insert(Arc::new(second_clip.clone()));

        self.update_max_sample_index();

        Ok((first_clip, second_clip))
    }

    /// Resize a clip by updating its time fields.
    /// Supports both left (slip edit) and right edge resizing.
    /// - `edge`: Which edge is being dragged (Left or Right)
    /// - `new_time_val`: The new timeline position for the dragged edge (in native units)
    pub fn resize_clip(
        &mut self,
        track_id: TrackId,
        clip_id: ClipId,
        edge: ResizeEdge,
        new_time_val: u64,
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
        let old_start = modified_clip.time.start_time_raw();
        let old_length = modified_clip.time.loop_length_raw();
        let old_end = old_start + old_length;
        let old_offset = modified_clip.time.offset_start_raw();

        match edge {
            ResizeEdge::Right => {
                // Dragging Right Edge: Only change loop_length
                if new_time_val > old_start {
                    let new_length = new_time_val - old_start;
                    match &mut modified_clip.time {
                        ClipTimeUnit::Samples { loop_length, .. } => {
                            *loop_length = new_length;
                        }
                        ClipTimeUnit::Ticks { loop_length, .. } => {
                            *loop_length = new_length as u32;
                        }
                    }
                }
            }
            ResizeEdge::Left => {
                // Dragging Left Edge: Slip Edit
                // Bound check: New Start cannot be past the old End
                if new_time_val < old_end {
                    let delta = (new_time_val as i64) - (old_start as i64);
                    let new_offset = (old_offset as i64) + delta;

                    // Constraint: offset cannot be negative
                    if new_offset >= 0 {
                        let new_length = old_end - new_time_val;
                        match &mut modified_clip.time {
                            ClipTimeUnit::Samples {
                                start_time,
                                loop_length,
                                offset_start,
                            } => {
                                *start_time = new_time_val;
                                *loop_length = new_length;
                                *offset_start = new_offset as u64;
                            }
                            ClipTimeUnit::Ticks {
                                start_time,
                                loop_length,
                                offset_start,
                            } => {
                                *start_time = new_time_val as u32;
                                *loop_length = new_length as u32;
                                *offset_start = new_offset as u32;
                            }
                        }
                    }
                }
            }
        }

        // Re-insert the clip
        track.clips.insert(Arc::new(modified_clip.clone()));

        self.update_max_sample_index();
        Ok(modified_clip)
    }

    /// Create a new clip and add it to the track.
    /// - `start_time`: Position on the timeline in ticks (from the UI).
    ///   For audio clips, this will be converted to samples at creation time.
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
                let source_frames = (buffer_len as u64) / (audio_source.channels as u64);

                // clip length in samples (resampled to project sample rate)
                let timeline_length_samples = if source_sample_rate > 0.0 {
                    ((source_frames as f64) * (project_sample_rate / source_sample_rate)) as u64
                } else {
                    source_frames // Fallback to avoid division by zero
                };

                // Convert the incoming start_time (ticks) to samples
                let bpm = if self.transport.bpm == 0.0 {
                    120.0
                } else {
                    self.transport.bpm
                };
                let samples_per_tick =
                    ClipTimeUnit::samples_per_tick(bpm, self.audio_config.sample_rate);
                let start_time_samples = ((start_time as f64) * samples_per_tick) as u64;

                let new_clip_id = ClipId::next(&mut self.clip_counter);

                let clip = Clip {
                    name: audio_source.name.clone(),
                    id: new_clip_id,
                    source: Some(DawSource::Audio(source_id)),
                    time: ClipTimeUnit::Samples {
                        start_time: start_time_samples,
                        loop_length: timeline_length_samples,
                        offset_start: 0,
                    },
                };
                self.add_clip_to_track(track_id, clip.clone())?;

                clip
            }
            ClipSourceType::Midi => {
                // Use existing pattern if source_id provided, otherwise create new
                let (pattern_id, timeline_length) = if let Some(id) = source_id {
                    let pattern_id = PatternId::from(id);
                    let pattern = self
                        .pattern_pool
                        .get(&pattern_id)
                        .ok_or_else(|| anyhow::anyhow!("Pattern {} not found", id))?;

                    // Length is in ticks
                    let length = pattern.length_ticks as u32;
                    (pattern_id, length)
                } else {
                    // Create new pattern
                    let new_pattern_id = PatternId::next(&mut self.pattern_counter);
                    let default_ticks = 4 * 960;
                    let timeline_length = 4 * 960; // 4 beats

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
                    source: Some(DawSource::Midi(pattern_id)),
                    time: ClipTimeUnit::Ticks {
                        start_time,
                        loop_length: timeline_length,
                        offset_start: 0,
                    },
                };

                self.add_clip_to_track(track_id, clip.clone())?;
                clip
            }
        };

        self.update_max_sample_index();

        Ok(clip)
    }

    /// Batch move clips by a delta (in native units).
    pub fn move_clip_batch(
        &mut self,
        source_track_id: TrackId,
        target_track_id: TrackId,
        clip_ids: Vec<ClipId>,
        delta: i64,
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
                    let old_start = modified_clip.time.start_time_raw() as i64;
                    let new_start = (old_start + delta).max(0);
                    match &mut modified_clip.time {
                        ClipTimeUnit::Samples { start_time, .. } => {
                            *start_time = new_start as u64;
                        }
                        ClipTimeUnit::Ticks { start_time, .. } => {
                            *start_time = new_start as u32;
                        }
                    }
                    track.clips.insert(Arc::new(modified_clip.clone()));
                    result_clips.push(modified_clip);
                }
            }
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
                            (TrackType::Audio, Some(DawSource::Audio(_))) => true,
                            (TrackType::Midi, Some(DawSource::Midi(_))) => true,
                            _ => false,
                        };
                        if !is_compatible {
                            continue; // Skip incompatible clips
                        }
                        source_track.clips.remove(&clip);
                        clips_to_move.push(clip);
                    }
                }
            }

            // Add to target track
            let target_track = Arc::make_mut(
                self.tracks
                    .get_mut(&target_track_id)
                    .ok_or("Target track not found")?,
            );
            for clip in clips_to_move {
                let mut modified_clip = (*clip).clone();
                let old_start = modified_clip.time.start_time_raw() as i64;
                let new_start = (old_start + delta).max(0);
                match &mut modified_clip.time {
                    ClipTimeUnit::Samples { start_time, .. } => {
                        *start_time = new_start as u64;
                    }
                    ClipTimeUnit::Ticks { start_time, .. } => {
                        *start_time = new_start as u32;
                    }
                }
                let _ = target_track.add_clip(modified_clip.clone());
                result_clips.push(modified_clip);
            }
        }
        self.update_max_sample_index();
        Ok(result_clips)
    }

    /// Batch resize clips by a delta (in native units).
    pub fn resize_clip_batch(
        &mut self,
        track_id: TrackId,
        clip_ids: Vec<ClipId>,
        edge: ResizeEdge,
        delta: i64,
    ) -> Result<Vec<Clip>, String> {
        let track_arc = self.tracks.get_mut(&track_id).ok_or("Track not found")?;
        let track = Arc::make_mut(track_arc);

        let mut result_clips = Vec::new();

        for clip_id in &clip_ids {
            if let Some(clip) = track.clips.iter().find(|c| c.id == *clip_id).cloned() {
                track.clips.remove(&clip);
                let mut modified_clip = (*clip).clone();

                let old_start = modified_clip.time.start_time_raw();
                let old_length = modified_clip.time.loop_length_raw();
                let old_end = old_start + old_length;
                let old_offset = modified_clip.time.offset_start_raw();

                match edge {
                    ResizeEdge::Right => {
                        let new_end =
                            ((old_end as i64) + delta).max((old_start as i64) + 10) as u64;
                        let new_length = new_end - old_start;
                        match &mut modified_clip.time {
                            ClipTimeUnit::Samples { loop_length, .. } => {
                                *loop_length = new_length;
                            }
                            ClipTimeUnit::Ticks { loop_length, .. } => {
                                *loop_length = new_length as u32;
                            }
                        }
                    }
                    ResizeEdge::Left => {
                        let new_start =
                            ((old_start as i64) + delta).clamp(0, (old_end as i64) - 10) as u64;

                        let d = (new_start as i64) - (old_start as i64);
                        let new_offset = ((old_offset as i64) + d).max(0) as u64;
                        let new_length = old_end - new_start;

                        match &mut modified_clip.time {
                            ClipTimeUnit::Samples {
                                start_time,
                                loop_length,
                                offset_start,
                            } => {
                                *start_time = new_start;
                                *loop_length = new_length;
                                *offset_start = new_offset;
                            }
                            ClipTimeUnit::Ticks {
                                start_time,
                                loop_length,
                                offset_start,
                            } => {
                                *start_time = new_start as u32;
                                *loop_length = new_length as u32;
                                *offset_start = new_offset as u32;
                            }
                        }
                    }
                }

                track.clips.insert(Arc::new(modified_clip.clone()));
                result_clips.push(modified_clip);
            }
        }
        self.update_max_sample_index();
        Ok(result_clips)
    }

    pub fn copy_clip_batch(
        &mut self,
        source_track_id: TrackId,
        clip_ids: &[ClipId],
    ) -> anyhow::Result<()> {
        let track_arc = self
            .tracks
            .get(&source_track_id)
            .ok_or_else(|| anyhow!("Track not found"))?;

        let copied_clips: Vec<_> = track_arc
            .clips
            .iter()
            .filter_map(|clip| {
                if clip_ids.contains(&clip.id) {
                    Some(clip.as_ref().clone())
                } else {
                    None
                }
            })
            .collect();

        if copied_clips.is_empty() {
            self.clipboard = ClipboardContent::Empty;
        } else {
            self.clipboard = ClipboardContent::Clips(copied_clips);
        }

        Ok(())
    }

    pub fn paste_clip_batch(
        &mut self,
        target_track_id: TrackId,
        start_pos: ClipTimeUnit,
    ) -> anyhow::Result<Vec<Clip>> {
        let copied_clips = match &self.clipboard {
            ClipboardContent::Clips(clips) => clips.clone(),
            _ => return Err(anyhow::anyhow!("Clipboard does not contain clips")),
        };

        if copied_clips.is_empty() {
            return Err(anyhow::anyhow!("Clipboard is empty"));
        }

        let min_time_raw = copied_clips
            .iter()
            .map(|c| c.time.start_time_raw())
            .min()
            .unwrap_or(0);

        let target_time_raw = start_pos.start_time_raw();
        let delta = (target_time_raw as i64) - (min_time_raw as i64);

        let mut pasted_clips = Vec::with_capacity(copied_clips.len());

        // Process and shift each clip
        for mut clip in copied_clips {
            // Assign a fresh ID
            clip.id = ClipId::next(&mut self.clip_counter);

            // Calculate new start time, ensuring it doesn't drop below 0
            let old_start = clip.time.start_time_raw() as i64;
            let new_start = (old_start + delta).max(0) as u64;

            match &mut clip.time {
                ClipTimeUnit::Samples { start_time, .. } => *start_time = new_start,
                ClipTimeUnit::Ticks { start_time, .. } => *start_time = new_start as u32,
            }

            // Attempt to add to the track
            self.add_clip_to_track(target_track_id, clip.clone())?;
            pasted_clips.push(clip);
        }

        self.update_max_sample_index();

        Ok(pasted_clips)
    }

    pub fn cut_clipboard_clip_batch(
        &mut self,
        source_track_id: TrackId,
        clip_ids: &[ClipId],
    ) -> Vec<Clip> {
        let _ = self.copy_clip_batch(source_track_id, clip_ids);

        // Delete them from the track
        let mut deleted_clips = Vec::new();
        if let Some(track_arc) = self.tracks.get_mut(&source_track_id) {
            let track = Arc::make_mut(track_arc);

            // Collect the Arc references that match the targeted IDs
            let clips_to_remove: Vec<_> = track
                .clips
                .iter()
                .filter(|c| clip_ids.contains(&c.id))
                .cloned()
                .collect();

            // Remove them from the underlying BTreeSet/HashSet
            for clip_arc in clips_to_remove {
                if track.clips.remove(&clip_arc) {
                    deleted_clips.push((*clip_arc).clone());
                }
            }
        }

        self.update_max_sample_index();

        deleted_clips
    }
}
