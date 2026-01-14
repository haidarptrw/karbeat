use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    commands::AudioCommand,
    core::project::{plugin::KarbeatEffect, ApplicationState, PluginInstance, TrackId},
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

impl MixerChannel {
    pub fn add_effect(
        &mut self,
        effect_registry_id: u32,
    ) -> anyhow::Result<(Box<dyn KarbeatEffect + Send + Sync>, String, EffectId)> {
        let effect_id = EffectId::next(&mut self.effect_counter);

        let (effect_plugin, effect_name, default_params) = {
            let registry = ctx().plugin_registry.read().unwrap();
            if let Some((effect_box, name)) = registry.create_effect_by_id(effect_registry_id) {
                let default_params = effect_box.default_parameters();
                (effect_box, name, default_params)
            } else {
                let message = format!(
                    "Effect with ID {} not found in registry",
                    effect_registry_id
                );
                log::error!("{}", message);
                // Decrement counters if failed to prevent gaps/orphans
                self.effect_counter -= 1;

                return Err(anyhow::anyhow!(message));
            }
        };

        let plugin_instance =
            PluginInstance::new_with_params(effect_registry_id, &effect_name, default_params);

        let effect_instance = EffectInstance::new(effect_id, plugin_instance);
        self.effects.push(effect_instance);

        Ok((effect_plugin, effect_name, effect_id))
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

    // set the master bus params
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

    /// Add an effect descriptor to a mixer channel by its registry ID.
    pub fn add_effect_descriptor_by_id(
        &mut self,
        track_id: &TrackId,
        registry_id: u32,
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

        let (effect_plugin, effect_name, effect_id) = channel.add_effect(registry_id)?;

        // // Push to the audio thread
        if let Some(sender) = ctx().command_sender.lock().unwrap().as_mut() {
            let _ = sender.push(AudioCommand::AddTrackEffect {
                track_id: track_id.clone(),
                effect_id,
                effect: effect_plugin,
            });
        }

        log::info!(
            "Effect {} (registry_id={}) added to track {:?}",
            effect_name,
            registry_id,
            track_id
        );

        Ok(())
    }

    /// Add an effect descriptor to a mixer channel by name (backwards compatible).
    /// Internally looks up the registry ID and delegates to the ID-based method.
    ///
    /// Note: The `internal_type` parameter is deprecated and ignored.
    #[deprecated(note = "Use add_effect_descriptor_by_id instead")]
    pub fn add_effect_descriptor(
        &mut self,
        track_id: &TrackId,
        effect_name: &str,
        _internal_type: &str,
    ) -> anyhow::Result<()> {
        // Look up the registry ID by name
        let registry_id = {
            let registry = ctx().plugin_registry.read().unwrap();
            registry
                .get_effect_id_by_name(effect_name)
                .ok_or_else(|| anyhow::anyhow!("Effect '{}' not found in registry", effect_name))?
        };

        // Delegate to ID-based method
        self.add_effect_descriptor_by_id(track_id, registry_id)
    }

    /// Get all effect instances from a mixer channel
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

    pub fn add_effect_to_master_bus(&mut self, registry_id: u32) -> anyhow::Result<()> {
        let mut master_bus_arc = self.master_bus.clone();
        let channel = Arc::make_mut(&mut master_bus_arc);
        let (effect_plugin, effect_name, effect_id) = channel.add_effect(registry_id)?;

        if let Some(sender) = ctx().command_sender.lock().unwrap().as_mut() {
            let _ = sender.push(AudioCommand::AddMasterEffect {
                effect_id,
                effect: effect_plugin,
            });
        }

        log::info!(
            "Effect {} (registry_id={}) added to master bus",
            effect_name,
            registry_id
        );
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
