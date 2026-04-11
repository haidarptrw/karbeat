use std::sync::Arc;

use crate::{
    context::utils::broadcast_state_change, core::project::{ TrackId, mixer::{ BusId, EffectId, EffectInstance, MixerBus, MixerChannel, MixerChannelParams, MixerState, RoutingConnection, RoutingNode } }, lock::{get_app_read, get_app_write}
};

/// **GETTER: Fetch the mixer state from application state and map it to T value**
pub fn get_mixer_state<T, F>(mapper: F) -> T where F: FnOnce(&MixerState) -> T {
    let app = get_app_read();
    mapper(&app.mixer)
}

/// **GETTER: Get Specific Mixer Channel and map it to T value
pub fn get_mixer_channel<T, F>(track_id: TrackId, mapper: F) -> anyhow::Result<T>
    where F: Fn(&MixerChannel) -> T
{
    let app = get_app_read();
    let mixer_state = &app.mixer;
    let channel = mixer_state.channels.get(&track_id);
    channel.ok_or_else(|| anyhow::anyhow!("Channel not found")).map(|c| mapper(c.as_ref()))
}

pub fn get_mixer_channel_populated<C, MC, EI, MixChanF, EffInstF>(
    track_id: TrackId,
    mixer_mapper: MixChanF,
    instance_mapper: EffInstF
)
    -> anyhow::Result<(MC, C)>
    where
        MixChanF: FnOnce(&MixerChannel) -> MC,
        EffInstF: Fn(&EffectInstance) -> EI,
        C: FromIterator<EI>
{
    let app = get_app_read();

    let channel = app.mixer.channels
        .get(&track_id)
        .ok_or_else(|| anyhow::anyhow!("Channel not found"))?;

    let mapped_channel = mixer_mapper(channel.as_ref());

    let mapped_effects: C = channel.effects
        .iter()
        .map(|e| instance_mapper(e))
        .collect();

    Ok((mapped_channel, mapped_effects))
}

pub fn get_master_bus() -> Arc<MixerChannel> {
    let app = get_app_read();
    app.mixer.master_bus.clone()
}

pub fn get_master_bus_populated<C,T,F>(mapper: F) -> C where F: Fn(&EffectInstance) -> T, C: FromIterator<T> {
    let app = get_app_read();
    app.mixer.master_bus.effects.iter()
        .map(|e| mapper(e))
        .collect()
}

/// **GETTER: Fetch all buses**
pub fn get_buses<C, T, F>(mut mapper: F) -> C
where
    F: FnMut(&BusId, &MixerBus) -> T,
    C: FromIterator<T>,
{
    let app = get_app_read();
    app.mixer.buses
        .iter()
        .map(|(id, bus)| mapper(id, bus.as_ref()))
        .collect()
}

/// **GETTER: Fetch the routing matrix**
pub fn get_routing_matrix<C, T, F>(mut mapper: F) -> C
where
    F: FnMut(&RoutingConnection) -> T,
    C: FromIterator<T>,
{
    let app = get_app_read();
    app.mixer.routing
        .iter()
        .map(|conn| mapper(conn))
        .collect()
}

pub fn set_master_bus_params(params: &[MixerChannelParams]) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer
            .set_params_master_bus(params)
            .map_err(|e| anyhow::anyhow!(e.message))?;
    }
    broadcast_state_change();
    Ok(())
}

pub fn set_mixer_channel_params(track_id: TrackId, params: &[MixerChannelParams]) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer
            .set_params_mixer_channel(&track_id, params)
            .map_err(|e| anyhow::anyhow!(e.message))?;
    }
    broadcast_state_change();
    Ok(())
}

pub fn add_effect_to_mixer_channel_by_id(track_id: TrackId, registry_id: u32) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer.add_effect_descriptor_by_id(&track_id, registry_id)?;
    }
    broadcast_state_change();
    Ok(())
}

pub fn remove_effect_from_mixer_channel(track_id: TrackId, effect_instance_id: EffectId) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer.remove_effect_by_id(&track_id, effect_instance_id)?;
    }
    broadcast_state_change();
    Ok(())
}

pub fn add_effect_to_master_bus(registry_id: u32) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer.add_effect_to_master_bus(registry_id)?;
    }
    broadcast_state_change();
    Ok(())
}

pub fn remove_effect_from_master_bus(effect_instance_id: EffectId) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer.remove_effect_from_master_bus(effect_instance_id)?;
    }
    broadcast_state_change();
    Ok(())
}

pub fn create_bus(name: String) -> BusId {
    let bus_id = {
        let mut app = get_app_write();
        app.mixer.create_bus(name)
    };
    broadcast_state_change();
    bus_id
}

pub fn delete_bus(bus_id: BusId) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer.remove_bus(bus_id)?;
    }
    broadcast_state_change();
    Ok(())
}

pub fn set_bus_params(bus_id: BusId, params: &[MixerChannelParams]) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer.set_params_bus(&bus_id, params)?;
    }
    broadcast_state_change();
    Ok(())
}

pub fn add_effect_to_bus(bus_id: BusId, registry_id: u32) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer.add_effect_to_bus(bus_id, registry_id)?;
    }
    broadcast_state_change();
    Ok(())
}

pub fn rename_bus(bus_id: BusId, new_name: &str) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer.rename_bus(bus_id, new_name)?;
    }
    broadcast_state_change();
    Ok(())
}

pub fn set_routing(conn: RoutingConnection) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer.add_routing(conn)?;
    }
    broadcast_state_change();
    Ok(())
}

pub fn remove_routing(source: RoutingNode, destination: RoutingNode, is_send: bool) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer.remove_routing(source, destination, is_send)?;
    }
    broadcast_state_change();
    Ok(())
}