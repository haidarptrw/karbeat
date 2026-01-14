use crate::{
    api::project::UiGeneratorInstance,
    broadcast_state_change,
    commands::AudioCommand,
    core::project::generator::{GeneratorId, GeneratorInstanceType},
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

/// Plugin info with ID for UI display
#[derive(Clone, Debug)]
pub struct UiPluginInfo {
    pub id: u32,
    pub name: String,
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

// pub fn get_effect(effect_id: u32) -> Result<UiEffectInstance, String> {

// }

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
        // Create a temporary plugin instance to get static parameter specs.
        // Use registry_id if available (preferred), otherwise fall back to name lookup.
        let registry = ctx().plugin_registry.read().unwrap();

        // Try ID-based lookup first (preferred), then fall back to name
        let temp_plugin = if plugin_instance.registry_id > 0 {
            registry
                .create_generator_by_id(plugin_instance.registry_id)
                .map(|(plugin, _)| plugin)
        } else {
            registry.create_generator(&plugin_instance.name)
        };

        if let Some(temp_plugin) = temp_plugin {
            let specs = temp_plugin.get_parameter_specs();
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

/// Parameter snapshot from the audio thread
#[derive(Clone, Debug)]
pub struct UiParameterSnapshot {
    pub generator_id: u32,
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
pub fn poll_parameter_feedback() -> Vec<UiParameterSnapshot> {
    use crate::commands::AudioFeedback;

    let mut snapshots = Vec::new();

    if let Some(consumer) = ctx().feedback_consumer.lock().unwrap().as_mut() {
        // Drain all pending feedback messages
        while let Ok(feedback) = consumer.pop() {
            match feedback {
                AudioFeedback::ParameterSnapshot(snapshot) => {
                    let ui_snapshot = UiParameterSnapshot {
                        generator_id: snapshot.generator_id.into(),
                        parameters: snapshot
                            .parameters
                            .into_iter()
                            .map(|(param_id, value)| UiParameterValue { param_id, value })
                            .collect(),
                    };
                    snapshots.push(ui_snapshot);
                }
                AudioFeedback::ParameterChanged(update) => {
                    // Single parameter change - could be from automation
                    // Convert to a single-parameter snapshot
                    let ui_snapshot = UiParameterSnapshot {
                        generator_id: update.generator_id.into(),
                        parameters: vec![UiParameterValue {
                            param_id: update.param_id,
                            value: update.value,
                        }],
                    };
                    snapshots.push(ui_snapshot);
                }
            }
        }
    }

    snapshots
}

/// Sync parameter values from audio thread to stored parameters.
///
/// Call this after `poll_parameter_feedback` to update the stored parameters
/// with the latest values from the audio thread.
pub fn sync_parameters_from_audio(snapshots: &[UiParameterSnapshot]) {
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
