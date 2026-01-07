use crate::{
    broadcast_state_change,
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

pub fn get_available_generators() -> Result<Vec<String>, String> {
    let registry = ctx().plugin_registry.read().unwrap();
    Ok(registry.list_generators())
}

/// Get parameter specifications for a generator plugin.
/// Returns a list of all parameters with their metadata for UI generation.
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
        if let Some(ref plugin_arc) = plugin_instance.instance {
            let plugin = plugin_arc
                .lock()
                .map_err(|e| format!("Failed to lock plugin: {}", e))?;

            let specs = plugin.get_parameter_specs();
            let ui_specs: Vec<UiPluginParameter> = specs
                .into_iter()
                .map(|p| UiPluginParameter {
                    id: p.id,
                    name: p.name,
                    group: p.group,
                    value: p.value,
                    min: p.min,
                    max: p.max,
                    default_value: p.default_value,
                    step: p.step,
                    param_type: UiParameterType::from(p.value_type),
                    choices: p.choices,
                })
                .collect();

            Ok(ui_specs)
        } else {
            Err("Plugin instance not initialized".to_string())
        }
    } else {
        Err("Generator is not a plugin type".to_string())
    }
}

/// Set a parameter on a generator plugin.
///
/// # Arguments
/// * `generator_id` - The ID of the generator
/// * `param_id` - The parameter ID
/// * `value` - The new value for the parameter
pub fn set_generator_parameter(generator_id: u32, param_id: u32, value: f32) -> Result<(), String> {
    {
        let app = get_app_read();
        let gen_id = GeneratorId::from(generator_id);

        let generator_lock = app
            .generator_pool
            .get(&gen_id)
            .ok_or_else(|| format!("Generator {} not found", generator_id))?;

        let generator = generator_lock
            .read()
            .map_err(|e| format!("Failed to read generator lock: {}", e))?;

        // Get the plugin instance
        if let GeneratorInstanceType::Plugin(ref plugin_instance) = generator.instance_type {
            if let Some(ref plugin_arc) = plugin_instance.instance {
                let mut plugin = plugin_arc
                    .lock()
                    .map_err(|e| format!("Failed to lock plugin: {}", e))?;
                plugin.set_parameter(param_id, value);
            } else {
                return Err("Plugin instance not initialized".to_string());
            }
        } else {
            return Err("Generator is not a plugin type".to_string());
        }
    }

    // Also update the stored parameter value for persistence
    {
        let mut app = get_app_write();
        let gen_id = GeneratorId::from(generator_id);

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
        if let Some(ref plugin_arc) = plugin_instance.instance {
            let plugin = plugin_arc
                .lock()
                .map_err(|e| format!("Failed to lock plugin: {}", e))?;
            Ok(plugin.get_parameter(param_id))
        } else {
            // Fall back to stored parameters
            plugin_instance
                .parameters
                .get(&param_id)
                .copied()
                .ok_or_else(|| format!("Parameter {} not found", param_id))
        }
    } else {
        Err("Generator is not a plugin type".to_string())
    }
}
