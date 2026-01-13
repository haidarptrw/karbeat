use std::collections::HashMap;

use crate::{
    core::project::mixer::{EffectInstance, MixerChannel, MixerChannelParams, MixerState},
    utils::lock::{get_app_read, get_app_write},
};

/// ======================================
/// Type Definitions
/// ======================================

/// UI representation of a mixer channel.
pub struct UiMixerChannel {
    pub volume: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub inverted_phase: bool,
    /// List of effect IDs. UI does not need the heavy data object of effect instance
    pub effects: Vec<u32>,
}

impl From<&MixerChannel> for UiMixerChannel {
    fn from(value: &MixerChannel) -> Self {
        Self {
            volume: value.volume,
            pan: value.pan,
            mute: value.mute,
            solo: value.solo,
            inverted_phase: value.inverted_phase,
            effects: value
                .effects
                .iter()
                .map(|instance| instance.id.to_u32())
                .collect(),
        }
    }
}

/// UI representation of the mixer state.
pub struct UiMixerState {
    pub channels: HashMap<u32, UiMixerChannel>,
    pub master_bus: UiMixerChannel,
}

impl From<&MixerState> for UiMixerState {
    fn from(value: &MixerState) -> Self {
        Self {
            channels: value
                .channels
                .iter()
                .map(|(id, channel)| (id.to_u32(), channel.as_ref().into()))
                .collect(),
            master_bus: value.master_bus.as_ref().into(),
        }
    }
}

pub struct UiEffectInstance {
    pub id: u32,
    pub name: String,
    pub internal_type: String,
    pub parameters: HashMap<u32, f32>,
}

impl From<&EffectInstance> for UiEffectInstance {
    fn from(value: &EffectInstance) -> Self {
        Self {
            id: value.id.to_u32(),
            name: value.instance.name.clone(),
            internal_type: value.instance.internal_type.clone(),
            parameters: value.instance.parameters.clone(),
        }
    }
}

pub enum UiMixerChannelParams {
    Volume(f32),
    Pan(f32),
    Mute(bool),
    InvertedPhase(bool),
}

impl Into<UiMixerChannelParams> for &MixerChannelParams {
    fn into(self) -> UiMixerChannelParams {
        match self {
            MixerChannelParams::Volume(value) => UiMixerChannelParams::Volume(*value),
            MixerChannelParams::Pan(value) => UiMixerChannelParams::Pan(*value),
            MixerChannelParams::Mute(value) => UiMixerChannelParams::Mute(*value),
            MixerChannelParams::InvertedPhase(value) => UiMixerChannelParams::InvertedPhase(*value),
        }
    }
}

impl Into<MixerChannelParams> for &UiMixerChannelParams {
    fn into(self) -> MixerChannelParams {
        match self {
            UiMixerChannelParams::Volume(value) => MixerChannelParams::Volume(*value),
            UiMixerChannelParams::Pan(value) => MixerChannelParams::Pan(*value),
            UiMixerChannelParams::Mute(value) => MixerChannelParams::Mute(*value),
            UiMixerChannelParams::InvertedPhase(value) => MixerChannelParams::InvertedPhase(*value),
        }
    }
}

/// ======================================
/// GETTERS
/// ======================================

/// **GETTER: Fetch the mixer state**
///
/// Does not throw error. panic if the app state lock is poisoned
pub fn get_mixer_state() -> UiMixerState {
    let app = get_app_read();

    let mixer_state = &app.mixer;
    mixer_state.into()
}

/// **GETTER: Fetch a specific mixer channel**
///
/// Throws error if the channel is not found
pub fn get_mixer_channel(track_id: u32) -> Result<UiMixerChannel, String> {
    let app = get_app_read();
    let mixer_state = &app.mixer;
    let channel = mixer_state.channels.get(&track_id.into());
    channel
        .ok_or("Channel not found".to_owned())
        .map(|c| c.as_ref().into())
}

/// **GETTER: Fetch the master bus**
///
/// Does not throw error. panic if the app state lock is poisoned
pub fn get_master_bus() -> UiMixerChannel {
    let app = get_app_read();
    let mixer_state = &app.mixer;
    mixer_state.master_bus.as_ref().into()
}

// ======================================
// MIXER ACTIONS AND APIs
// ======================================

pub fn set_master_bus_params(params: Vec<UiMixerChannelParams>) -> Result<(), String> {
    let mut app = get_app_write();
    let mixer_state = &mut app.mixer;
    let params_legit: Vec<MixerChannelParams> = params.iter().map(|p| p.into()).collect();
    mixer_state
        .set_params_master_bus(&params_legit)
        .map_err(|e| e.message)?;
    Ok(())
}

pub fn set_mixer_channel_params(
    track_id: u32,
    params: Vec<UiMixerChannelParams>,
) -> Result<(), String> {
    let mut app = get_app_write();
    let mixer_state = &mut app.mixer;
    let params_legit: Vec<MixerChannelParams> = params.iter().map(|p| p.into()).collect();
    mixer_state
        .set_params_mixer_channel(&track_id.into(), &params_legit)
        .map_err(|e| e.message)?;
    Ok(())
}
