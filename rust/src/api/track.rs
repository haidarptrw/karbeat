// rust\src\api\track.rs

use crate::{APP_STATE, broadcast_state_change, core::project::Clip};

pub enum UiSourceType {
    Audio,
    Midi,
}

pub fn create_clip(
    source_id: u32,
    source_type: UiSourceType,
    track_id: u32,
    start_time: u32,
) -> Result<(), String> {
    {
        let Ok(mut app) = APP_STATE.write() else {
            return Err("error acquiring write lock for create_clip".to_string());
        };

        match source_type {
            UiSourceType::Audio => {
                // check the source
                let audio_source = app
                    .asset_library
                    .source_map
                    .get(&source_id)
                    .ok_or("The audio source is not available in the library".to_string())?
                    .clone();

                let project_sample_rate = app.audio_config.sample_rate as f64;
                let source_sample_rate = audio_source.sample_rate as f64;

                let source_frames = audio_source.buffer.len() as u64 / audio_source.channels as u64;
                let timeline_length = if source_sample_rate > 0.0 {
                     (source_frames as f64 * (project_sample_rate / source_sample_rate)) as u64
                } else {
                     source_frames // Fallback to avoid division by zero
                };

                app.clip_counter += 1;
                let new_clip_id = app.clip_counter;

                let clip = Clip {
                    name: audio_source.name.clone(),
                    id: new_clip_id, 
                    start_time: start_time as u64,
                    source: crate::core::project::KarbeatSource::Audio(audio_source.clone()),
                    offset_start: 0,
                    loop_length: timeline_length,
                    trim_start: 0,
                    trim_end: 0,
                };

                // let track= app.tracks.get_mut(&track_id).ok_or("The track does not exist".to_string())?;
                // // Add to track
                // let track_mut = Arc::make_mut(track);
                // track_mut.add_clip(Clip {
                //     name: audio_source.name.clone(),
                //     id: new_clip_id, 
                //     start_time: start_time as u64,
                //     source: crate::core::project::KarbeatSource::Audio(audio_source.clone()),
                //     offset_start: 0,
                //     loop_length: length_in_frames,
                //     trim_start: 0,
                //     trim_end: 0,
                // });
                app.add_clip_to_track(track_id, clip);
            }
            UiSourceType::Midi => {},
        }
    }

    broadcast_state_change();

    Ok(())
}
