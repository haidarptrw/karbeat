use std::sync::Arc;

use crate::{
    api::{mixer::UiEffectInstance, project::UiGeneratorInstance},
    broadcast_state_change,
    commands::{AudioCommand, AudioFeedback, EffectTarget},
    core::project::{
        generator::{GeneratorId, GeneratorInstanceType},
        mixer::{BusId, EffectId},
        TrackId,
    },
    ctx,
    plugin::wrapper::ParameterValueType,
    utils::lock::{get_app_read, get_app_write},
};

// ============================================================================
// UI TYPES FOR FLUTTER RUST BRIDGE
// ============================================================================

/// Parameter type enum for FRB
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UiParameterType {
    Float,
    Int,
    Bool,
    Choice,
}

impl From<ParameterValueType> for UiParameterType {
    fn from(value: ParameterValueType) -> Self {
        match value {
            ParameterValueType::Float => UiParameterType::Float,
            ParameterValueType::Int => UiParameterType::Int,
            ParameterValueType::Bool => UiParameterType::Bool,
            ParameterValueType::Choice => UiParameterType::Choice,
        }
    }
}

/// Plugin parameter description for UI generation
#[derive(Clone, Debug)]
pub struct UiPluginParameter {
    pub id: u32,
    pub name: String,
    pub group: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub default_value: f32,
    pub step: f32,
    pub param_type: UiParameterType,
    pub choices: Vec<String>,
}

// ============================================================================
// PLUGIN API FUNCTIONS
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KarbeatPluginType {
    Generator,
    Effect,
}
#[derive(Clone, Debug)]
pub struct UiPluginInfo {
    pub id: u32,
    pub name: String,
    pub plugin_type: KarbeatPluginType,
}

/// Get all available generators in Plugin Registry (names only, backwards compatible)
pub fn get_available_generators() -> Result<Vec<String>, String> {
    let registry = ctx().plugin_registry.read().unwrap();
    Ok(registry.list_generators())
}

/// Get all available effects in Plugin Registry (names only, backwards compatible)
pub fn get_available_effects() -> Result<Vec<String>, String> {
    let registry = ctx().plugin_registry.read().unwrap();
    Ok(registry.list_effects())
}

/// Get all available generators with their registry IDs (preferred for UI)
pub fn get_available_generators_with_ids() -> Result<Vec<UiPluginInfo>, String> {
    let registry = ctx().plugin_registry.read().unwrap();
    Ok(registry
        .list_generators_with_ids()
        .into_iter()
        .map(|p| UiPluginInfo {
            id: p.id,
            name: p.name,
            plugin_type: KarbeatPluginType::Generator,
        })
        .collect())
}

/// Get all available effects with their registry IDs (preferred for UI)
pub fn get_available_effects_with_ids() -> Result<Vec<UiPluginInfo>, String> {
    let registry = ctx().plugin_registry.read().unwrap();
    Ok(registry
        .list_effects_with_ids()
        .into_iter()
        .map(|p| UiPluginInfo {
            id: p.id,
            name: p.name,
            plugin_type: KarbeatPluginType::Effect,
        })
        .collect())
}

/// Get a single generator state from the Generator Pool
pub fn get_generator(generator_id: u32) -> Result<UiGeneratorInstance, String> {
    let app = get_app_read();
    let gen_id = GeneratorId::from(generator_id);

    let generator_lock = app
        .generator_pool
        .get(&gen_id)
        .ok_or_else(|| format!("Generator {} not found", generator_id))?;

    let generator = generator_lock
        .read()
        .map_err(|e| format!("Failed to read generator lock: {}", e))?;

    let ui_generator = UiGeneratorInstance::from(&*generator);
    Ok(ui_generator)
}

