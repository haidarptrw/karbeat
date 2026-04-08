use crate::api::{mixer::UiEffectInstance, project::UiGeneratorInstance};
use flutter_rust_bridge::frb;
use karbeat_core::api::plugin_api;
use karbeat_core::core::project::{
    generator::GeneratorId,
    mixer::{BusId, EffectId},
    TrackId,
};
use karbeat_core::plugin_types::ParameterValueType;
use karbeat_plugins::registry::PluginInfo;
use karbeat_utils::parser::FromPluginCommand;

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

#[frb(ignore)]
impl UiPluginInfo {
    pub fn from_info_to_synth(info: &PluginInfo) -> Self {
        Self {
            id: info.id,
            name: info.name.clone(),
            plugin_type: KarbeatPluginType::Generator,
        }
    }

    pub fn from_info_to_effect(info: &PluginInfo) -> Self {
        Self {
            id: info.id,
            name: info.name.clone(),
            plugin_type: KarbeatPluginType::Effect,
        }
    }
}

/// Get all available generators with their registry IDs (preferred for UI)
pub fn get_available_generators_with_ids() -> Result<Vec<UiPluginInfo>, String> {
    Ok(plugin_api::get_available_generators(|plugin_info| {
        UiPluginInfo::from_info_to_synth(plugin_info)
    }))
}

/// Get all available effects with their registry IDs (preferred for UI)
pub fn get_available_effects_with_ids() -> Result<Vec<UiPluginInfo>, String> {
    Ok(plugin_api::get_available_effects(|plugin_info| {
        UiPluginInfo::from_info_to_effect(plugin_info)
    }))
}

/// Get a single generator state from the Generator Pool
pub fn get_generator(generator_id: u32) -> Result<UiGeneratorInstance, String> {
    let gen_id = GeneratorId::from(generator_id);
    let gen_instance = plugin_api::get_generator(&gen_id, |g| UiGeneratorInstance::from(g))
        .ok_or_else(|| format!("Generator {} not found", generator_id))?;
    Ok(gen_instance)
}

pub fn get_effect(track_id: u32, effect_id: u32) -> Result<UiEffectInstance, String> {
    let track_id = TrackId::from(track_id);
    let effect_id = EffectId::from(effect_id);
    plugin_api::get_effect(&track_id, &effect_id, |e| UiEffectInstance::from(e))
        .ok_or_else(|| format!("Effect {} not found", effect_id.0))
}

pub fn get_effect_from_master(effect_id: u32) -> Result<UiEffectInstance, String> {
    let effect_id_typed = EffectId::from(effect_id);
    plugin_api::get_effect_from_master(&effect_id_typed, |e| UiEffectInstance::from(e))
        .ok_or_else(|| format!("Effect {} not found", effect_id))
}

pub fn get_effects_from_track(track_id: u32) -> Result<Vec<UiEffectInstance>, String> {
    let track_id = TrackId::from(track_id);
    plugin_api::get_effects_from_track(&track_id, |e| UiEffectInstance::from(e))
        .ok_or_else(|| format!("Track {} not found", track_id.0))
}

pub fn get_master_effects() -> Vec<UiEffectInstance> {
    plugin_api::get_master_effects(|e| UiEffectInstance::from(e))
}

/// Get parameter specifications for a generator plugin.
pub fn get_generator_parameter_specs(generator_id: u32) -> Result<Vec<UiPluginParameter>, String> {
    let gen_id = GeneratorId::from(generator_id);
    plugin_api::get_generator_parameter_specs(&gen_id, |p, value| UiPluginParameter {
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
    })
}

/// Set a parameter on a generator plugin.
pub fn set_generator_parameter(generator_id: u32, param_id: u32, value: f32) -> Result<(), String> {
    let gen_id = GeneratorId::from(generator_id);
    plugin_api::set_generator_parameter(&gen_id, param_id, value)
}

/// Get a parameter value from a generator plugin.
pub fn get_generator_parameter(generator_id: u32, param_id: u32) -> Result<f32, String> {
    let gen_id = GeneratorId::from(generator_id);
    plugin_api::get_generator_parameter(&gen_id, param_id)
}

// ============================================================================
// PARAMETER FEEDBACK API (Audio -> UI)
// ============================================================================

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

impl From<karbeat_core::commands::EffectTarget> for UiEffectTarget {
    fn from(target: karbeat_core::commands::EffectTarget) -> Self {
        match target {
            karbeat_core::commands::EffectTarget::Track(track_id) => {
                UiEffectTarget::Track(track_id.into())
            }
            karbeat_core::commands::EffectTarget::Master => UiEffectTarget::Master,
            karbeat_core::commands::EffectTarget::Bus(bus_id) => UiEffectTarget::Bus(bus_id.into()),
        }
    }
}

