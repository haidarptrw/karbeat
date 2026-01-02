use crate::plugin::registry::PLUGIN_REGISTRY;

pub fn get_available_generators() -> Result<Vec<String>, String> {
    let registry = PLUGIN_REGISTRY.read().unwrap();
    Ok(registry.list_generators())
}