use std::sync::Arc;

use karbeat_plugin_types::ParameterSpec;

use crate::{
    commands::{
        AudioCommand, AudioFeedback, EffectTarget, MixerChannelSnapshot, MixerChannelTarget,
    },
    context::{
        ctx,
        utils::{get_effect_plugin_box, send_audio_command},
    },
    core::project::{
        mixer::{
            BusMixerChannel, EffectInstance, MixerChannel, MixerChannelParams, MixerState,
            RoutingConnection, RoutingNode,
        },
        TrackId,
    },
    lock::{get_app_read, get_app_write},
    shared::id::*,
};

/// **GETTER: Fetch the mixer state from application state and map it to T value**
pub fn get_mixer_state<T, F>(mapper: F) -> T
where
    F: FnOnce(&MixerState) -> T,
{
    let app = get_app_read();
    mapper(&app.mixer)
}

/// **GETTER: Get Specific Mixer Channel and map it to T value
pub fn get_mixer_channel<T, F>(track_id: TrackId, mapper: F) -> anyhow::Result<T>
where
    F: Fn(&MixerChannel) -> T,
{
    let app = get_app_read();
    let mixer_state = &app.mixer;
    let channel = mixer_state.channels.get(&track_id);
    channel
        .ok_or_else(|| anyhow::anyhow!("Channel not found"))
        .map(|c| mapper(&c.channel))
}

/// Get track channel's parameter specs
pub fn get_track_mixer_channel_specs<C, U, M>(track_id: &TrackId, mapper: M) -> Option<C>
where
    M: Fn(&ParameterSpec) -> U,
    C: FromIterator<U>,
{
    let app = get_app_read();
    let mix_channel = app.mixer.channels.get(track_id)?;
    Some(
        mix_channel
            .channel
            .get_channel_specs()
            .iter()
            .map(mapper)
            .collect(),
    )
}

/// Get bus channel's parameter specs
pub fn get_bus_mixer_channel_specs<C, U, M>(bus_id: &BusId, mapper: M) -> Option<C>
where
    M: Fn(&ParameterSpec) -> U,
    C: FromIterator<U>,
{
    let app = get_app_read();
    let bus_channel = app.mixer.buses.get(bus_id)?;
    Some(
        bus_channel
            .channel
            .get_channel_specs()
            .iter()
            .map(mapper)
            .collect(),
    )
}

/// get master channel's parameter specs
pub fn get_master_channel_specs<C, U, M>(mapper: M) -> C
where
    M: Fn(&ParameterSpec) -> U,
    C: FromIterator<U>,
{
    let app = get_app_read();
    app.mixer
        .master_bus
        .get_channel_specs()
        .iter()
        .map(mapper)
        .collect()
}

pub fn get_mixer_channel_populated<C, MC, EI, MixChanF, EffInstF>(
    track_id: TrackId,
    mixer_mapper: MixChanF,
    instance_mapper: EffInstF,
) -> anyhow::Result<(MC, C)>
where
    MixChanF: FnOnce(&MixerChannel) -> MC,
    EffInstF: Fn(&EffectInstance) -> EI,
    C: FromIterator<EI>,
{
    let app = get_app_read();

    let channel = app
        .mixer
        .channels
        .get(&track_id)
        .ok_or_else(|| anyhow::anyhow!("Channel not found"))?;

    let mapped_channel = mixer_mapper(&channel.channel);

    let mapped_effects: C = channel
        .channel
        .effects
        .iter()
        .map(|e| instance_mapper(e))
        .collect();

    Ok((mapped_channel, mapped_effects))
}

pub fn get_master_bus() -> Arc<MixerChannel> {
    let app = get_app_read();
    app.mixer.master_bus.clone()
}

pub fn get_master_bus_populated<C, T, F>(mapper: F) -> C
where
    F: Fn(&EffectInstance) -> T,
    C: FromIterator<T>,
{
    let app = get_app_read();
    app.mixer
        .master_bus
        .effects
        .iter()
        .map(|e| mapper(e))
        .collect()
}

