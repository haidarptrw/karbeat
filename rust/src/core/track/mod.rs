// src/core/track/mod.rs

use std::sync::Arc;

use crate::core::project::{ApplicationState, KarbeatTrack, TrackType};

pub mod audio_waveform;

impl ApplicationState {
    pub fn add_new_track(&mut self, track_type: TrackType) {
        let new_track_id = self.track_counter;
        let new_track = KarbeatTrack {
            track_type,
            id: new_track_id,
            ..Default::default()
        };
        self.tracks.insert(new_track_id, Arc::new(new_track));

        // increment track_counter
        self.track_counter += 1;
    }
}