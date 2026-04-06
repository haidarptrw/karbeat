use crate::{ core::project::{TrackId, mixer::{MixerChannel, MixerState}}, lock::get_app_read };

/// **GETTER: Fetch the mixer state from application state and map it to T value**
pub fn get_mixer_state<T, F>(mapper: F) -> T where F: Fn(&MixerState) -> T {
    let app = get_app_read();
    mapper(&app.mixer)
}

/// **GETTER: Get Specific Mixer Channel and map it to T value
pub fn get_mixer_channel<T, F>(track_id: TrackId, mapper: F) -> anyhow::Result<T> where F: Fn(&MixerChannel) -> T{
    let app = get_app_read();
    let mixer_state = &app.mixer;
    let channel = mixer_state.channels.get(&track_id);
    channel.ok_or_else(|| anyhow::anyhow!("Channel not found")).map(|c| mapper(c.as_ref()))
}