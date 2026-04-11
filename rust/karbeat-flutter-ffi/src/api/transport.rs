//! api/transport.rs
//! Transport API - all functions push AudioCommands to the audio thread.
//! BPM is also persisted in ApplicationState for project serialization.

use karbeat_core::api::transport_api;

/// set the play state of the transport
pub fn set_playing(val: bool) -> Result<(), String> {
    transport_api::set_playing(val);
    Ok(())
}

/// set what position the playhead is at (in samples)
pub fn set_playhead(val: u32) -> Result<(), String> {
    transport_api::set_playhead(val);
    Ok(())
}

/// set whether the transport is looping
pub fn set_looping(val: bool) -> Result<(), String> {
    transport_api::set_looping(val);
    Ok(())
}

/// set the BPM of the transport.
/// writes to both ApplicationState (for serialization) and AudioCommand (for audio thread)
pub fn set_bpm(val: f32) -> Result<(), String> {
    transport_api::set_bpm(val);
    Ok(())
}

/// stop the song playback and reset the playhead to 0
pub fn stop_song_playback() -> Result<(), String> {
    transport_api::stop_song_playback();
    Ok(())
}
