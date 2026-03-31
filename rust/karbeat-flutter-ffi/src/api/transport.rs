/// api/transport.rs
/// Transport API - all functions push AudioCommands to the audio thread.
/// BPM is also persisted in ApplicationState for project serialization.

use karbeat_core::{ commands::AudioCommand, context::ctx };
use karbeat_core::lock::get_app_write;

/// helper to push a command to the audio thread
fn push_command(cmd: AudioCommand) {
    if let Some(cmd_producer) = ctx().command_sender.lock().as_mut() {
        let _ = cmd_producer.push(cmd);
    }
}

/// set the play state of the transport
pub fn set_playing(val: bool) -> Result<(), String> {
    push_command(AudioCommand::SetPlaying(val));
    Ok(())
}

/// set what position the playhead is at (in samples)
pub fn set_playhead(val: u32) -> Result<(), String> {
    push_command(AudioCommand::SetPlayhead(val));
    Ok(())
}

/// set whether the transport is looping
pub fn set_looping(val: bool) -> Result<(), String> {
    push_command(AudioCommand::SetLooping(val));
    Ok(())
}

/// set the BPM of the transport.
/// writes to both ApplicationState (for serialization) and AudioCommand (for audio thread)
pub fn set_bpm(val: f32) -> Result<(), String> {
    {
        let mut app = get_app_write();
        app.transport.bpm = val;
    }
    push_command(AudioCommand::SetBPM(val));
    Ok(())
}

/// stop the song playback and reset the playhead to 0
pub fn stop_song_playback() -> Result<(), String> {
    push_command(AudioCommand::StopAndReset);
    Ok(())
}
