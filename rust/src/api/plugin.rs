use crate::plugin::registry::PLUGIN_REGISTRY;

pub fn get_available_generators() -> Result<Vec<String>, String> {
    let registry = PLUGIN_REGISTRY.read().map_err(|e| e.to_string())?;
    Ok(registry.list_generators())
}