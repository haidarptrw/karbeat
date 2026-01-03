use crate::ctx;

pub fn get_available_generators() -> Result<Vec<String>, String> {
    let registry = ctx().plugin_registry.read().unwrap();
    Ok(registry.list_generators())
}
