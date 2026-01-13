use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    commands::AudioCommand,
    core::project::{ApplicationState, PluginInstance, TrackId},
    ctx, define_id,
};

define_id!(EffectId);

/// Custom Error type for better error clarity
///
/// This represents an error that occur due to param setting operation
#[derive(Error, Debug, Clone)] // Added Error
#[error("Mixer param error for track {track_id}: {message}")]
pub struct MixerSetParamError {
    pub message: String,
    pub track_id: TrackId,
}

#[derive(Error, Debug, Clone)] // Added Error
#[error("Effect creation error: {message}")]
pub struct EffectCreationError {
    pub message: String,
}

impl MixerSetParamError {
    pub fn new(track_id: TrackId, message: &str) -> Self {
        Self {
            track_id,
            message: message.to_string(),
        }
    }
}

#[derive(Error, Debug)]
#[error("Mixer not found for track {track_id}: {message}")] //
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
pub struct EffectInstance {
    pub id: EffectId,
    pub instance: Arc<PluginInstance>,
}

impl EffectInstance {
    pub fn new(id: EffectId, instance: PluginInstance) -> Self {
        Self {
            id,
            instance: Arc::new(instance),
        }
    }
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
    pub effects: Vec<EffectInstance>,
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
            effects: Vec::new(),
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

    pub fn set_params_master_bus(
        &mut self,
        params: &[MixerChannelParams],
    ) -> Result<Arc<MixerChannel>, MixerSetParamError> {
        let mut master_bus_channel = self.master_bus.clone();
        let channel = Arc::make_mut(&mut master_bus_channel);

        // Check what we are going to change
        for param in params.iter() {
            match param {
                MixerChannelParams::Volume(value) => channel.volume = *value,
                MixerChannelParams::Pan(value) => channel.pan = *value,
                MixerChannelParams::Mute(value) => channel.mute = *value,
                MixerChannelParams::InvertedPhase(value) => channel.inverted_phase = *value,
            }
        }

        Ok(master_bus_channel)
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
    ) -> anyhow::Result<()> {
        let mixer_channel_arc = self
            .channels
            .get_mut(track_id)
            .ok_or_else(|| {
                MixerNotFoundError::new(track_id.clone(), "Cannot find the mixer channel")
            })
            .map_err(|e| anyhow::anyhow!(e))?;

        // Clone and modify the channel
        let channel = Arc::make_mut(mixer_channel_arc);
        let effect_id = EffectId::next(&mut channel.effect_counter);

        let (effect_plugin, default_params) = {
            let registry = ctx().plugin_registry.read().unwrap();
            if let Some(effect_box) = registry.create_effect(effect_name) {
                let default_params = effect_box.default_parameters();
                (effect_box, default_params)
            } else {
                let message = format!("Generator '{}' not found in registry", effect_name);
                log::error!("{}", message);
                // Decrement counters if failed to prevent gaps/orphans
                channel.effect_counter -= 1;
                return Err(anyhow::anyhow!(EffectCreationError { message }));
            }
        };

        // Push to the audio thread
        if let Some(sender) = ctx().command_sender.lock().unwrap().as_mut() {
            let _ = sender.push(AudioCommand::AddTrackEffect {
                track_id: track_id.clone(),
                effect_id,
                effect: effect_plugin,
            });
        }
        
        let plugin_instance = PluginInstance::new(effect_name, internal_type);
        
        let effect_instance = EffectInstance {
            id: effect_id,
            instance: Arc::new(plugin_instance),
        };  
        
        channel.effects.push(effect_instance);

        Ok(())
    }

    pub fn get_effects(
        &self,
        track_id: &TrackId,
    ) -> Result<Vec<EffectInstance>, MixerNotFoundError> {
        let mut mixer_channel_arc = self
            .channels
            .get(track_id)
            .ok_or_else(|| {
                MixerNotFoundError::new(track_id.clone(), "Cannot find the mixer channel")
            })?
            .to_owned();

        // Clone and modify the channel
        let channel = Arc::make_mut(&mut mixer_channel_arc);
        Ok(channel.effects.clone())
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