/// **GETTER: Fetch all buses**
pub fn get_buses<C, T, F>(mut mapper: F) -> C
where
    F: FnMut(&BusId, &BusMixerChannel) -> T,
    C: FromIterator<T>,
{
    let app = get_app_read();
    app.mixer
        .buses
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
    app.mixer.routing.iter().map(|conn| mapper(conn)).collect()
}

/// **GETTER: Fetch the routing destinations for a specific source channel**
pub fn get_destinations_of_mixer_channel<C, T, F>(source: &RoutingNode, mut mapper: F) -> C
where
    F: FnMut(&RoutingConnection) -> T,
    C: FromIterator<T>,
{
    let app = get_app_read();
    app.mixer
        .routing
        .iter()
        .filter(|conn| conn.source == *source)
        .map(|conn| mapper(conn))
        .collect()
}

// ======================================
// Mixer Channel DSP Parameter Commands
// ======================================

/// Push a single DSP parameter change for a mixer channel into the audio thread
/// via the ring buffer. The audio thread is the sole owner of these values;
/// AppState is only updated during save_project.
pub fn set_mixer_channel_param(target: MixerChannelTarget, param: MixerChannelParams) {
    send_audio_command(AudioCommand::SetMixerChannelParameter { target, param });
}

/// Ask the audio thread to emit a full MixerChannelSnapshot for the given
/// channel. Poll the result with `poll_mixer_channel_feedback`.
pub fn query_mixer_channel(target: MixerChannelTarget) {
    send_audio_command(AudioCommand::QueryMixerChannel { target });
}

/// Drain all pending `MixerChannelSnapshot` messages from the shared feedback
/// buffer and map each one through the provided `mapper` closure.
///
/// This follows the exact same pattern as `poll_generator_parameter_feedback`
/// in `plugin_api`. The FFI layer spawns a polling thread that calls this
/// at ~60 fps and forwards results to Flutter via a `StreamSink`.
///
/// Unrelated feedback messages are kept in the pending buffer so other
/// pollers (plugin parameters, etc.) can still consume them.
pub fn poll_mixer_channel_feedback<T, F>(mut mapper: F) -> Vec<T>
where
    F: FnMut(MixerChannelSnapshot) -> T,
{
    let mut results = Vec::new();
    // All pollers share the same pending buffer that lives on DawContext
    let mut pending = ctx().pending_feedback.lock();

    // Drain the live ring buffer into the shared pending store first
    if let Some(consumer) = ctx().feedback_consumer.lock().as_mut() {
        while let Ok(feedback) = consumer.pop() {
            pending.push(feedback);
        }
    }

    // Extract only MixerChannelSnapshot entries; leave everything else intact
    pending.retain(|feedback| match feedback {
        AudioFeedback::MixerChannelSnapshot(snap) => {
            results.push(mapper(snap.clone()));
            false // consumed
        }
        _ => true,
    });

    results
}

// ======================================
// Effect Chain (structural, still AppState-backed)
// ======================================

pub fn add_effect_to_mixer_channel_by_id(
    track_id: TrackId,
    registry_id: u32,
) -> anyhow::Result<()> {
    let effect_id = {
        let mut app = get_app_write();
        app.mixer
            .add_effect_descriptor_by_id(&track_id, registry_id)?;
        // Retrieve the ID of the freshly-added effect
        app.mixer
            .channels
            .get(&track_id)
            .and_then(|ch| ch.channel.effects.last())
            .map(|e| e.id)
            .ok_or_else(|| anyhow::anyhow!("Effect not found after insertion"))?
    };

    if let Some(plugin) = get_effect_plugin_box(registry_id) {
        send_audio_command(AudioCommand::AddEffect {
            target: EffectTarget::Track(track_id),
            effect_id,
            effect: plugin,
        });
    } else {
        log::warn!(
            "[mixer_api] Could not instantiate effect plugin {:?} for audio thread",
            registry_id
        );
    }
    Ok(())
}

