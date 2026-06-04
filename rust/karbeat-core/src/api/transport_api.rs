use crate::audio::engine::PlaybackMode;
use crate::commands::AudioCommand;
use crate::context::utils::{
    broadcast_track_graph, send_audio_command, try_send_audio_command_chain,
};
use crate::lock::get_app_write;
use crate::shared::{GeneratorId, PatternId};

/// Set is playing for song mode
pub fn set_playing(val: bool) -> anyhow::Result<()> {
    try_send_audio_command_chain(vec![
        AudioCommand::SetPlaybackMode(PlaybackMode::Song),
        AudioCommand::SetPlaying(val),
    ])?;

    Ok(())
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
        // Audio clips (ClipTimeUnit::Samples) are BPM-independent — no rescaling needed.
        // MIDI clips (ClipTimeUnit::Ticks) are converted at playback time, so max_sample_index
        // must be recalculated when BPM changes.
        app.update_max_sample_index();
    }
    send_audio_command(AudioCommand::SetBPM(val));
    // max_sample_index changes with BPM — push the updated track graph snapshot
    broadcast_track_graph();
}

pub fn stop_song_playback() {
    send_audio_command(AudioCommand::StopAndReset);
}

pub fn toggle_pattern_playback(pattern_id: PatternId, generator_id: GeneratorId) {
    send_audio_command(AudioCommand::TogglePatternPlayback {
        pattern_id,
        generator_id,
    });
}

pub fn toggle_playing_with_playback(playback_mode: PlaybackMode) {
    send_audio_command(AudioCommand::TogglePlayingWithPlaybackMode(playback_mode));
}

pub fn switch_pattern_generator(generator_id: GeneratorId) {
    send_audio_command(AudioCommand::SwitchPatternGenerator(generator_id));
}
