use std::collections::HashMap;

use crate::{
    core::project::mixer::{MixerChannel, MixerState},
    utils::lock::get_app_read,
};

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

impl From<MixerChannel> for UiMixerChannel {
    fn from(value: MixerChannel) -> Self {
        Self {
            volume: value.volume,
            pan: value.pan,
            mute: value.mute,
            solo: value.solo,
            inverted_phase: value.inverted_phase,
            effects: value.effects.iter().map(|(id, _)| id.to_u32()).collect(),
        }
    }
}

impl Into<UiMixerChannel> for &MixerChannel {
    fn into(self) -> UiMixerChannel {
        UiMixerChannel {
            volume: self.volume,
            pan: self.pan,
            mute: self.mute,
            solo: self.solo,
            inverted_phase: self.inverted_phase,
            effects: self.effects.iter().map(|(id, _)| id.to_u32()).collect(),
        }
    }
}

/// UI representation of the mixer state.
pub struct UiMixerState {
    pub channels: HashMap<u32, UiMixerChannel>,
    pub master_bus: UiMixerChannel,
}

impl From<MixerState> for UiMixerState {
    fn from(value: MixerState) -> Self {
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

impl Into<UiMixerState> for &MixerState {
    fn into(self) -> UiMixerState {
        UiMixerState {
            channels: self
                .channels
                .iter()
                .map(|(id, channel)| (id.to_u32(), channel.as_ref().into()))
                .collect(),
            master_bus: self.master_bus.as_ref().into(),
        }
    }
}

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
// Mixer actions
// ======================================

