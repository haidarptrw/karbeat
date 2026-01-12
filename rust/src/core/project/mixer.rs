use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    core::project::{ApplicationState, PluginInstance, TrackId},
    define_id,
};

define_id!(EffectId);

/// Custom Error type for better error clarity
///
/// This represents an error that occur due to param setting operation
#[derive(Clone, Debug)]
pub struct MixerSetParamError {
    pub message: String,
    pub track_id: TrackId,
}

impl MixerSetParamError {
    pub fn new(track_id: TrackId, message: &str) -> Self {
        Self {
            track_id,
            message: message.to_string(),
        }
    }
}

pub struct MixerNotFoundError {
    pub message: String,
    pub track_id: TrackId,
}

impl MixerNotFoundError {
    pub fn new(track_id: TrackId, message: &str) -> Self {
        Self {
            track_id,
            message: message.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MixerChannelParams {
    Volume(f32),
    Pan(f32),
    Mute(bool),
    InvertedPhase(bool),
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct MixerState {
    // Map Track ID -> Mixer Channel
    pub channels: HashMap<TrackId, Arc<MixerChannel>>,
    pub master_bus: Arc<MixerChannel>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MixerChannel {
    pub volume: f32, // 0.0 to 1.0 (or dB)
    pub pan: f32,    // -1.0 to 1.0
    pub mute: bool,
    pub solo: bool,
    pub inverted_phase: bool,

    pub effect_counter: u32,

    // The effects chain (EQ, Compressor) comes AFTER the generator
    pub effects: HashMap<EffectId, Arc<PluginInstance>>,
}

impl Default for MixerChannel {
    fn default() -> Self {
        Self {
            volume: 0.0,
            pan: 0.0,
            mute: Default::default(),
            solo: Default::default(),
            effect_counter: 0,
            inverted_phase: Default::default(),
            effects: HashMap::new(),
        }
    }
}

impl MixerState {
    /// Set params of mixer channel besides the effect
    pub fn set_params_mixer_channel(
        &mut self,
        track_id: &TrackId,
        params: &[MixerChannelParams],
    ) -> Result<Arc<MixerChannel>, MixerSetParamError> {
        let mixer_channel_arc = self.channels.get_mut(track_id).ok_or_else(|| {
            MixerSetParamError::new(track_id.clone(), "Cannot find the mixer channel")
        })?;

        let channel = Arc::make_mut(mixer_channel_arc);

        // Check what we are going to change
        for param in params.iter() {
            match param {
                MixerChannelParams::Volume(value) => channel.volume = *value,
                MixerChannelParams::Pan(value) => channel.pan = *value,
                MixerChannelParams::Mute(value) => channel.mute = *value,
                MixerChannelParams::InvertedPhase(value) => channel.inverted_phase = *value,
            }
        }

        Ok(mixer_channel_arc.clone())
    }

    /// Add an effect descriptor to a mixer channel's metadata.
    ///
    /// Note: The actual effect instance should be sent to the audio thread via
    /// `AudioCommand::AddTrackEffect`. This function only updates the metadata.
    pub fn add_effect_descriptor(
        &mut self,
        track_id: &TrackId,
        effect_name: &str,
        internal_type: &str,
    ) -> Result<(), MixerNotFoundError> {
        let mixer_channel_arc = self.channels.get_mut(track_id).ok_or_else(|| {
            MixerNotFoundError::new(track_id.clone(), "Cannot find the mixer channel")
        })?;

        // Clone and modify the channel
        let channel = Arc::make_mut(mixer_channel_arc);
        let effect_id = EffectId::next(&mut channel.effect_counter);
        let effect_instance = PluginInstance::new(effect_name, internal_type);
        channel.effects.insert(effect_id, Arc::new(effect_instance));

        Ok(())
    }
}

impl ApplicationState {
    /// Get the mixer of a track ID
    pub fn get_mixer_from_track(&self, track_id: &TrackId) -> Option<Arc<MixerChannel>> {
        // check if the track exists
        if self.tracks.get(track_id).is_none() {
            return None;
        }

        if let Some(mixer_channel) = self.mixer.channels.get(track_id) {
            let owned_mixer_chan = mixer_channel.to_owned();
            Some(owned_mixer_chan)
        } else {
            None
        }
    }

    /// Get the entire mixer state
    pub fn get_mixer_state(&self) -> &MixerState {
        return &self.mixer;
    }
}
