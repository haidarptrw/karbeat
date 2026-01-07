use std::{cmp::Ordering, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    api::track::ResizeEdge, core::project::{ApplicationState, KarbeatSource, track::TrackId}, define_id
};

define_id!(ClipId);

/// Clip struct that holds data for clip in the timeline
/// # Example:
/// ```rust
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
        self.partial_cmp(other).unwrap()
    }
}

impl ApplicationState {
    pub fn add_clip_to_track(&mut self, track_id: TrackId, clip: Clip) {
        // Get the track
        if let Some(track_arc) = self.tracks.get_mut(&track_id) {
            // COW: Get mutable track
            let track = Arc::make_mut(track_arc);

            // Add Clip & Check bounds
            // We pass the Clip by value. The track takes ownership and wraps it in Arc.
            if let Ok(_) = track.add_clip(clip) {
                // 4. Update Global Max (Cheap u64 comparison)
                self.update_max_sample_index();
            }
        }
    }

    pub fn delete_clip_from_track(
        &mut self,
        track_id: TrackId,
        clip_id: ClipId,
    ) -> anyhow::Result<Arc<Clip>> {
        if let Some(track_arc) = self.tracks.get_mut(&track_id) {
            let track = Arc::make_mut(track_arc);
            match track.remove_clip(&clip_id) {
                Ok(clip) => {
                    self.update_max_sample_index();
                    Ok(clip)
                }
                Err(e) => Err(e),
            }
        } else {
            Err(anyhow::anyhow!("Track not found"))
        }
    }

    /// Get a clip from a track by its ID.
    /// Returns an owned clone of the Clip if found.
    pub fn get_clip(&self, track_id: TrackId, clip_id: ClipId) -> Option<Clip> {
        self.tracks
            .get(&track_id)
            .and_then(|track| track.clips.iter().find(|c| c.id == clip_id))
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
    ) -> Result<(), String> {
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

            track.add_clip(modified_clip).map_err(|e| e.to_string())?;
        }

        self.update_max_sample_index();
        Ok(())
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
    ) -> Result<(), String> {
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
        track.clips.insert(Arc::new(modified_clip));
        track.update_max_sample_index();

        self.update_max_sample_index();
        Ok(())
    }
}
