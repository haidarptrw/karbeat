use karbeat_plugin_api::wrapper::PluginParameter;
use karbeat_plugins::registry::PluginInfo;
use parking_lot::Mutex;

use crate::{
    commands::{AudioCommand, AudioFeedback, EffectTarget},
    context::ctx,
    core::project::{
        generator::GeneratorInstanceType,
        mixer::{EffectId, EffectInstance},
        GeneratorId, GeneratorInstance, TrackId,
    },
    lock::{get_app_read, get_app_write, get_plugin_registry_read},
};

use std::sync::Arc;

pub fn get_available_generators<C, U, M>(mapper: M) -> C
where
    M: Fn(&PluginInfo) -> U,
    C: FromIterator<U>,
{
    let registry = get_plugin_registry_read();
    registry
        .list_generators_with_ids()
        .iter()
        .map(mapper)
        .collect()
}

pub fn get_available_effects<C, U, M>(mapper: M) -> C
where
    M: Fn(&PluginInfo) -> U,
    C: FromIterator<U>,
{
    let registry = get_plugin_registry_read();
    registry
        .list_effects_with_ids()
        .iter()
        .map(mapper)
        .collect()
}

pub fn get_generator<M, U>(generator_id: &GeneratorId, mapper: M) -> Option<U>
where
    M: FnOnce(&GeneratorInstance) -> U,
{
    let app = get_app_read();

    let generator_arc = app.generator_pool.get(generator_id)?;

    let generator = generator_arc.as_ref();
    Some(mapper(generator))
}

pub fn get_effect<M, U>(track_id: &TrackId, effect_id: &EffectId, mapper: M) -> Option<U>
where
    M: FnOnce(&EffectInstance) -> U,
{
    let app = get_app_read();

    let channel = app.mixer.channels.get(track_id)?;

    let effect = channel.effects.iter().find(|e| e.id == *effect_id)?;

    Some(mapper(effect))
}

pub fn get_effect_from_master<M, U>(effect_id: &EffectId, mapper: M) -> Option<U>
where
    M: FnOnce(&EffectInstance) -> U,
{
    let app = get_app_read();

    let channel = app.mixer.master_bus.as_ref();

    let effect = channel.effects.iter().find(|e| e.id == *effect_id)?;

    Some(mapper(effect))
}

pub fn get_effects_from_track<C, U, M>(track_id: &TrackId, mapper: M) -> Option<C>
where
    M: Fn(&EffectInstance) -> U,
    C: FromIterator<U>,
{
    let app = get_app_read();

    let channel = app.mixer.channels.get(track_id)?;

    Some(channel.effects.iter().map(mapper).collect())
}

pub fn get_master_effects<C, U, M>(mapper: M) -> C
where
    M: Fn(&EffectInstance) -> U,
    C: FromIterator<U>,
{
    let app = get_app_read();

    let channel = app.mixer.master_bus.as_ref();

    channel.effects.iter().map(mapper).collect()
}

pub fn get_generator_parameter_specs<F, T>(
    generator_id: &GeneratorId,
    mapper: F,
) -> Result<Vec<T>, String>
where
    F: Fn(PluginParameter, f32) -> T,
{
    let app = get_app_read();
    let generator_arc = app
        .generator_pool
        .get(generator_id)
        .ok_or_else(|| format!("Generator {} not found", generator_id.0))?;

    let generator = generator_arc.as_ref();

    if let GeneratorInstanceType::Plugin(ref plugin_instance) = generator.instance_type {
        let registry = get_plugin_registry_read();

        let specs = if plugin_instance.registry_id > 0 {
            registry.get_generator_parameter_specs_by_id(plugin_instance.registry_id)
        } else {
            registry
                .get_generator_id_by_name(&plugin_instance.name)
                .and_then(|id| registry.get_generator_parameter_specs_by_id(id))
        };

        if let Some(specs) = specs {
            let result: Vec<T> = specs
                .into_iter()
                .map(|p| {
                    let value = plugin_instance
                        .parameters
                        .get(&p.id)
                        .copied()
                        .unwrap_or(p.default_value);
                    mapper(p, value)
                })
                .collect();
            Ok(result)
        } else {
            Err(format!(
                "Generator '{}' (registry_id={}) not found in registry",
                plugin_instance.name, plugin_instance.registry_id
            ))
        }
    } else {
        Err("Generator is not a plugin type".to_string())
    }
}

