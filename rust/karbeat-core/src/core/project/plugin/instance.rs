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
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PluginInstance {
    /// Registry ID for plugin lookup (stable identifier)
    pub registry_id: u32,
    /// Name of the plugin (for display purposes)
    pub name: String,
    /// Whether this plugin is bypassed
    pub bypass: bool,
    /// Plugin parameter specifications for persistence
    #[serde(default)]
    pub parameter_specs: Vec<karbeat_plugin_types::ParameterSpec>,

    #[serde(default)]
    pub plugin_state: Vec<u8>,
}

impl PartialEq for PluginInstance {
    fn eq(&self, other: &Self) -> bool {
        self.registry_id == other.registry_id
            && self.name == other.name
            && self.bypass == other.bypass
            && self.plugin_state == other.plugin_state
        // Note: We ignore parameter_specs for equality checks because they are just metadata
    }
}

impl PluginInstance {
    /// Create a new plugin instance with name only (backwards compatible)
    pub fn new(name: &str) -> Self {
        Self {
            registry_id: 0,
            name: name.to_string(),
            bypass: false,
            parameter_specs: Vec::new(),
            plugin_state: Vec::new(),
        }
    }

    /// Create a new plugin instance with registry ID and name
    pub fn new_with_id(registry_id: u32, name: &str) -> Self {
        Self {
            registry_id,
            name: name.to_string(),
            bypass: false,
            parameter_specs: Vec::new(),
            plugin_state: Vec::new(),
        }
    }
}