pub fn remove_effect_from_mixer_channel(
    track_id: TrackId,
    effect_instance_id: EffectId,
) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer
            .remove_effect_by_id(&track_id, effect_instance_id)?;
    }
    send_audio_command(AudioCommand::RemoveEffect {
        target: EffectTarget::Track(track_id),
        effect_id: effect_instance_id,
    });
    Ok(())
}

pub fn add_effect_to_master_bus(registry_id: u32) -> anyhow::Result<()> {
    let effect_id = {
        let mut app = get_app_write();
        app.mixer.add_effect_to_master_bus(registry_id)?;
        app.mixer
            .master_bus
            .effects
            .last()
            .map(|e| e.id)
            .ok_or_else(|| anyhow::anyhow!("Effect not found after insertion"))?
    };

    if let Some(plugin) = get_effect_plugin_box(registry_id) {
        send_audio_command(AudioCommand::AddEffect {
            target: EffectTarget::Master,
            effect_id,
            effect: plugin,
        });
    } else {
        log::warn!(
            "[mixer_api] Could not instantiate master effect plugin {:?}",
            registry_id
        );
    }
    Ok(())
}

pub fn remove_effect_from_master_bus(effect_instance_id: EffectId) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer
            .remove_effect_from_master_bus(effect_instance_id)?;
    }
    send_audio_command(AudioCommand::RemoveEffect {
        target: EffectTarget::Master,
        effect_id: effect_instance_id,
    });
    Ok(())
}

pub fn create_bus(name: String) -> BusId {
    let bus_id = {
        let mut app = get_app_write();
        app.mixer.create_bus(name.clone())
    };
    send_audio_command(AudioCommand::AddBus { bus_id, name });
    bus_id
}

pub fn delete_bus(bus_id: BusId) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer.remove_bus(bus_id)?;
    }
    send_audio_command(AudioCommand::RemoveBus { bus_id });
    Ok(())
}

pub fn add_effect_to_bus(bus_id: BusId, registry_id: u32) -> anyhow::Result<()> {
    let effect_id = {
        let mut app = get_app_write();
        app.mixer.add_effect_to_bus(bus_id, registry_id)?;
        app.mixer
            .buses
            .get(&bus_id)
            .and_then(|b| b.channel.effects.last())
            .map(|e| e.id)
            .ok_or_else(|| anyhow::anyhow!("Effect not found after insertion"))?
    };

    if let Some(plugin) = get_effect_plugin_box(registry_id) {
        send_audio_command(AudioCommand::AddEffect {
            target: EffectTarget::Bus(bus_id),
            effect_id,
            effect: plugin,
        });
    } else {
        log::warn!(
            "[mixer_api] Could not instantiate bus effect plugin {:?}",
            registry_id
        );
    }
    Ok(())
}

pub fn rename_bus(bus_id: BusId, new_name: &str) -> anyhow::Result<()> {
    {
        let mut app = get_app_write();
        app.mixer.rename_bus(bus_id, new_name)?;
    }
    // Bus name is not used in audio DSP — no engine notification needed.
    Ok(())
}

pub fn set_routing(conn: RoutingConnection) -> anyhow::Result<()> {
    let routing = {
        let mut app = get_app_write();
        app.mixer.add_routing(conn)?;
        app.mixer.routing.clone()
    };
    send_audio_command(AudioCommand::UpdateRouting { routing });
    Ok(())
}

pub fn remove_routing(
    source: RoutingNode,
    destination: RoutingNode,
    is_send: bool,
) -> anyhow::Result<()> {
    let routing = {
        let mut app = get_app_write();
        app.mixer.remove_routing(source, destination, is_send)?;
        app.mixer.routing.clone()
    };
    send_audio_command(AudioCommand::UpdateRouting { routing });
    Ok(())
}
