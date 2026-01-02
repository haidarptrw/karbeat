use std::{cmp::Ordering, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    core::project::{track::TrackId, ApplicationState, KarbeatSource},
    define_id,
};

define_id!(ClipId);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Clip {
    pub name: String,
    pub id: ClipId,
    /// Refer to where it sits on the global timeline
    pub start_time: u64,
    pub source: KarbeatSource,
    pub source_id: u32,
    pub offset_start: u64, // currently this does nothing since we set it always to 0
    pub loop_length: u64,  // Refer to length of the entire clip when not shrinked
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

    pub fn delete_clip_from_track(&mut self, track_id: TrackId, clip_id: ClipId) {
        if let Some(track_arc) = self.tracks.get_mut(&track_id) {
            let track = Arc::make_mut(track_arc);
            if track.remove_clip(clip_id) {
                // Only recompute global max if that track actually changed
                self.update_max_sample_index();
            }
        }
    }
}
