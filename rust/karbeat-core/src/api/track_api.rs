use crate::context::utils::broadcast_track_graph;
use crate::core::project::track::RemovedTrackType;
use crate::core::project::AudioTrack;
use crate::lock::{get_app_read, get_app_write};
use crate::shared::id::*;
use karbeat_utils::color::Color;
use std::sync::Arc;

pub fn get_track<T, F>(track_id: TrackId, mapper: F) -> anyhow::Result<T>
where
    F: Fn(&AudioTrack) -> T,
{
    let app = get_app_read();
    let track = app
        .tracks
        .get(&track_id)
        .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;
    Ok(mapper(track.as_ref()))
}

pub fn add_midi_track_with_generator_id(registry_id: u32) -> anyhow::Result<Arc<AudioTrack>> {
    let res = {
        let mut app = get_app_write();
        app.add_new_midi_track_with_generator_id(registry_id)
    };
    broadcast_track_graph();
    res
}

pub fn change_track_name(track_id: TrackId, new_name: &str) -> anyhow::Result<()> {
    if new_name.len() > 20 {
        return Err(anyhow::anyhow!("Track name cannot exceed 20 characters"));
    }
    {
        let mut app = get_app_write();
        let track_arc = app
            .tracks
            .get_mut(&track_id)
            .ok_or_else(|| anyhow::anyhow!("Track not found"))?;
        let track = Arc::make_mut(track_arc);
        track.name = new_name.to_string();
    }
    // Track name does not affect audio DSP — no engine notification needed.
    Ok(())
}

pub fn change_track_color(track_id: TrackId, new_color: &str) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        let track_arc = app
            .tracks
            .get_mut(&track_id)
            .ok_or_else(|| anyhow::anyhow!("Track not found"))?;
        let track = Arc::make_mut(track_arc);
        track.color = Color::new_from_string(new_color).ok_or_else(|| {
            anyhow::anyhow!("Invalid color format. Use hex string like #RRGGBB or #RRGGBBAA")
        })?;
    }
    // Track color does not affect audio DSP — no engine notification needed.
    Ok(())
}

pub fn add_new_audio_track() -> Arc<AudioTrack> {
    let arc_track = {
        let mut app = get_app_write();
        app.add_new_audio_track()
    };
    broadcast_track_graph();
    arc_track
}

pub fn get_tracks<C, U, M>(mapper: M) -> anyhow::Result<C>
where
    M: Fn(u32, &AudioTrack) -> U,
    C: FromIterator<U>,
{
    let app = get_app_read();
    Ok(app
        .tracks
        .iter()
        .map(|(id, track)| mapper(id.to_u32(), track.as_ref()))
        .collect())
}

/// Get tracks ordered by index (For UI)
pub fn get_tracks_ordered<C, U, M>(mapper: M) -> anyhow::Result<C>
where
    M: Fn(u32, &AudioTrack) -> U,
    C: FromIterator<U>,
{
    let app = get_app_read();
    Ok(app
        .get_track_ordered_by_index()
        .iter()
        .map(|t| mapper(t.id.into(), t.as_ref()))
        .collect())
}

pub fn delete_track(track_id: TrackId) -> anyhow::Result<RemovedTrackType> {
    let deleted_track_type = {
        let mut app = get_app_write();
        app.remove_track(track_id)?
    };
    broadcast_track_graph();
    Ok(deleted_track_type)
}

/// Update track order
pub fn update_track_order(track_id: TrackId, new_idx: usize) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.update_track_order(track_id, new_idx)?;
    }

    // broadcast_state_change();
    Ok(())
}
