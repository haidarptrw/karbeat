// rust\src\api\track.rs

use std::sync::Arc;

use crate::{broadcast_state_change, core::project::Clip, APP_STATE};

pub enum UiSourceType {
    Audio,
    Midi,
}

pub enum ResizeEdge {
    Left,
    Right,
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
                    source_id: source_id,
                };
                app.add_clip_to_track(track_id, clip);
            }
            UiSourceType::Midi => {}
        }
    }

    broadcast_state_change();

    Ok(())
}

pub fn delete_clip(track_id: u32, clip_id: u32) -> Result<(), String> {
    {
        let Ok(mut app) = APP_STATE.write() else {
            return Err("error acquiring write lock for create_clip".to_string());
        };

        app.delete_clip_from_track(track_id, clip_id);
    }

    broadcast_state_change();

    Ok(())
}

pub fn resize_clip(
    track_id: u32,
    clip_id: u32,
    edge: ResizeEdge,
    new_time_val: u64,
) -> Result<(), String> {
    {
        let mut app = APP_STATE.write().map_err(|_| "Failed to lock state")?;
        let track_arc = app.tracks.get_mut(&track_id).ok_or("Track not found")?;

        let track = Arc::make_mut(track_arc);

        let clips = Arc::make_mut(&mut track.clips);

        if let Some(clip) = clips.iter().find(|c| c.id == clip_id).cloned() {
            clips.remove(&clip);

            let mut modified_clip = (*clip).clone();

            match edge {
                ResizeEdge::Right => {
                    if new_time_val > modified_clip.start_time {
                        let new_length = new_time_val - modified_clip.start_time;
                        modified_clip.loop_length = new_length;
                    }
                }
                ResizeEdge::Left => {
                    // Dragging Left Edge: Slip Edit
                    let old_start = modified_clip.start_time;
                    let old_end = old_start + modified_clip.loop_length;

                    // Bound check: New Start cannot be past the old End
                    if new_time_val < old_end {
                        let new_start = new_time_val;

                        // Calculate delta (positive = trimmed right, negative = expanded left)
                        let delta = new_start as i64 - old_start as i64;

                        let current_offset = modified_clip.offset_start as i64;
                        let new_offset = current_offset + delta;

                        // Constraint: offset cannot be negative (can't start before 0 of source)
                        if new_offset >= 0 {
                            modified_clip.start_time = new_start;
                            // Length shrinks as start moves right (or grows as it moves left)
                            modified_clip.loop_length = old_end - new_start;
                            modified_clip.offset_start = new_offset as u64;
                        }
                    }
                }
            }

            clips.insert(Arc::new(modified_clip));
            track.update_max_sample_index();

        } else {
            return Err("Clip not found".to_string());
        }

        // update since there is a modification of a clip
        app.update_max_sample_index();
    }

    broadcast_state_change();
    Ok(())
}
