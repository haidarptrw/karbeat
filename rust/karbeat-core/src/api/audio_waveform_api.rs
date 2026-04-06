use std::{collections::HashSet, sync::Arc};

use anyhow::Context;

use crate::{
    core::{
        file_manager::audio_loader::AudioLoader,
        project::{ AudioSourceId, AudioWaveform, KarbeatSource, TrackId, TrackType },
    },
    lock::{get_app_read, get_app_write},
};

/// Get audio waveform clips data from Application and map them into U value
pub fn get_audio_waveform_clips_data<C, U, M>(mapper: M) -> anyhow::Result<C>
    where M: Fn(&AudioSourceId, &AudioWaveform) -> U, C: FromIterator<U>
{
    let app = get_app_read();
    let map = app
        .get_audio_sources()
        .iter()
        .map(|(id, audio_waveform)| { mapper(id, audio_waveform) })
        .collect();
    Ok(map)
}

pub fn get_audio_waveform_for_clip(
    audio_source_id: &AudioSourceId
) -> anyhow::Result<Arc<AudioWaveform>> {
    let app = get_app_read();
    let audio_waveform = app
        .get_audio_source(audio_source_id)
        .ok_or_else(|| anyhow::anyhow!("Cannot get the audio source with id {}", audio_source_id))?;

    Ok(audio_waveform.clone())
}

pub fn get_audio_waveform_for_clip_only_in_specific_track<C, U, M>(
    track_id: &TrackId,
    mapper: M
)
    -> Option<C>
    where M: Fn(&AudioSourceId, &AudioWaveform) -> U, C: FromIterator<U>
{
    let app = get_app_read();
    let track = app.tracks
        .get(track_id)?
        .as_ref();

    let TrackType::Audio = track.track_type else {
        return Some(std::iter::empty().collect()); // Return empty since it is not a audio track
    };

    let return_map = track
        .clips()
        .iter()
        .filter_map(|c| {
            // Get source Id from clip
            let KarbeatSource::Audio(id) = c.source else {
                return None;
            };

            let audio_waveform = app.get_audio_source(&id)?;
            Some(mapper(&id, &audio_waveform))
        })
        .collect();

    Some(return_map)
}

pub fn get_audio_waveform_for_clip_all_available_in_tracks<C, U, M>(mapper: M) -> anyhow::Result<C>
where
    M: Fn(u32, &AudioWaveform) -> U,
    C: FromIterator<U>,
{
    let app = get_app_read();
    let mut processed = HashSet::new();

    let return_col = app.tracks.values()
        .filter(|t| matches!(t.track_type, TrackType::Audio))
        .flat_map(|t| t.clips().iter())
        .filter_map(|clip| {
            if let KarbeatSource::Audio(id) = clip.source {
                let id_u32 = id.to_u32();
                if processed.insert(id_u32) { // Prevents duplicate IDs natively
                    if let Some(audio_source) = app.get_audio_source(&id) {
                        return Some(mapper(id_u32, audio_source.as_ref()));
                    }
                }
            }
            None
        })
        .collect::<C>();

    Ok(return_col)
}

pub fn get_audio_source_list<C, U, M>(mapper: M) -> anyhow::Result<C>
where 
    M: Fn(u32, &AudioWaveform) -> U, 
    C: FromIterator<U> 
{
    let app = get_app_read();
    Ok(app.asset_library.source_map.iter().map(|(&id, wf)| mapper(id.to_u32(), wf.as_ref())).collect())
}

pub fn add_audio_source(file_path: &str) -> anyhow::Result<u32> {
    let mut app = get_app_write();
    let id = app.load_audio(file_path, None)?;
    Ok(id)
}

pub fn get_audio_waveform<T, F>(source_id: u32, mapper: F) -> anyhow::Result<T>
where F: Fn(&AudioWaveform) -> T {
    let app = get_app_read();
    let waveform = app.get_audio_source(&AudioSourceId::from(source_id))
        .ok_or_else(|| anyhow::anyhow!("Cannot find audio source"))?;
    Ok(mapper(waveform.as_ref()))
}