impl Into<karbeat_core::commands::EffectTarget> for UiEffectTarget {
    fn into(self) -> karbeat_core::commands::EffectTarget {
        match self {
            UiEffectTarget::Track(id) => {
                karbeat_core::commands::EffectTarget::Track(TrackId::from(id))
            }
            UiEffectTarget::Master => karbeat_core::commands::EffectTarget::Master,
            UiEffectTarget::Bus(id) => karbeat_core::commands::EffectTarget::Bus(BusId::from(id)),
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
pub fn query_generator_parameters(generator_id: u32) -> Result<(), String> {
    let gen_id = GeneratorId::from(generator_id);
    plugin_api::query_generator_parameters(&gen_id)
}

/// Poll for parameter feedback from the audio thread.
pub fn poll_generator_parameter_feedback() -> Vec<UiGeneratorParameterSnapshot> {
    plugin_api::poll_generator_parameter_feedback(|generator_id, parameters| {
        UiGeneratorParameterSnapshot {
            generator_id: generator_id.into(),
            parameters: parameters
                .into_iter()
                .map(|(param_id, value)| UiParameterValue { param_id, value })
                .collect(),
        }
    })
}

/// Sync parameter values from audio thread to stored parameters.
pub fn sync_generator_parameters_from_audio(snapshots: &[UiGeneratorParameterSnapshot]) {
    let items = snapshots.iter().map(|snapshot| {
        let params = snapshot
            .parameters
            .iter()
            .map(|p| (p.param_id, p.value))
            .collect::<Vec<_>>();
        (GeneratorId::from(snapshot.generator_id), params)
    });
    plugin_api::sync_generator_parameters_from_audio(items);
}

// Do the same for effect plugin
pub fn poll_effect_parameter_feedback() -> Vec<UiEffectParameterSnapshot> {
    plugin_api::poll_effect_parameter_feedback(|target, effect_id, parameters| {
        UiEffectParameterSnapshot {
            effect_id: effect_id.into(),
            target: target.into(),
            parameters: parameters
                .into_iter()
                .map(|(param_id, value)| UiParameterValue { param_id, value })
                .collect(),
        }
    })
}

pub fn sync_effect_parameters_from_audio(snapshots: &[UiEffectParameterSnapshot]) {
    let items = snapshots.iter().map(|snapshot| {
        let params = snapshot
            .parameters
            .iter()
            .map(|p| (p.param_id, p.value))
            .collect::<Vec<_>>();
        (
            snapshot.target.clone().into(),
            EffectId::from(snapshot.effect_id),
            params,
        )
    });
    plugin_api::sync_effect_parameters_from_audio(items);
}

// ============================================================================
// EFFECT PARAMETER API
// ============================================================================

pub fn get_effect_parameter_specs(
    target: UiEffectTarget,
    effect_id: u32,
) -> Result<Vec<UiPluginParameter>, String> {
    let effect_target = target.into();
    let effect_id_typed = EffectId::from(effect_id);

    plugin_api::get_effect_parameter_specs(&effect_target, &effect_id_typed, |p, value| {
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
}

pub fn set_effect_parameter(
    target: UiEffectTarget,
    effect_id: u32,
    param_id: u32,
    value: f32,
) -> Result<(), String> {
    let effect_target = target.into();
    let effect_id_typed = EffectId::from(effect_id);
    plugin_api::set_effect_parameter(&effect_target, &effect_id_typed, param_id, value)
}

pub fn query_effect_parameters(target: UiEffectTarget, effect_id: u32) -> Result<(), String> {
    let effect_target = target.into();
    let effect_id_typed = EffectId::from(effect_id);
    plugin_api::query_effect_parameters(&effect_target, &effect_id_typed)
}

// ============================================================================
// 1. STATELESS COMMANDS (Operates on defaults from the Registry)
// ============================================================================

pub fn execute_plugin_command_generator(
    gen_registry_id: u32,
    command: String,
    payload_json: String,
) -> Option<String> {
    let payload_value: serde_json::Value =
        serde_json::from_str(&payload_json).unwrap_or(serde_json::json!({}));

    plugin_api::execute_plugin_command_generator(gen_registry_id, &command, &payload_value)
        .map(|v| v.to_string())
}

pub fn execute_plugin_command_effect(
    effect_registry_id: u32,
    command: String,
    payload_json: String,
) -> Option<String> {
    let payload_value: serde_json::Value =
        serde_json::from_str(&payload_json).unwrap_or(serde_json::json!({}));

    plugin_api::execute_plugin_command_effect(effect_registry_id, &command, &payload_value)
        .map(|v| v.to_string())
}

// ============================================================================
// 2. STATEFUL COMMANDS (Operates on active instances with User Parameters applied)
// ============================================================================

pub fn execute_effect_instance_command(
    target: UiEffectTarget,
    effect_id: u32,
    command: String,
    payload_json: String,
) -> Result<String, String> {
    let effect_target = target.into();
    let effect_id_typed = EffectId::from(effect_id);
    let payload_value: serde_json::Value =
        serde_json::from_str(&payload_json).unwrap_or(serde_json::json!({}));

    plugin_api::execute_effect_instance_command(
        &effect_target,
        &effect_id_typed,
        &command,
        &payload_value,
    )
    .map(|v| v.to_string())
}

pub fn execute_generator_instance_command(
    generator_id: u32,
    command: String,
    payload_json: String,
) -> Result<String, String> {
    let gen_id_typed = GeneratorId::from(generator_id);
    let payload_value: serde_json::Value =
        serde_json::from_str(&payload_json).unwrap_or(serde_json::json!({}));

    plugin_api::execute_generator_instance_command(&gen_id_typed, &command, &payload_value)
        .map(|v| v.to_string())
}

#[frb(ignore)]
pub fn parse_plugin_response<T: FromPluginCommand>(json_str: &str) -> Result<T, String> {
    let payload: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse JSON string: {}", e))?;

    T::from_json(&payload)
}
