use crate::context::utils::broadcast_state_change;
use crate::core::history::ProjectAction;
use crate::core::project::clip::{Clip, ClipSourceType, ResizeEdge};
use crate::core::project::clipboard::ClipboardContent;
use crate::lock::{get_app_read, get_app_write, get_history_lock};
use crate::shared::id::*;
use std::sync::Arc;

pub fn get_clip<T, F>(track_id: TrackId, clip_id: ClipId, mapper: F) -> anyhow::Result<T>
where
    F: FnOnce(&Clip) -> T,
{
    let app = get_app_read();
    let track = app.tracks.get(&track_id).ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;
    
    // Uses your recently fixed `get_clip` which returns `Option<Arc<Clip>>`
    let clip = track.get_clip(&clip_id).ok_or_else(|| anyhow::anyhow!("Clip {:?} not found in track {:?}", clip_id, track_id))?;
    
    Ok(mapper(clip.as_ref()))
}

pub fn add_clip(
    source_id: Option<u32>,
    source_type: ClipSourceType,
    track_id: TrackId,
    start_time: u32,
) -> anyhow::Result<Clip> {
    // 1. Mutate state
    let clip = {
        let mut app = get_app_write();
        app.create_new_clip(source_id, source_type, track_id, start_time)?
    };

    // 2. Update history
    {
        let mut history = get_history_lock();
        history.push(ProjectAction::AddClip {
            track_id,
            clip: clip.clone(),
        });
    }

    broadcast_state_change();

    Ok(clip)
}

pub fn delete_clip(track_id: TrackId, clip_id: ClipId) -> anyhow::Result<Arc<Clip>> {
    // 1. Mutate state
    let deleted_clip = {
        let mut app = get_app_write();
        app.delete_clip_from_track(track_id, clip_id, true)?
    };

    // 2. Update history
    {
        let mut history = get_history_lock();
        history.push(ProjectAction::DeleteClip {
            track_id,
            clip: (*deleted_clip).clone(),
        });
    }

    broadcast_state_change();

    Ok(deleted_clip)
}

pub fn move_clip(
    source_track_id: TrackId,
    target_track_id: TrackId,
    clip_id: ClipId,
    new_start_time: u32,
) -> anyhow::Result<Clip> {
    // 1. Capture old state if we want to support undo for single move
    let old_clip = {
        let app = crate::lock::get_app_read();
        app.get_clip(&source_track_id, &clip_id)
            .ok_or_else(|| anyhow::anyhow!("Clip not found"))?
    };

    // 2. Mutate state
    let modified_clip = {
        let mut app = get_app_write();
        app.move_clip(source_track_id, target_track_id, clip_id, new_start_time)
            .map_err(|e| anyhow::anyhow!("{}", e))?
    };

    // 3. Update history
    {
        let mut history = get_history_lock();
        history.push(ProjectAction::MoveClip {
            old_track_id: source_track_id,
            new_track_id: target_track_id,
            clip_id,
            old_start_time: old_clip.start_time,
            new_start_time: modified_clip.start_time,
        });
    }

    broadcast_state_change();

    Ok(modified_clip)
}

pub fn resize_clip(
    track_id: TrackId,
    clip_id: ClipId,
    edge: ResizeEdge,
    new_time_val: u32,
) -> anyhow::Result<Clip> {
    // 1. Capture old state
    let old_clip = {
        let app = crate::lock::get_app_read();
        app.get_clip(&track_id, &clip_id)
            .ok_or_else(|| anyhow::anyhow!("Clip not found"))?
    };

    // 2. Mutate state
    let modified_clip = {
        let mut app = get_app_write();
        app.resize_clip(track_id, clip_id, edge, new_time_val)
            .map_err(|e| anyhow::anyhow!("{}", e))?
    };

    // 3. Update history
    {
        let mut history = get_history_lock();
        history.push(ProjectAction::ResizeClip {
            track_id,
            old_clip,
            new_clip: modified_clip.clone(),
        });
    }
    broadcast_state_change();
    Ok(modified_clip)
}

