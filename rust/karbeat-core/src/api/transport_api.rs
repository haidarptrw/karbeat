use crate::commands::AudioCommand;
use crate::context::utils::{broadcast_state_change, send_audio_command};
use crate::lock::get_app_write;

pub fn set_playing(val: bool) {
    send_audio_command(AudioCommand::SetPlaying(val));
}

pub fn set_playhead(val: u32) {
    send_audio_command(AudioCommand::SetPlayhead(val));
}

pub fn set_looping(val: bool) {
    send_audio_command(AudioCommand::SetLooping(val));
}

pub fn set_bpm(val: f32) {
    {
        let mut app = get_app_write();
        app.transport.bpm = val;
    }
    send_audio_command(AudioCommand::SetBPM(val));
    broadcast_state_change();
}

pub fn stop_song_playback() {
    send_audio_command(AudioCommand::StopAndReset);
}