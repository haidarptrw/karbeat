use std::sync::Arc;

use crate::{
    context::utils::broadcast_state_change,
    core::project::automation::{ AutomationLane, AutomationPoint, AutomationTarget },
    lock::{ get_app_read, get_app_write },
    shared::{ AutomationId, BusId, TrackId },
};

/// Get all automations lane for all types
pub fn get_automations_lanes_all<C, U, M>(mapper: M) -> C
    where M: Fn(&AutomationLane) -> U, C: FromIterator<U>
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
    default_value: f32
) -> anyhow::Result<Arc<AutomationLane>> {
    let lane = {
        let mut app = get_app_write();
        app.add_automation_lane_for_track(&track_id, target, label, min, max, default_value)?
    };

    broadcast_state_change();

    // TODO: add history

    Ok(lane)
}

pub fn add_automation_lane_for_bus(
    bus_id: BusId,
    target: AutomationTarget
) -> anyhow::Result<Arc<AutomationLane>> {
    let lane = {
        let mut app = get_app_write();
        app.add_new_automation_track_from_bus(bus_id, target)?
    };

    broadcast_state_change();

    // TODO: add history

    Ok(lane)
}

pub fn remove_automation_lane(automation_id: AutomationId) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.remove_automation_lane(automation_id)?;
    }

    broadcast_state_change();
    Ok(())
}

pub fn add_new_automation_point(
    automation_id: AutomationId,
    time_ticks: u32,
    value: f32,
) -> anyhow::Result<AutomationPoint> {
    let auto_point  = {
        let mut app = get_app_write();
        let point = AutomationPoint::new(time_ticks, value);
        app.add_automation_point(automation_id, point.clone())?;
        point
    };
    
    broadcast_state_change();

    // TODO: Add history
    Ok(auto_point)
}

pub fn remove_automation_point (
    automation_id: AutomationId,
    index: usize
) -> anyhow::Result<()>{
    {
        let mut app = get_app_write();
        app.remove_automation_point(automation_id, index)?;
    }
    broadcast_state_change();
    Ok(())
}

pub fn update_automation_point (
    automation_id: AutomationId,
    index: usize,
    time_ticks: u32,
    value: f32,
 ) -> anyhow::Result<usize> {
    let new_index = {
        let mut app = get_app_write();

        let (_, new_index) = app.update_automation_point(automation_id, index, time_ticks, value)?;
        new_index
    };

    broadcast_state_change();
    Ok(new_index) // FIXME: update the app.update_autoation_point to return new index
}