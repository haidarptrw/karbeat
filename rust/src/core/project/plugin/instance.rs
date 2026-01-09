use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Define a plugin instance descriptor.
///
/// This is a lightweight struct for serialization and UI purposes.
/// The actual plugin processing instance is owned by the audio thread's `AudioPluginState`.
///
/// # Example:
/// ```rust
/// let instance = PluginInstance {
///     name: "Basic Reverb".to_string(),
///     internal_type: "REVERB".to_string(),
///     bypass: false,
///     parameters: HashMap::new(),
/// };
/// ```
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PluginInstance {
    /// Name of the plugin
    pub name: String,
    /// Internal Type name (e.g., "EQ_3BAND", "COMPRESSOR")
    pub internal_type: String,
    /// Whether this plugin is bypassed
    pub bypass: bool,
    /// Plugin parameters for persistence (Param ID -> Value)
    pub parameters: HashMap<u32, f32>,
}

impl PluginInstance {
    pub fn new(name: &str, internal_type: &str) -> Self {
        Self {
            name: name.to_string(),
            internal_type: internal_type.to_string(),
            bypass: false,
            parameters: HashMap::new(),
        }
    }
}
