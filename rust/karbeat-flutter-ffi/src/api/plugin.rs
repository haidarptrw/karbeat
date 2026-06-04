use crate::api::plugins::opaque::ZeroCopyHandle;
use crate::api::{mixer::UiEffectInstance, project::UiGeneratorInstance};
use crate::frb_generated::StreamSink;
use flutter_rust_bridge::frb;
use karbeat_core::api::plugin_api;
use karbeat_core::audio::event::PluginTarget;
use karbeat_core::core::project::TrackId;
use karbeat_core::plugin_types::ParameterValueType;
use karbeat_core::shared::id::*;
use karbeat_plugins::registry::PluginInfo;
use karbeat_utils::parser::FromPluginCommand;
pub use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;

#[frb(opaque)]
pub struct PluginBufferHandle(Arc<Mutex<Vec<f32>>>);

impl PluginBufferHandle {
    /// Reads the current buffer state directly into a Dart Float32List.
    /// This bypasses JSON entirely and takes less than a microsecond.
    #[frb(sync)]
    pub fn read(&self) -> Vec<f32> {
        self.0.lock().clone()
    }
}

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
    pub path: String,
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

#[derive(Clone, Debug)]
pub enum UiParamId {
    Id(u32),
    Path(String),
}

impl UiParamId {
    /// Resolves the FRB enum into the raw u32 hash expected by the audio thread
    pub fn resolve(self) -> u32 {
        match self {
            UiParamId::Id(id) => id,
            UiParamId::Path(path) => karbeat_utils::hash::hash_str(&path),
        }
    }
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

/// A response message arriving from the audio thread containing the zero-copy buffer.
/// Dart uses the `request_id` to correlate with the original command sent via `query_zero_copy_buffer`.
pub struct UiZeroCopyBufferResponse {
    pub request_id: u32,
    /// The opaque handle that Dart can use to read raw memory.
    /// It is `None` if the plugin did not recognize the buffer name.
    pub handle: Option<ZeroCopyHandle>,
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
        path: p.path,
        name: p.name,
        group: p.group,
        value,
        min: p.min as f32,
        max: p.max as f32,
        default_value: p.default_value as f32,
        step: p.step as f32,
        param_type: UiParameterType::from(p.value_type),
        choices: p.choices,
    })
}

/// Set a parameter on a generator plugin.
pub fn set_generator_parameter(
    generator_id: u32,
    param_id: UiParamId,
    value: f32,
) -> Result<(), String> {
    let gen_id = GeneratorId::from(generator_id);
    plugin_api::set_generator_parameter(&gen_id, param_id.resolve(), value)
}

/// Get a parameter value from a generator plugin.
pub fn get_generator_parameter(generator_id: u32, param_id: UiParamId) -> Result<f32, String> {
    let gen_id = GeneratorId::from(generator_id);
    plugin_api::get_generator_parameter(&gen_id, param_id.resolve())
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

impl From<UiEffectTarget> for karbeat_core::commands::EffectTarget {
    fn from(val: UiEffectTarget) -> Self {
        match val {
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
            path: p.path,
            name: p.name,
            group: p.group,
            value,
            min: p.min as f32,
            max: p.max as f32,
            default_value: p.default_value as f32,
            step: p.step as f32,
            param_type: UiParameterType::from(p.value_type),
            choices: p.choices,
        }
    })
}

