use std::time::Duration;

use karbeat_core::context::utils::send_audio_command;
use karbeat_core::core::file_manager::loader::AudioLoader;
use karbeat_core::core::project::TrackId;
use karbeat_core::{ audio::event::TransportFeedback, commands::AudioCommand, context::ctx };
use crate::api::project::{ AudioWaveformUiForAudioProperties, UiAudioHardwareConfig };
use crate::frb_generated::StreamSink;
use karbeat_core::lock::get_app_read;

#[derive(Clone, Copy, Debug)]
pub struct UiTransportFeedback {
    pub samples: u32,
    pub beat: usize,
    pub bar: usize,
    pub tempo: f32,
    pub sample_rate: u32,
    pub is_playing: bool,
    pub is_looping: bool,
    pub is_recording: bool,
    pub is_pattern_playing: bool,
    pub is_pattern_mode: bool,
    pub pattern_samples: u32,
    pub pattern_beat: usize,
    pub pattern_bar: usize,
}

impl From<TransportFeedback> for UiTransportFeedback {
    fn from(f: TransportFeedback) -> Self {
        Self {
            samples: f.samples,
            beat: f.beat,
            bar: f.bar,
            tempo: f.tempo,
            sample_rate: f.sample_rate,
            is_playing: f.is_playing,
            is_looping: f.is_looping,
            is_recording: f.is_recording,
            is_pattern_playing: f.is_pattern_playing,
            is_pattern_mode: f.is_pattern_mode,
            pattern_samples: f.pattern_samples,
            pattern_beat: f.pattern_beat,
            pattern_bar: f.pattern_bar,
        }
    }
}

/// GETTER: Fetch details + Downsampled Buffer for UI
pub fn get_audio_properties(id: u32) -> Option<AudioWaveformUiForAudioProperties> {
    let app = get_app_read();
    let waveform = app.get_audio_source(id)?;

    //TODO:Add dyanamic downsampling to send this to the frontend side for smaller memory footprint
    // downsample(waveform.buffer)

    Some(AudioWaveformUiForAudioProperties::from(waveform.as_ref()))
}

/// ACTION: Play the sound via the Engine
pub fn play_source_preview(id: u32) {
    let app = get_app_read();

    if let Some(waveform_arc) = app.get_audio_source(id) {
        let waveform_to_play = (*waveform_arc).clone();

        send_audio_command(AudioCommand::PlayOneShot(waveform_to_play));
        log::info!("Preview command sent for ID: {}", id);
    }
}

pub fn stop_all_previews() {
    send_audio_command(AudioCommand::StopAllPreviews);
    println!("Stop all preview sounds");
}

pub fn get_audio_config() -> Result<UiAudioHardwareConfig, String> {
    let app_state = get_app_read();
    Ok(UiAudioHardwareConfig::from(app_state.audio_config.clone()))
}

pub fn create_position_stream(sink: StreamSink<UiTransportFeedback>) -> Result<(), String> {
    // Spawn a thread to poll the ring buffer
    std::thread::spawn(move || {
        loop {
            // Get access to the consumer
            if let Some(consumer) = ctx().position_consumer.lock().as_mut() {
                //Read everything currently in the buffer
                while let Ok(pos_data) = consumer.pop() {
                    if sink.add(UiTransportFeedback::from(pos_data)).is_err() {
                        println!("[Rust] PlaybackPosition Stream disconnected! Stopping thread.");
                        return;
                    }
                }
            }

            // Sleep to prevent high CPU usage on this polling thread
            // 16ms ~= 60fps
            std::thread::sleep(Duration::from_millis(16));
        }
    });
    Ok(())
}

/// play preview sound when drawing note or pressing the piano tile on the UI
pub fn play_preview_note(
    track_id: u32,
    note_key: i32,
    velocity: i32,
    is_on: bool
) -> Result<(), String> {
    // validate input
    if note_key < 0 || note_key > 127 {
        return Err("Note key must be between 0 and 127".to_string());
    }

    if velocity < 0 || velocity > 100 {
        return Err("Note velocity must be between 0 and 100".to_string());
    }

    let note_key: u8 = note_key as u8;

    let velocity: u8 = velocity as u8;

    let generator_id = {
        let app = get_app_read();
        let track = app.tracks.get(&TrackId::from(track_id)).ok_or("Can't find requested track")?;
        track.generator.as_ref().ok_or("Track has no generator")?.id
    };

    if let Some(sender) = ctx().command_sender.lock().as_mut() {
        let _ = sender.push(AudioCommand::PlayPreviewNote {
            note_key: note_key,
            generator_id: generator_id.into(),
            velocity,
            is_note_on: is_on,
        });
    }

    Ok(())
}

/// Play preview sound directly on a generator (without requiring a track).
/// Used in plugin editor screens to test synth sounds.
pub fn play_preview_note_generator(
    generator_id: u32,
    note_key: i32,
    velocity: i32,
    is_on: bool
) -> Result<(), String> {
    // validate input
    if note_key < 0 || note_key > 127 {
        return Err("Note key must be between 0 and 127".to_string());
    }

    if velocity < 0 || velocity > 127 {
        return Err("Note velocity must be between 0 and 127".to_string());
    }

    let note_key: u8 = note_key as u8;
    let velocity: u8 = velocity as u8;

    log::info!("Playing note {}", note_key);

    if let Some(sender) = ctx().command_sender.lock().as_mut() {
        let _ = sender.push(AudioCommand::PlayPreviewNote {
            note_key,
            generator_id: generator_id.into(),
            velocity,
            is_note_on: is_on,
        });
    }

    Ok(())
}