pub fn get_effect(track_id: u32, effect_id: u32) -> Result<UiEffectInstance, String> {
    let app = get_app_read();
    let mixer_state = &app.mixer;
    let channel = mixer_state
        .channels
        .get(&track_id.into())
        .ok_or("Channel not found".to_owned())?;
    let effect = channel
        .effects
        .iter()
        .find(|e| e.id.to_u32() == effect_id)
        .ok_or("Effect instance not found".to_owned())?;
    Ok(effect.into())
}

pub fn get_effect_from_master(effect_id: u32) -> Result<UiEffectInstance, String> {
    let app = get_app_read();
    let mixer_state = &app.mixer;
    let channel = mixer_state.master_bus.as_ref();
    let effect = channel
        .effects
        .iter()
        .find(|e| e.id.to_u32() == effect_id)
        .ok_or("Effect instance not found".to_owned())?;
    Ok(effect.into())
}

pub fn get_effects_from_track(track_id: u32) -> Result<Vec<UiEffectInstance>, String> {
    let app = get_app_read();
    let mixer_state = &app.mixer;
    let channel = mixer_state
        .channels
        .get(&track_id.into())
        .ok_or("Channel not found".to_owned())?;
    let effects = channel.effects.iter().map(|e| e.into()).collect();
    Ok(effects)
}

pub fn get_master_effects() -> Result<Vec<UiEffectInstance>, String> {
    let app = get_app_read();
    let mixer_state = &app.mixer;
    let master_channel = mixer_state.master_bus.as_ref();
    let effects = master_channel.effects.iter().map(|e| e.into()).collect();
    Ok(effects)
}

