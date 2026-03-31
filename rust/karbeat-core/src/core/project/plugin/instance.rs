use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Define a plugin instance descriptor.
///
/// This is a lightweight struct for serialization and UI purposes.
/// The actual plugin processing instance is owned by the audio thread's `AudioPluginState`.
///
/// # Example:
/// ```rust,ignore
/// let instance = PluginInstance {
///     registry_id: 0,
///     name: "Basic Reverb".to_string(),
///     bypass: false,
///     parameters: indexmap::IndexMap::new(),
/// };
/// ```
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct PluginInstance {
    /// Registry ID for plugin lookup (stable identifier)
    pub registry_id: u32,
    /// Name of the plugin (for display purposes)
    pub name: String,
    /// Whether this plugin is bypassed
    pub bypass: bool,
    /// Plugin parameters for persistence (Param ID -> Value)
    pub parameters: IndexMap<u32, f32>,
}

impl PluginInstance {
    /// Create a new plugin instance with name only (backwards compatible)
    pub fn new(name: &str) -> Self {
        Self {
            registry_id: 0,
            name: name.to_string(),
            bypass: false,
            parameters: IndexMap::new(),
        }
    }

    /// Create a new plugin instance with registry ID and name
    pub fn new_with_id(registry_id: u32, name: &str) -> Self {
        Self {
            registry_id,
            name: name.to_string(),
            bypass: false,
            parameters: IndexMap::new(),
        }
    }

    /// Create a new plugin instance with registry ID, name, and default parameters
    pub fn new_with_params(registry_id: u32, name: &str, parameters: IndexMap<u32, f32>) -> Self {
        Self {
            registry_id,
            name: name.to_string(),
            bypass: false,
            parameters,
        }
    }
}