pub fn cut_clip(
    track_id: TrackId,
    clip_id: ClipId,
    cut_point_sample: u32,
) -> anyhow::Result<(Clip, Clip)> {
    // 1. Capture old state
    let old_clip = {
        let app = crate::lock::get_app_read();
        app.get_clip(&track_id, &clip_id)
            .ok_or_else(|| anyhow::anyhow!("Clip not found"))?
    };

    // 2. Mutate state
    let (c1, c2) = {
        let mut app = get_app_write();
        app.cut_clip(&track_id, &clip_id, cut_point_sample)
            .map_err(|e| anyhow::anyhow!("{}", e))?
    };

    // 3. Update history (as a batch: delete old, add two new)
    {
        let mut history = get_history_lock();
        history.push(ProjectAction::Batch(vec![
            ProjectAction::DeleteClip {
                track_id,
                clip: old_clip,
            },
            ProjectAction::AddClip {
                track_id,
                clip: c1.clone(),
            },
            ProjectAction::AddClip {
                track_id,
                clip: c2.clone(),
            },
        ]));
    }
    broadcast_state_change();
    Ok((c1, c2))
}

pub fn batch_delete_clips(track_id: TrackId, clip_ids: Vec<ClipId>) -> anyhow::Result<()> {
    let mut deleted_actions = Vec::new();

    // 1. Mutate state and collect actions
    {
        let mut app = get_app_write();
        for clip_id in clip_ids {
            if let Ok(deleted_clip_arc) = app.delete_clip_from_track(track_id, clip_id, true) {
                deleted_actions.push(ProjectAction::DeleteClip {
                    track_id,
                    clip: (*deleted_clip_arc).clone(),
                });
            }
        }
    }

    // 2. Update history
    if !deleted_actions.is_empty() {
        let mut history = get_history_lock();
        if deleted_actions.len() == 1 {
            history.push(deleted_actions.remove(0));
        } else {
            history.push(ProjectAction::Batch(deleted_actions));
        }
    }
    broadcast_state_change();
    Ok(())
}

pub fn batch_move_clips(
    source_track_id: TrackId,
    target_track_id: TrackId,
    clip_ids: Vec<ClipId>,
    delta_samples: i64,
) -> anyhow::Result<Vec<Clip>> {
    // 1. Capture old states
    let old_clips = {
        let app = crate::lock::get_app_read();
        clip_ids
            .iter()
            .filter_map(|&id| app.get_clip(&source_track_id, &id))
            .collect::<Vec<_>>()
    };

    // 2. Mutate state
    let modified_clips = {
        let mut app = get_app_write();
        app.move_clip_batch(source_track_id, target_track_id, clip_ids, delta_samples)
            .map_err(|e| anyhow::anyhow!("{}", e))?
    };

    // 3. Update history
    {
        let mut history_actions = Vec::new();
        for (old, new) in old_clips.iter().zip(modified_clips.iter()) {
            history_actions.push(ProjectAction::MoveClip {
                old_track_id: source_track_id,
                new_track_id: target_track_id,
                clip_id: new.id,
                old_start_time: old.start_time,
                new_start_time: new.start_time,
            });
        }

        if !history_actions.is_empty() {
            let mut history = get_history_lock();
            if history_actions.len() == 1 {
                history.push(history_actions.remove(0));
            } else {
                history.push(ProjectAction::Batch(history_actions));
            }
        }
    }
    broadcast_state_change();
    Ok(modified_clips)
}