/// Get parameter specifications for a generator plugin.
///
/// With the lock-free architecture, the live plugin instance runs on the audio thread
/// and cannot be accessed directly. Instead, we use the registry factory to create
/// a temporary plugin instance for querying its static parameter specifications.
/// This is safe because parameter specs are static metadata that don't depend on state.
///
/// The returned specs include the current stored parameter values (from generator pool)
/// which may differ from defaults if the user has modified them.
pub fn get_generator_parameter_specs(generator_id: u32) -> Result<Vec<UiPluginParameter>, String> {
    let app = get_app_read();
    let gen_id = GeneratorId::from(generator_id);

    let generator_lock = app
        .generator_pool
        .get(&gen_id)
        .ok_or_else(|| format!("Generator {} not found", generator_id))?;

    let generator = generator_lock
        .read()
        .map_err(|e| format!("Failed to read generator lock: {}", e))?;

    if let GeneratorInstanceType::Plugin(ref plugin_instance) = generator.instance_type {
        let registry = ctx().plugin_registry.read().unwrap();

        // Try ID-based lookup first (preferred), then fall back to name
        let specs = if plugin_instance.registry_id > 0 {
            registry.get_generator_parameter_specs_by_id(plugin_instance.registry_id)
        } else {
            registry
                .get_generator_id_by_name(&plugin_instance.name)
                .and_then(|id| registry.get_generator_parameter_specs_by_id(id))
        };

        if let Some(specs) = specs {
            let ui_specs: Vec<UiPluginParameter> = specs
                .into_iter()
                .map(|p| {
                    // Use stored parameter value if available, otherwise default
                    let value = plugin_instance
                        .parameters
                        .get(&p.id)
                        .copied()
                        .unwrap_or(p.default_value);

                    UiPluginParameter {
                        id: p.id,
                        name: p.name,
                        group: p.group,
                        value,
                        min: p.min,
                        max: p.max,
                        default_value: p.default_value,
                        step: p.step,
                        param_type: UiParameterType::from(p.value_type),
                        choices: p.choices,
                    }
                })
                .collect();

            Ok(ui_specs)
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

/// Set a parameter on a generator plugin.
///
/// This sends a command to the audio thread to update the parameter.
/// The parameter value is also stored in PluginInstance for persistence.
///
/// # Arguments
/// * `generator_id` - The ID of the generator
/// * `param_id` - The parameter ID
/// * `value` - The new value for the parameter
pub fn set_generator_parameter(generator_id: u32, param_id: u32, value: f32) -> Result<(), String> {
    let gen_id = GeneratorId::from(generator_id);

    // Send command to audio thread (lock-free)
    if let Some(sender) = ctx().command_sender.lock().unwrap().as_mut() {
        let _ = sender.push(AudioCommand::SetGeneratorParameter {
            generator_id: gen_id,
            param_id,
            value,
        });
    }

    // Also update the stored parameter value for persistence
    {
        let app = get_app_write();

        if let Some(generator_lock) = app.generator_pool.get(&gen_id) {
            let mut generator = generator_lock
                .write()
                .map_err(|e| format!("Failed to write generator lock: {}", e))?;

            if let GeneratorInstanceType::Plugin(ref mut plugin_instance) = generator.instance_type
            {
                plugin_instance.parameters.insert(param_id, value);
            }
        }
    }

    broadcast_state_change();
    Ok(())
}

/// Get a parameter value from a generator plugin.
///
/// Returns the stored parameter value for the given parameter ID.
/// With lock-free architecture, we read from the stored parameters,
/// not the live plugin on the audio thread.
pub fn get_generator_parameter(generator_id: u32, param_id: u32) -> Result<f32, String> {
    let app = get_app_read();
    let gen_id = GeneratorId::from(generator_id);

    let generator_lock = app
        .generator_pool
        .get(&gen_id)
        .ok_or_else(|| format!("Generator {} not found", generator_id))?;

    let generator = generator_lock
        .read()
        .map_err(|e| format!("Failed to read generator lock: {}", e))?;

    if let GeneratorInstanceType::Plugin(ref plugin_instance) = generator.instance_type {
        // Read from stored parameters
        plugin_instance
            .parameters
            .get(&param_id)
            .copied()
            .ok_or_else(|| format!("Parameter {} not found", param_id))
    } else {
        Err("Generator is not a plugin type".to_string())
    }
}

// ============================================================================
// PARAMETER FEEDBACK API (Audio -> UI)
// ============================================================================

static PENDING_FEEDBACK: std::sync::Mutex<Vec<AudioFeedback>> = std::sync::Mutex::new(Vec::new());

/// Parameter snapshot from the audio thread (DTO)
#[derive(Clone, Debug)]
pub struct UiGeneratorParameterSnapshot {
    pub generator_id: u32,
    pub parameters: Vec<UiParameterValue>,
}

#[derive(Clone, Debug)]
pub enum UiEffectTarget {
    Track(u32),
    Master,
    Bus(u32),
}

impl From<EffectTarget> for UiEffectTarget {
    fn from(target: EffectTarget) -> Self {
        match target {
            EffectTarget::Track(track_id) => UiEffectTarget::Track(track_id.into()),
            EffectTarget::Master => UiEffectTarget::Master,
            EffectTarget::Bus(bus_id) => UiEffectTarget::Bus(bus_id.into()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UiEffectParameterSnapshot {
    pub target: UiEffectTarget,
    pub effect_id: u32,
    pub parameters: Vec<UiParameterValue>,
}

/// Single parameter value from the audio thread
#[derive(Clone, Debug)]
pub struct UiParameterValue {
    pub param_id: u32,
    pub value: f32,
}

/// Request a parameter snapshot from the audio thread.
///
/// This sends a query to the audio thread, which will respond with the
/// current parameter values via the feedback channel. Use `poll_parameter_feedback`
/// to receive the response.
pub fn query_generator_parameters(generator_id: u32) -> Result<(), String> {
    let gen_id = GeneratorId::from(generator_id);

    if let Some(sender) = ctx().command_sender.lock().unwrap().as_mut() {
        sender
            .push(AudioCommand::QueryGeneratorParameters {
                generator_id: gen_id,
            })
            .map_err(|_| "Command queue full".to_string())?;
        Ok(())
    } else {
        Err("Audio stream not initialized".to_string())
    }
}

/// Poll for parameter feedback from the audio thread.
///
/// This should be called periodically (e.g., in a timer or on parameter screen)
/// to receive parameter updates from the audio thread. Returns all pending
/// parameter snapshots.
pub fn poll_generator_parameter_feedback() -> Vec<UiGeneratorParameterSnapshot> {
    let mut snapshots = Vec::new();

    let mut pending = PENDING_FEEDBACK.lock().unwrap();

    if let Some(consumer) = ctx().feedback_consumer.lock().unwrap().as_mut() {
        // Drain all pending feedback messages
        while let Ok(feedback) = consumer.pop() {
            pending.push(feedback);
        }
    }

    pending.retain(|feedback| {
        match feedback {
            AudioFeedback::GeneratorParameterSnapshot(snapshot) => {
                let ui_snapshot = UiGeneratorParameterSnapshot {
                    generator_id: snapshot.generator_id.into(),
                    parameters: snapshot
                        .parameters
                        .clone()
                        .into_iter()
                        .map(|(param_id, value)| UiParameterValue { param_id, value })
                        .collect(),
                };
                snapshots.push(ui_snapshot);
                false
            }
            AudioFeedback::GeneratorParameterChanged(update) => {
                // Single parameter change - could be from automation
                // Convert to a single-parameter snapshot
                let ui_snapshot = UiGeneratorParameterSnapshot {
                    generator_id: update.generator_id.into(),
                    parameters: vec![UiParameterValue {
                        param_id: update.param_id,
                        value: update.value,
                    }],
                };
                snapshots.push(ui_snapshot);
                false
            }
            _ => true, // Keep effect feedbacks
        }
    });

    snapshots
}

/// Sync parameter values from audio thread to stored parameters.
///
/// Call this after `poll_parameter_feedback` to update the stored parameters
/// with the latest values from the audio thread.
pub fn sync_generator_parameters_from_audio(snapshots: &[UiGeneratorParameterSnapshot]) {
    let app = get_app_write();

    for snapshot in snapshots {
        let gen_id = GeneratorId::from(snapshot.generator_id);

        if let Some(generator_lock) = app.generator_pool.get(&gen_id) {
            if let Ok(mut generator) = generator_lock.write() {
                if let GeneratorInstanceType::Plugin(ref mut plugin_instance) =
                    generator.instance_type
                {
                    for param in &snapshot.parameters {
                        plugin_instance
                            .parameters
                            .insert(param.param_id, param.value);
                    }
                }
            }
        }
    }
}

// Do the same for effect plugin
pub fn poll_effect_parameter_feedback() -> Vec<UiEffectParameterSnapshot> {
    let mut snapshots = Vec::new();

    let mut pending = PENDING_FEEDBACK.lock().unwrap();

    if let Some(consumer) = ctx().feedback_consumer.lock().unwrap().as_mut() {
        // Drain all pending feedback messages
        while let Ok(feedback) = consumer.pop() {
            pending.push(feedback);
        }
    }

    pending.retain(|feedback| {
        match feedback {
            AudioFeedback::EffectParameterSnapshot(snapshot) => {
                let ui_snapshot = UiEffectParameterSnapshot {
                    effect_id: snapshot.effect_id.into(),
                    parameters: snapshot
                        .parameters
                        .clone()
                        .into_iter()
                        .map(|(param_id, value)| UiParameterValue { param_id, value })
                        .collect(),
                    target: snapshot.target.clone().into(),
                };
                snapshots.push(ui_snapshot);
                false
            }
            AudioFeedback::EffectParameterChanged(update) => {
                // Single parameter change - could be from automation
                // Convert to a single-parameter snapshot
                let ui_snapshot = UiEffectParameterSnapshot {
                    effect_id: update.effect_id.into(),
                    parameters: vec![UiParameterValue {
                        param_id: update.param_id,
                        value: update.value,
                    }],
                    target: update.target.clone().into(),
                };
                snapshots.push(ui_snapshot);
                false
            }
            _ => true, // Keep generator feedbacks
        }
    });

    snapshots
}

pub fn sync_effect_parameters_from_audio(snapshots: &[UiEffectParameterSnapshot]) {
    let mut app = get_app_write();

    for snapshot in snapshots {
        let effect_id = EffectId::from(snapshot.effect_id);

        match snapshot.target {
            UiEffectTarget::Master => {
                // Update Master Bus Effect
                let master = Arc::make_mut(&mut app.mixer.master_bus);
                if let Some(effect) = master.effects.iter_mut().find(|e| e.id == effect_id) {
                    let plugin = Arc::make_mut(&mut effect.instance);
                    for param in &snapshot.parameters {
                        plugin.parameters.insert(param.param_id, param.value);
                    }
                }
            }
            UiEffectTarget::Track(track_id_u32) => {
                // Update specific Track Effect
                let track_id = TrackId::from(track_id_u32);
                if let Some(channel_arc) = app.mixer.channels.get_mut(&track_id) {
                    let channel = Arc::make_mut(channel_arc);
                    if let Some(effect) = channel.effects.iter_mut().find(|e| e.id == effect_id) {
                        let plugin = Arc::make_mut(&mut effect.instance);
                        for param in &snapshot.parameters {
                            plugin.parameters.insert(param.param_id, param.value);
                        }
                    }
                }
            }
            UiEffectTarget::Bus(bus_id_u32) => {
                // Update specific Bus Effect
                let bus_id = BusId::from(bus_id_u32);
                if let Some(bus) = app.mixer.buses.get_mut(&bus_id) {
                    let bus_mut = Arc::make_mut(bus);
                    if let Some(effect) = bus_mut
                        .channel
                        .effects
                        .iter_mut()
                        .find(|e| e.id == effect_id)
                    {
                        let plugin = Arc::make_mut(&mut effect.instance);
                        for param in &snapshot.parameters {
                            plugin.parameters.insert(param.param_id, param.value);
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// EFFECT PARAMETER API (mirrors Generator Parameter API)
// ============================================================================

/// Get parameter specifications for a track effect plugin.
///
/// Creates a temporary plugin instance from the registry to query static parameter
/// specs, then overlays the current stored parameter values.
pub fn get_effect_parameter_specs(
    target: UiEffectTarget,
    effect_id: u32,
) -> Result<Vec<UiPluginParameter>, String> {
    let app = get_app_read();
    let effect_id_typed = EffectId::from(effect_id);

    let (plugin_name, plugin_registry_id, plugin_parameters) = match &target {
        UiEffectTarget::Track(track_id) => {
            let channel = app
                .mixer
                .channels
                .get(&TrackId::from(*track_id))
                .ok_or_else(|| format!("Track channel {} not found", track_id))?;
            let effect = channel
                .effects
                .iter()
                .find(|e| e.id == effect_id_typed)
                .ok_or_else(|| format!("Effect {} not found", effect_id))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
        UiEffectTarget::Bus(bus_id) => {
            let bus = app
                .mixer
                .buses
                .get(&BusId::from(*bus_id))
                .ok_or_else(|| format!("Bus {} not found", bus_id))?;
            let effect = bus
                .channel
                .effects
                .iter()
                .find(|e| e.id == effect_id_typed)
                .ok_or_else(|| format!("Effect {} not found", effect_id))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
        UiEffectTarget::Master => {
            let effect = app
                .mixer
                .master_bus
                .effects
                .iter()
                .find(|e| e.id == effect_id_typed)
                .ok_or_else(|| format!("Effect {} not found", effect_id))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
    };

    // Create a temporary plugin instance to get static parameter specs
    let registry = ctx().plugin_registry.read().unwrap();

    let specs = if plugin_registry_id > 0 {
        registry.get_effect_parameter_specs_by_id(plugin_registry_id)
    } else {
        registry
            .get_effect_id_by_name(&plugin_name)
            .and_then(|id| registry.get_effect_parameter_specs_by_id(id))
    };

    if let Some(specs) = specs {
        let ui_specs: Vec<UiPluginParameter> = specs
            .into_iter()
            .map(|p| {
                // Use stored parameter value if available, otherwise default
                let value = plugin_parameters
                    .get(&p.id)
                    .copied()
                    .unwrap_or(p.default_value);

                UiPluginParameter {
                    id: p.id,
                    name: p.name,
                    group: p.group,
                    value,
                    min: p.min,
                    max: p.max,
                    default_value: p.default_value,
                    step: p.step,
                    param_type: UiParameterType::from(p.value_type),
                    choices: p.choices,
                }
            })
            .collect();

        Ok(ui_specs)
    } else {
        Err(format!(
            "Effect '{}' (registry_id={}) not found in registry",
            plugin_name, plugin_registry_id
        ))
    }
}

/// Set a parameter on a track effect plugin.
///
/// Sends a command to the audio thread to update the parameter and also
/// persists the value in the stored PluginInstance parameters.
pub fn set_effect_parameter(
    target: UiEffectTarget,
    effect_id: u32,
    param_id: u32,
    value: f32,
) -> Result<(), String> {
    let effect_id_typed = EffectId::from(effect_id);

    // Send command to audio thread (lock-free)
    if let Some(sender) = ctx().command_sender.lock().unwrap().as_mut() {
        match target {
            UiEffectTarget::Track(track_id) => {
                let _ = sender.push(AudioCommand::SetTrackEffectParameter {
                    track_id: TrackId::from(track_id),
                    effect_id: effect_id_typed,
                    param_id,
                    value,
                });
            }
            UiEffectTarget::Bus(bus_id) => {
                let _ = sender.push(AudioCommand::SetBusEffectParameter {
                    bus_id: BusId::from(bus_id),
                    effect_id: effect_id_typed,
                    param_id,
                    value,
                });
            }
            UiEffectTarget::Master => {
                let _ = sender.push(AudioCommand::SetMasterEffectParameter {
                    effect_id: effect_id_typed,
                    param_id,
                    value,
                });
            }
        }
    }

    // Also update the stored parameter value for persistence
    {
        let mut app = get_app_write();

        match target {
            UiEffectTarget::Track(track_id) => {
                if let Some(channel_arc) = app.mixer.channels.get_mut(&TrackId::from(track_id)) {
                    let channel = Arc::make_mut(channel_arc);
                    if let Some(effect) =
                        channel.effects.iter_mut().find(|e| e.id == effect_id_typed)
                    {
                        let plugin = Arc::make_mut(&mut effect.instance);
                        plugin.parameters.insert(param_id, value);
                    }
                }
            }
            UiEffectTarget::Bus(bus_id) => {
                if let Some(bus) = app.mixer.buses.get_mut(&BusId::from(bus_id)) {
                    let bus_mut = Arc::make_mut(bus);
                    if let Some(effect) = bus_mut
                        .channel
                        .effects
                        .iter_mut()
                        .find(|e| e.id == effect_id_typed)
                    {
                        let plugin = Arc::make_mut(&mut effect.instance);
                        plugin.parameters.insert(param_id, value);
                    }
                }
            }
            UiEffectTarget::Master => {
                let master = Arc::make_mut(&mut app.mixer.master_bus);
                if let Some(effect) = master.effects.iter_mut().find(|e| e.id == effect_id_typed) {
                    let plugin = Arc::make_mut(&mut effect.instance);
                    plugin.parameters.insert(param_id, value);
                }
            }
        }
    }

    broadcast_state_change();
    Ok(())
}

/// Request a parameter snapshot from the audio thread for a track effect.
///
/// The audio thread will respond via AudioFeedback::EffectParameterSnapshot,
/// which can be polled using `poll_effect_parameter_feedback`.
pub fn query_effect_parameters(target: UiEffectTarget, effect_id: u32) -> Result<(), String> {
    let effect_id_typed = EffectId::from(effect_id);

    if let Some(sender) = ctx().command_sender.lock().unwrap().as_mut() {
        match target {
            UiEffectTarget::Track(track_id) => {
                let _ = sender.push(AudioCommand::QueryTrackEffectParameters {
                    track_id: TrackId::from(track_id),
                    effect_id: effect_id_typed,
                });
            }
            UiEffectTarget::Bus(bus_id) => {
                let _ = sender.push(AudioCommand::QueryBusEffectParameters {
                    bus_id: BusId::from(bus_id),
                    effect_id: effect_id_typed,
                });
            }
            UiEffectTarget::Master => {
                let _ = sender.push(AudioCommand::QueryMasterEffectParameters {
                    effect_id: effect_id_typed,
                });
            }
        }
        Ok(())
    } else {
        Err("Audio stream not initialized".to_string())
    }
}

// ============================================================================
// EQ RESPONSE CURVE API
// ============================================================================

/// A point on the EQ response curve (DTO for FRB)
#[derive(Clone, Debug)]
pub struct UiResponseCurvePoint {
    pub frequency: f32,
    pub magnitude_db: f32,
}

/// Compute the magnitude response curve for a parametric EQ effect on a track.
///
/// Creates a temporary plugin instance, applies stored parameters, and evaluates
/// the exact biquad transfer function at log-spaced frequency points.
pub fn get_eq_response_curve(
    target: UiEffectTarget,
    effect_id: u32,
    num_points: u32,
) -> Result<Vec<UiResponseCurvePoint>, String> {
    let app = get_app_read();
    let effect_id_typed = EffectId::from(effect_id);

    let (plugin_name, plugin_registry_id, plugin_parameters) = match target {
        UiEffectTarget::Track(track_id) => {
            let channel = app
                .mixer
                .channels
                .get(&TrackId::from(track_id))
                .ok_or_else(|| format!("Track channel {} not found", track_id))?;
            let effect = channel
                .effects
                .iter()
                .find(|e| e.id == effect_id_typed)
                .ok_or_else(|| format!("Effect {} not found", effect_id))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
        UiEffectTarget::Bus(bus_id) => {
            let bus = app
                .mixer
                .buses
                .get(&BusId::from(bus_id))
                .ok_or_else(|| format!("Bus {} not found", bus_id))?;
            let effect = bus
                .channel
                .effects
                .iter()
                .find(|e| e.id == effect_id_typed)
                .ok_or_else(|| format!("Effect {} not found", effect_id))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
        UiEffectTarget::Master => {
            let effect = app
                .mixer
                .master_bus
                .effects
                .iter()
                .find(|e| e.id == effect_id_typed)
                .ok_or_else(|| format!("Effect {} not found", effect_id))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
    };

    // Create a temporary plugin from registry
    let registry = ctx().plugin_registry.read().unwrap();
    let temp_plugin = if plugin_registry_id > 0 {
        registry
            .create_effect_by_id(plugin_registry_id)
            .map(|(plugin, _)| plugin)
    } else {
        registry.create_effect(&plugin_name)
    };

    let mut temp_plugin =
        temp_plugin.ok_or_else(|| format!("Effect '{}' not found in registry", plugin_name))?;

    // Apply stored parameters to the temp plugin
    for (&param_id, &value) in &plugin_parameters {
        temp_plugin.set_parameter(param_id, value);
    }

    // Downcast to KarbeatParametricEQ to access compute_magnitude_response
    use crate::plugin::effect::parametric_eq::KarbeatParametricEQ;
    let eq = temp_plugin
        .as_any()
        .downcast_ref::<KarbeatParametricEQ>()
        .ok_or_else(|| "Effect is not a Parametric EQ".to_string())?;

    let response = eq.engine.compute_magnitude_response(num_points as usize);

    Ok(response
        .into_iter()
        .map(|(freq, mag_db)| UiResponseCurvePoint {
            frequency: freq,
            magnitude_db: mag_db,
        })
        .collect())
}