pub fn set_effect_parameter(
    target: UiEffectTarget,
    effect_id: u32,
    param_id: UiParamId,
    value: f32,
) -> Result<(), String> {
    let effect_target = target.into();
    let effect_id_typed = EffectId::from(effect_id);
    plugin_api::set_effect_parameter(&effect_target, &effect_id_typed, param_id.resolve(), value)
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

// ============================================================================
// Real-time Plugin Command Channel (UI → Audio → UI via StreamSink)
// ============================================================================

/// Identifies which live plugin instance on the audio thread to target.
/// Mirrors `karbeat_core::audio::event::PluginTarget` for FRB exposure.
#[derive(Clone, Debug)]
pub enum UiPluginTarget {
    /// A generator plugin identified by its GeneratorId
    Generator(u32),
    /// A track effect identified by (TrackId, EffectId)
    TrackEffect { track_id: u32, effect_id: u32 },
    /// A bus effect identified by (BusId, EffectId)
    BusEffect { bus_id: u32, effect_id: u32 },
    /// An effect on the master bus
    MasterEffect(u32),
}

impl From<UiPluginTarget> for PluginTarget {
    fn from(val: UiPluginTarget) -> Self {
        match val {
            UiPluginTarget::Generator(id) => PluginTarget::Generator(GeneratorId::from(id)),
            UiPluginTarget::TrackEffect {
                track_id,
                effect_id,
            } => PluginTarget::TrackEffect(TrackId::from(track_id), EffectId::from(effect_id)),
            UiPluginTarget::BusEffect { bus_id, effect_id } => {
                PluginTarget::BusEffect(BusId::from(bus_id), EffectId::from(effect_id))
            }
            UiPluginTarget::MasterEffect(id) => PluginTarget::MasterEffect(EffectId::from(id)),
        }
    }
}

impl From<&PluginTarget> for UiPluginTarget {
    fn from(value: &PluginTarget) -> Self {
        match value {
            PluginTarget::Generator(generator_id) => Self::Generator(generator_id.to_u32()),
            PluginTarget::TrackEffect(track_id, effect_id) => Self::TrackEffect {
                track_id: track_id.to_u32(),
                effect_id: effect_id.to_u32(),
            },
            PluginTarget::BusEffect(bus_id, effect_id) => Self::BusEffect {
                bus_id: bus_id.to_u32(),
                effect_id: effect_id.to_u32(),
            },
            PluginTarget::MasterEffect(effect_id) => Self::MasterEffect(effect_id.to_u32()),
        }
    }
}

/// A response message arriving from the audio thread to Flutter.
/// Flutter uses the `request_id` to correlate with the original command sent
/// via `execute_realtime_plugin_command`.
#[derive(Clone, Debug)]
pub struct UiPluginCommandResponse {
    /// Matches the `request_id` returned by `execute_realtime_plugin_command`
    pub request_id: u32,
    /// JSON-encoded response from the plugin's `execute_custom_command`
    pub response_json: String,
}

/// Dispatches a real-time command to a live plugin instance on the audio thread.
///
/// The plugin's `execute_custom_command` is called from within the audio callback,
/// so the command and payload must be cheap to process. The response arrives
/// asynchronously via the `StreamSink` opened by `create_plugin_message_stream`.
///
/// # Parameters
/// - `target`: Which plugin instance to target.
/// - `command`: Command key string (e.g. `"get_meter"`, `"get_spectrum"`).
/// - `payload_json`: JSON string sent as the command argument. Defaults to `{}`
///   if the string is not valid JSON.
///
/// # Returns
/// `Ok(request_id)` — correlate this with `UiPluginCommandResponse.request_id`
/// in the stream. Returns `Err` if the audio stream is not active or the
/// command queue is full.
pub fn execute_realtime_plugin_command(
    target: UiPluginTarget,
    command: String,
    payload_json: String,
) -> Result<u32, String> {
    let payload: serde_json::Value =
        serde_json::from_str(&payload_json).unwrap_or(serde_json::json!({}));

    plugin_api::execute_plugin_command(target.into(), command, payload)
}

/// Opens a stream that continuously polls the audio→UI feedback channel and
/// forwards any `PluginCommandResponse` messages to Flutter.
///
/// Flutter can subscribe to this stream with a `StreamBuilder<UiPluginCommandResponse>`
/// and use `response.request_id` to match responses to pending commands.
///
/// The polling thread runs at ~16ms intervals (≈60fps). It terminates
/// automatically when Flutter closes the stream (sink returns an error).
pub fn create_plugin_message_stream(
    sink: StreamSink<UiPluginCommandResponse>,
) -> Result<(), String> {
    std::thread::spawn(move || {
        loop {
            let responses = plugin_api::drain_plugin_command_responses(|request_id, response| {
                UiPluginCommandResponse {
                    request_id,
                    response_json: response.to_string(),
                }
            });

            for msg in responses {
                if sink.add(msg).is_err() {
                    // Flutter closed the stream — exit cleanly
                    return;
                }
            }

            // ~60fps poll rate, same as the transport position stream
            std::thread::sleep(Duration::from_millis(16));
        }
    });

    Ok(())
}

/// Dispatches a request to the audio thread to fetch a zero-copy buffer from a live plugin.
///
/// # Parameters
/// - `target`: Which plugin instance to target.
/// - `name`: The requested buffer name (e.g., `"magnitude"` or `"spectrum"`).
///
/// # Returns
/// `Ok(request_id)` — correlate this with `UiZeroCopyBufferResponse.request_id` in the stream.
#[frb]
pub fn query_live_plugin_zero_copy_buf(
    target: UiPluginTarget,
    name: String,
) -> Result<u32, String> {
    plugin_api::query_zero_copy_buffer_from_live_plugin(target.into(), name)
}

/// Opens a stream that continuously polls the audio→UI feedback channel and
/// forwards any requested `ZeroCopyBuffer` handles to Flutter.
///
/// The polling thread runs at ~16ms intervals (≈60fps). It terminates
/// automatically when Flutter closes the stream.
pub fn create_zero_copy_buffer_stream(
    sink: StreamSink<UiZeroCopyBufferResponse>,
) -> Result<(), String> {
    std::thread::spawn(move || {
        loop {
            // Poll the core API. The mapper closure converts the FFI-agnostic
            // ZeroCopyBuffer into the FFI-specific ZeroCopyHandle.
            let responses =
                plugin_api::poll_zero_copy_buffer_from_live_plugin(|request_id, buffer_opt| {
                    UiZeroCopyBufferResponse {
                        request_id,
                        handle: buffer_opt.map(|buffer| ZeroCopyHandle::new(buffer)),
                    }
                });

            for msg in responses {
                if sink.add(msg).is_err() {
                    // Flutter closed the stream - exit gracefully
                    return;
                }
            }

            // ~60fps poll rate
            std::thread::sleep(Duration::from_millis(16));
        }
    });

    Ok(())
}
