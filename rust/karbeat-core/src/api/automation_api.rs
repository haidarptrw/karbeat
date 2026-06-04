use std::sync::Arc;

use crate::{
    commands::AudioCommand,
    context::utils::{broadcast_automation_lane, send_audio_command},
    core::project::{
        automation::{AutomationLane, AutomationPoint, AutomationTarget},
        ModulationLink, ModulationSource,
    },
    lock::{get_app_read, get_app_write},
    shared::{AutomationId, BusId, ModulationId, ModulationLinkId, TrackId},
};

/// Get all automations lane for all types
pub fn get_automations_lanes_all<C, U, M>(mapper: M) -> C
where
    M: Fn(&AutomationLane) -> U,
    C: FromIterator<U>,
{
    let app = get_app_read();

    app.automation_pool
        .values()
        .map(|a| mapper(a.as_ref()))
        .collect()
}

pub fn add_automation_lane_for_track(
    track_id: TrackId,
    target: AutomationTarget,
    label: impl Into<String>,
    min: f32,
    max: f32,
    default_value: f32,
) -> anyhow::Result<Arc<AutomationLane>> {
    let (lane, _link_id) = {
        let mut app = get_app_write();
        app.add_automation_lane_for_track(track_id, target, label, min, max, default_value)?
    };

    // Broadcast the new lane to the audio thread by its AutomationId
    broadcast_automation_lane(lane.id);

    // TODO: add history

    Ok(lane)
}

pub fn add_automation_lane(
    target: AutomationTarget,
    label: impl Into<String>,
    min: f32,
    max: f32,
    default_value: f32,
) -> anyhow::Result<Arc<AutomationLane>> {
    let (lane, _link_id) = {
        let mut app = get_app_write();
        app.add_automation_lane(target, label, min, max, default_value)?
    };

    broadcast_automation_lane(lane.id);

    // TODO: add history

    Ok(lane)
}

pub fn add_automation_lane_for_bus(
    bus_id: BusId,
    target: AutomationTarget,
    label: impl Into<String>,
    min: f32,
    max: f32,
    default_value: f32,
) -> anyhow::Result<Arc<AutomationLane>> {
    let (lane, _link_id) = {
        let mut app = get_app_write();
        app.add_automation_lane_for_bus(bus_id, target, label, min, max, default_value)?
    };

    broadcast_automation_lane(lane.id);

    // TODO: add history

    Ok(lane)
}

// pub fn remove_automation_lane(automation_id: AutomationId) -> anyhow::Result<()> {
//     {
//         let mut app = get_app_write();
//         app.remove_automation_lane(automation_id)?;
//     }
//
//     send_audio_command(AudioCommand::RemoveAutomationLane { id: automation_id });
//     Ok(())
// }

pub fn add_new_automation_point(
    automation_id: AutomationId,
    time_ticks: u32,
    value: f32,
) -> anyhow::Result<AutomationPoint> {
    let auto_point = {
        let mut app = get_app_write();
        let point = AutomationPoint::new(time_ticks, value);
        app.add_automation_point(automation_id, time_ticks, value)?;
        point
    };

    broadcast_automation_lane(automation_id);

    // TODO: Add history
    Ok(auto_point)
}

pub fn remove_automation_point(automation_id: AutomationId, index: usize) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.remove_automation_point(automation_id, index)?;
    }
    broadcast_automation_lane(automation_id);
    Ok(())
}

pub fn update_automation_point(
    automation_id: AutomationId,
    index: usize,
    time_ticks: u32,
    value: f32,
    tension: f32,
) -> anyhow::Result<usize> {
    let new_index = {
        let mut app = get_app_write();

        let (_, new_index) =
            app.update_automation_point(automation_id, index, time_ticks, value, tension)?;
        new_index
    };

    broadcast_automation_lane(automation_id);
    Ok(new_index)
}

pub fn get_automation_lanes_for_track(
    track_id: TrackId,
) -> Vec<(ModulationLinkId, AutomationId, Arc<AutomationLane>)> {
    let app = get_app_read();
    app.get_automation_lanes_for_track(track_id)
}

pub fn get_automation_lanes_for_bus(
    bus_id: BusId,
) -> Vec<(ModulationLinkId, AutomationId, Arc<AutomationLane>)> {
    let app = get_app_read();
    app.get_automation_lanes_for_bus(bus_id)
}

// ▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱
// Modulation API
// ▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱

/// Get all modulations in the project
pub fn get_all_linked_modulation_params<Id, T, F>(f: F) -> std::collections::HashMap<Id, T>
where
    Id: std::hash::Hash + std::cmp::Eq,
    F: Fn(&ModulationLinkId, &crate::core::project::ModulationLinkForOrderedLaneView) -> (Id, T),
{
    let app = get_app_read();
    app.modulation_links
        .iter()
        .map(|(id, modulation)| f(id, modulation))
        .collect()
}

/// Add generic modulation source
pub fn add_modulation_source(source: ModulationSource) -> ModulationId {
    let id = {
        let mut app = get_app_write();
        app.add_modulation_source(source.clone())
    };

    // AddModulationSource is already a granular ring-buffer command
    send_audio_command(AudioCommand::AddModulationSource { id, source });
    id
}

/// Remove the modulation source. This function also cascade delete all link
/// with this source
pub fn remove_modulation_source(mod_id: ModulationId) {
    {
        let mut app = get_app_write();
        let _ = app.remove_modulation_source(mod_id);
    }
    send_audio_command(AudioCommand::RemoveModulationSource(mod_id));
}

pub fn remove_modulation_link(mod_link_id: ModulationLinkId) {
    {
        let mut app = get_app_write();
        let _ = app.remove_modulation_link(mod_link_id);
    }

    send_audio_command(AudioCommand::RemoveModulationLink(mod_link_id));
}

/// Link the target param to a modulation source
pub fn link_this_param_to_controller(
    source_id: ModulationId,
    target: AutomationTarget,
    depth: f32,
    base_value: f32,
) -> anyhow::Result<ModulationLinkId> {
    let (id, link) = {
        let mut app = get_app_write();
        let id = app.link_modulation(source_id, target, depth, base_value)?;
        let link = app
            .modulation_links
            .get(&id)
            .map(|l| ModulationLink {
                id,
                source_id: l.prop.source_id,
                target: l.prop.target.clone(),
                depth: l.prop.depth,
                base_value: l.prop.base_value,
            })
            .ok_or_else(|| anyhow::anyhow!("Link not found after insertion"))?;
        (id, link)
    };

    send_audio_command(AudioCommand::AddModulationLink { id, link });
    Ok(id)
}