pub fn batch_resize_clips(
    track_id: TrackId,
    clip_ids: Vec<ClipId>,
    edge: ResizeEdge,
    delta_samples: i64,
) -> anyhow::Result<Vec<Clip>> {
    // 1. Capture old states
    let old_clips = {
        let app = crate::lock::get_app_read();
        clip_ids
            .iter()
            .filter_map(|&id| app.get_clip(&track_id, &id))
            .collect::<Vec<_>>()
    };

    // 2. Mutate state
    let modified_clips = {
        let mut app = get_app_write();
        app.resize_clip_batch(track_id, clip_ids, edge, delta_samples)
            .map_err(|e| anyhow::anyhow!("{}", e))?
    };

    // 3. Update history
    {
        let mut history_actions = Vec::new();
        for (old, new) in old_clips.iter().zip(modified_clips.iter()) {
            history_actions.push(ProjectAction::ResizeClip {
                track_id,
                old_clip: old.clone(),
                new_clip: new.clone(),
            });
        }

        if !history_actions.is_empty() {
            let mut history = get_history_lock();
            if history_actions.len() == 1 {
                history.push(history_actions.remove(0));
            } else {
                history.push(ProjectAction::Batch(history_actions));
            }
        }
    }
    broadcast_state_change();
    Ok(modified_clips)
}

pub fn copy_clips<T, F>(
    track_id: TrackId, 
    clip_ids: Vec<ClipId>, 
    mapper: F
) -> anyhow::Result<T>
where
    F: FnOnce(&ClipboardContent) -> T,
{
    let mut app = get_app_write();
    let mut clips_to_copy = Vec::with_capacity(clip_ids.len());

    let track = app.tracks.get(&track_id)
        .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;

    // Clone the requested clips
    for clip_id in &clip_ids {
        let clip_arc = track.get_clip(clip_id)
            .ok_or_else(|| anyhow::anyhow!("Clip {:?} not found", clip_id))?;
            
        // Dereference the Arc and clone the inner Clip struct
        clips_to_copy.push(clip_arc.as_ref().clone());
    }

    // Update the App's clipboard state
    if !clips_to_copy.is_empty() {
        app.clipboard = ClipboardContent::Clips(clips_to_copy);
    } else {
        // Standardize empty behavior
        app.clipboard = ClipboardContent::Empty;
    }

    // Return the mapped DTO
    Ok(mapper(&app.clipboard))
}

pub fn paste_clips(target_track_id: TrackId, paste_start_time: u32) -> anyhow::Result<()> {
    let mut actions = Vec::new();

    // 1. Mutate state
    {
        let mut app = get_app_write();

        let clips_to_paste = match &app.clipboard {
            ClipboardContent::Clips(clips) => clips.clone(),
            _ => return Ok(()),
        };

        if clips_to_paste.is_empty() {
            return Ok(());
        }

        let min_start = clips_to_paste
            .iter()
            .map(|c| c.start_time)
            .min()
            .unwrap_or(0);
        let offset = paste_start_time as i64 - min_start as i64;

        let new_clips: Vec<Clip> = clips_to_paste
            .iter()
            .map(|clip| {
                let new_clip_id = ClipId::next(&mut app.clip_counter);
                let new_start = (clip.start_time as i64 + offset).max(0) as u32;
                Clip {
                    id: new_clip_id,
                    name: clip.name.clone(),
                    start_time: new_start,
                    source: clip.source.clone(),
                    offset_start: clip.offset_start,
                    loop_length: clip.loop_length,
                }
            })
            .collect();

        let track_arc = app
            .tracks
            .get_mut(&target_track_id)
            .ok_or_else(|| anyhow::anyhow!("Track not found"))?;
        let track = Arc::make_mut(track_arc);

        for new_clip in new_clips {
            if let Ok(_) = track.add_clip(new_clip.clone()) {
                actions.push(ProjectAction::AddClip {
                    track_id: target_track_id,
                    clip: new_clip,
                });
            }
        }
    }

    // 2. Update history
    if !actions.is_empty() {
        let mut history = get_history_lock();
        if actions.len() == 1 {
            history.push(actions.remove(0));
        } else {
            history.push(ProjectAction::Batch(actions));
        }
    }
    broadcast_state_change();
    Ok(())
}
