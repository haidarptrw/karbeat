use crate::{
    audio::event::TransportFeedback,
    commands::AudioCommand,
    context::{ctx, utils::send_audio_command},
    core::{
        file_manager::audio_loader::AudioLoader,
        project::{ AudioHardwareConfig, AudioSourceId, AudioWaveform, GeneratorId, TrackId },
    },
    lock::get_app_read,
};

pub fn get_audio_source<T, F>(id: AudioSourceId, mapper: F) -> Option<T>
    where F: FnOnce(&AudioWaveform) -> T
{
    let app = get_app_read();
    app.get_audio_source(&id).map(|w| mapper(w.as_ref()))
}

pub fn play_source_preview(id: AudioSourceId) -> anyhow::Result<()> {
    let app = get_app_read();
    if let Some(waveform_arc) = app.get_audio_source(&id) {
        let waveform_to_play = (*waveform_arc).clone();
        send_audio_command(AudioCommand::PlayOneShot(waveform_to_play));
        Ok(())
    } else {
        Err(anyhow::anyhow!("Audio source not found"))
    }
}

pub fn stop_all_previews() {
    send_audio_command(AudioCommand::StopAllPreviews);
}

pub fn get_audio_config<T, F>(mapper: F) -> T where F: FnOnce(&AudioHardwareConfig) -> T {
    let app = get_app_read();
    mapper(&app.audio_config)
}

/// Drains the position feedback ring buffer and maps it to UI types
pub fn drain_position_feedback<T, F>(mut mapper: F) -> Vec<T> where F: FnMut(TransportFeedback) -> T {
    let mut results = Vec::new();
    if let Some(consumer) = ctx().position_consumer.lock().as_mut() {
        while let Ok(pos_data) = consumer.pop() {
            results.push(mapper(pos_data));
        }
    }
    results
}

pub fn play_preview_note(
    track_id: TrackId,
    note_key: u8,
    velocity: u8,
    is_on: bool
) -> anyhow::Result<()> {
    let generator_id = {
        let app = get_app_read();
        let track = app.tracks
            .get(&track_id)
            .ok_or_else(|| anyhow::anyhow!("Can't find requested track"))?;
        track.generator.as_ref().ok_or_else(|| anyhow::anyhow!("Track has no generator"))?.id
    };

    if let Some(sender) = ctx().command_sender.lock().as_mut() {
        let _ = sender.push(AudioCommand::PlayPreviewNote {
            note_key,
            generator_id,
            velocity,
            is_note_on: is_on,
        });
    }

    Ok(())
}

pub fn play_preview_note_generator(
    generator_id: GeneratorId,
    note_key: u8,
    velocity: u8,
    is_on: bool
) -> anyhow::Result<()> {
    if let Some(sender) = ctx().command_sender.lock().as_mut() {
        let _ = sender.push(AudioCommand::PlayPreviewNote {
            note_key,
            generator_id,
            velocity,
            is_note_on: is_on,
        });
    }

    Ok(())
}