pub fn get_effect_parameter_specs<F, T>(
    target: &EffectTarget,
    effect_id: &EffectId,
    mapper: F,
) -> Result<Vec<T>, String>
where
    F: Fn(PluginParameter, f32) -> T,
{
    let app = get_app_read();

    let (plugin_name, plugin_registry_id, plugin_parameters) = match target {
        EffectTarget::Track(track_id) => {
            let channel = app
                .mixer
                .channels
                .get(track_id)
                .ok_or_else(|| format!("Track channel {} not found", track_id.0))?;
            let effect = channel
                .effects
                .iter()
                .find(|e| e.id == *effect_id)
                .ok_or_else(|| format!("Effect {} not found", effect_id.0))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
        EffectTarget::Bus(bus_id) => {
            let bus = app
                .mixer
                .buses
                .get(bus_id)
                .ok_or_else(|| format!("Bus {} not found", bus_id.0))?;
            let effect = bus
                .channel
                .effects
                .iter()
                .find(|e| e.id == *effect_id)
                .ok_or_else(|| format!("Effect {} not found", effect_id.0))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
        EffectTarget::Master => {
            let effect = app
                .mixer
                .master_bus
                .effects
                .iter()
                .find(|e| e.id == *effect_id)
                .ok_or_else(|| format!("Effect {} not found", effect_id.0))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
    };

    let registry = get_plugin_registry_read();

    let specs = if plugin_registry_id > 0 {
        registry.get_effect_parameter_specs_by_id(plugin_registry_id)
    } else {
        registry
            .get_effect_id_by_name(&plugin_name)
            .and_then(|id| registry.get_effect_parameter_specs_by_id(id))
    };

    if let Some(specs) = specs {
        let result: Vec<T> = specs
            .into_iter()
            .map(|p| {
                let value = plugin_parameters
                    .get(&p.id)
                    .copied()
                    .unwrap_or(p.default_value);
                mapper(p, value)
            })
            .collect();
        Ok(result)
    } else {
        Err(format!(
            "Effect '{}' (registry_id={}) not found in registry",
            plugin_name, plugin_registry_id
        ))
    }
}

pub fn set_generator_parameter(
    generator_id: &GeneratorId,
    param_id: u32,
    value: f32,
) -> Result<(), String> {
    if let Some(sender) = ctx().command_sender.lock().as_mut() {
        let _ = sender.push(AudioCommand::SetGeneratorParameter {
            generator_id: *generator_id,
            param_id,
            value,
        });
    }

    {
        let mut app = get_app_write();
        if let Some(generator_arc) = app.generator_pool.get_mut(generator_id) {
            let generator = Arc::make_mut(generator_arc);
            if let GeneratorInstanceType::Plugin(plugin_instance) = &mut generator.instance_type {
                plugin_instance.parameters.insert(param_id, value);
            }
        }
    }

    crate::context::utils::broadcast_state_change();
    Ok(())
}

pub fn get_generator_parameter(generator_id: &GeneratorId, param_id: u32) -> Result<f32, String> {
    let app = get_app_read();

    let generator_arc = app
        .generator_pool
        .get(generator_id)
        .ok_or_else(|| format!("Generator {} not found", generator_id.0))?;

    let generator = generator_arc.as_ref();

    if let GeneratorInstanceType::Plugin(ref plugin_instance) = generator.instance_type {
        plugin_instance
            .parameters
            .get(&param_id)
            .copied()
            .ok_or_else(|| format!("Parameter {} not found", param_id))
    } else {
        Err("Generator is not a plugin type".to_string())
    }
}

pub fn set_effect_parameter(
    target: &EffectTarget,
    effect_id: &EffectId,
    param_id: u32,
    value: f32,
) -> Result<(), String> {
    if let Some(sender) = ctx().command_sender.lock().as_mut() {
        match target {
            EffectTarget::Track(track_id) => {
                let _ = sender.push(AudioCommand::SetTrackEffectParameter {
                    track_id: *track_id,
                    effect_id: *effect_id,
                    param_id,
                    value,
                });
            }
            EffectTarget::Bus(bus_id) => {
                let _ = sender.push(AudioCommand::SetBusEffectParameter {
                    bus_id: *bus_id,
                    effect_id: *effect_id,
                    param_id,
                    value,
                });
            }
            EffectTarget::Master => {
                let _ = sender.push(AudioCommand::SetMasterEffectParameter {
                    effect_id: *effect_id,
                    param_id,
                    value,
                });
            }
        }
    }

    {
        let mut app = get_app_write();

        match target {
            EffectTarget::Track(track_id) => {
                if let Some(channel_arc) = app.mixer.channels.get_mut(track_id) {
                    let channel = Arc::make_mut(channel_arc);
                    if let Some(effect) = channel.effects.iter_mut().find(|e| e.id == *effect_id) {
                        let plugin = Arc::make_mut(&mut effect.instance);
                        plugin.parameters.insert(param_id, value);
                    }
                }
            }
            EffectTarget::Bus(bus_id) => {
                if let Some(bus_arc) = app.mixer.buses.get_mut(bus_id) {
                    let bus_mut = Arc::make_mut(bus_arc);
                    if let Some(effect) = bus_mut
                        .channel
                        .effects
                        .iter_mut()
                        .find(|e| e.id == *effect_id)
                    {
                        let plugin = Arc::make_mut(&mut effect.instance);
                        plugin.parameters.insert(param_id, value);
                    }
                }
            }
            EffectTarget::Master => {
                let master = Arc::make_mut(&mut app.mixer.master_bus);
                if let Some(effect) = master.effects.iter_mut().find(|e| e.id == *effect_id) {
                    let plugin = Arc::make_mut(&mut effect.instance);
                    plugin.parameters.insert(param_id, value);
                }
            }
        }
    }

    crate::context::utils::broadcast_state_change();
    Ok(())
}

pub fn query_generator_parameters(generator_id: &GeneratorId) -> Result<(), String> {
    if let Some(sender) = ctx().command_sender.lock().as_mut() {
        sender
            .push(AudioCommand::QueryGeneratorParameters {
                generator_id: *generator_id,
            })
            .map_err(|_| "Command queue full".to_string())?;
        Ok(())
    } else {
        Err("Audio stream not initialized".to_string())
    }
}

pub fn query_effect_parameters(target: &EffectTarget, effect_id: &EffectId) -> Result<(), String> {
    if let Some(sender) = ctx().command_sender.lock().as_mut() {
        match target {
            EffectTarget::Track(track_id) => {
                let _ = sender.push(AudioCommand::QueryTrackEffectParameters {
                    track_id: *track_id,
                    effect_id: *effect_id,
                });
            }
            EffectTarget::Bus(bus_id) => {
                let _ = sender.push(AudioCommand::QueryBusEffectParameters {
                    bus_id: *bus_id,
                    effect_id: *effect_id,
                });
            }
            EffectTarget::Master => {
                let _ = sender.push(AudioCommand::QueryMasterEffectParameters {
                    effect_id: *effect_id,
                });
            }
        }
        Ok(())
    } else {
        Err("Audio stream not initialized".to_string())
    }
}

pub fn sync_generator_parameters_from_audio<I, P>(items: I)
where
    I: IntoIterator<Item = (GeneratorId, P)>,
    P: IntoIterator<Item = (u32, f32)>,
{
    let mut app = get_app_write();

    for (gen_id, params) in items {
        if let Some(generator_arc) = app.generator_pool.get_mut(&gen_id) {
            let generator = Arc::make_mut(generator_arc);
            if let GeneratorInstanceType::Plugin(ref mut plugin_instance) = generator.instance_type
            {
                for (param_id, value) in params {
                    plugin_instance.parameters.insert(param_id, value);
                }
            }
        }
    }
}

pub fn sync_effect_parameters_from_audio<I, P>(items: I)
where
    I: IntoIterator<Item = (EffectTarget, EffectId, P)>,
    P: IntoIterator<Item = (u32, f32)>,
{
    let mut app = get_app_write();

    for (target, effect_id, params) in items {
        match target {
            EffectTarget::Master => {
                let master = Arc::make_mut(&mut app.mixer.master_bus);
                if let Some(effect) = master.effects.iter_mut().find(|e| e.id == effect_id) {
                    let plugin = Arc::make_mut(&mut effect.instance);
                    for (param_id, value) in params {
                        plugin.parameters.insert(param_id, value);
                    }
                }
            }
            EffectTarget::Track(track_id) => {
                if let Some(channel_arc) = app.mixer.channels.get_mut(&track_id) {
                    let channel = Arc::make_mut(channel_arc);
                    if let Some(effect) = channel.effects.iter_mut().find(|e| e.id == effect_id) {
                        let plugin = Arc::make_mut(&mut effect.instance);
                        for (param_id, value) in params {
                            plugin.parameters.insert(param_id, value);
                        }
                    }
                }
            }
            EffectTarget::Bus(bus_id) => {
                if let Some(bus) = app.mixer.buses.get_mut(&bus_id) {
                    let bus_mut = Arc::make_mut(bus);
                    if let Some(effect) = bus_mut
                        .channel
                        .effects
                        .iter_mut()
                        .find(|e| e.id == effect_id)
                    {
                        let plugin = Arc::make_mut(&mut effect.instance);
                        for (param_id, value) in params {
                            plugin.parameters.insert(param_id, value);
                        }
                    }
                }
            }
        }
    }
}

pub fn execute_plugin_command_generator(
    gen_registry_id: u32,
    command: &str,
    payload_value: &serde_json::Value,
) -> Option<serde_json::Value> {
    let registry = get_plugin_registry_read();
    let (mut plugin, _) = registry.create_generator_by_id(gen_registry_id)?;
    plugin.execute_custom_command(command, payload_value)
}

pub fn execute_plugin_command_effect(
    effect_registry_id: u32,
    command: &str,
    payload_value: &serde_json::Value,
) -> Option<serde_json::Value> {
    let registry = get_plugin_registry_read();
    let (mut plugin, _) = registry.create_effect_by_id(effect_registry_id)?;
    plugin.execute_custom_command(command, payload_value)
}

pub fn execute_effect_instance_command(
    target: &EffectTarget,
    effect_id: &EffectId,
    command: &str,
    payload_value: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let app = get_app_read();

    let (plugin_name, plugin_registry_id, plugin_parameters) = match target {
        EffectTarget::Track(track_id) => {
            let channel = app
                .mixer
                .channels
                .get(track_id)
                .ok_or_else(|| format!("Track channel {} not found", track_id.0))?;
            let effect = channel
                .effects
                .iter()
                .find(|e| e.id == *effect_id)
                .ok_or_else(|| format!("Effect {} not found", effect_id.0))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
        EffectTarget::Bus(bus_id) => {
            let bus = app
                .mixer
                .buses
                .get(bus_id)
                .ok_or_else(|| format!("Bus {} not found", bus_id.0))?;
            let effect = bus
                .channel
                .effects
                .iter()
                .find(|e| e.id == *effect_id)
                .ok_or_else(|| format!("Effect {} not found", effect_id.0))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
        EffectTarget::Master => {
            let effect = app
                .mixer
                .master_bus
                .effects
                .iter()
                .find(|e| e.id == *effect_id)
                .ok_or_else(|| format!("Effect {} not found", effect_id.0))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
    };

    let registry = get_plugin_registry_read();
    let mut temp_plugin = if plugin_registry_id > 0 {
        registry
            .create_effect_by_id(plugin_registry_id)
            .map(|(p, _)| p)
    } else {
        registry.create_effect(&plugin_name)
    }
    .ok_or_else(|| format!("Effect '{}' not found in registry", plugin_name))?;

    for (&param_id, &value) in &plugin_parameters {
        temp_plugin.set_parameter(param_id, value);
    }

    temp_plugin
        .execute_custom_command(command, payload_value)
        .ok_or_else(|| format!("Command '{}' not supported by '{}'", command, plugin_name))
}

pub fn execute_generator_instance_command(
    generator_id: &GeneratorId,
    command: &str,
    payload_value: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let app = get_app_read();

    let gen_arc = app
        .generator_pool
        .get(generator_id)
        .ok_or_else(|| format!("Generator {} not found", generator_id.0))?;

    let (plugin_name, plugin_registry_id, plugin_parameters) = match &gen_arc.instance_type {
        GeneratorInstanceType::Plugin(p) => (p.name.clone(), p.registry_id, p.parameters.clone()),
        _ => return Err("Generator is not a plugin".into()),
    };

    let registry = get_plugin_registry_read();
    let mut temp_plugin = if plugin_registry_id > 0 {
        registry
            .create_generator_by_id(plugin_registry_id)
            .map(|(p, _)| p)
    } else {
        registry.create_generator(&plugin_name)
    }
    .ok_or_else(|| format!("Generator '{}' not found in registry", plugin_name))?;

    for (&param_id, &value) in &plugin_parameters {
        temp_plugin.set_parameter(param_id, value);
    }

    temp_plugin
        .execute_custom_command(command, payload_value)
        .ok_or_else(|| format!("Command '{}' not supported by '{}'", command, plugin_name))
}

static PENDING_FEEDBACK: Mutex<Vec<AudioFeedback>> = Mutex::new(Vec::new());

pub fn poll_generator_parameter_feedback<T, F>(mut mapper: F) -> Vec<T>
where
    F: FnMut(GeneratorId, Vec<(u32, f32)>) -> T,
{
    let mut snapshots = Vec::new();
    let mut pending = PENDING_FEEDBACK.lock();

    // Drain context queues
    if let Some(consumer) = ctx().feedback_consumer.lock().as_mut() {
        while let Ok(feedback) = consumer.pop() {
            pending.push(feedback);
        }
    }

    pending.retain(|feedback| match feedback {
        AudioFeedback::GeneratorParameterSnapshot(snapshot) => {
            snapshots.push(mapper(snapshot.generator_id, snapshot.parameters.clone()));
            false
        }
        AudioFeedback::GeneratorParameterChanged(update) => {
            snapshots.push(mapper(
                update.generator_id,
                vec![(update.param_id, update.value)],
            ));
            false
        }
        _ => true,
    });

    snapshots
}

pub fn poll_effect_parameter_feedback<T, F>(mut mapper: F) -> Vec<T>
where
    F: FnMut(EffectTarget, EffectId, Vec<(u32, f32)>) -> T,
{
    let mut snapshots = Vec::new();
    let mut pending = PENDING_FEEDBACK.lock();

    // Drain context queues
    if let Some(consumer) = ctx().feedback_consumer.lock().as_mut() {
        while let Ok(feedback) = consumer.pop() {
            pending.push(feedback);
        }
    }

    pending.retain(|feedback| match feedback {
        AudioFeedback::EffectParameterSnapshot(snapshot) => {
            snapshots.push(mapper(
                snapshot.target.clone(),
                snapshot.effect_id,
                snapshot.parameters.clone(),
            ));
            false
        }
        AudioFeedback::EffectParameterChanged(update) => {
            snapshots.push(mapper(
                update.target.clone(),
                update.effect_id,
                vec![(update.param_id, update.value)],
            ));
            false
        }
        _ => true,
    });

    snapshots
}