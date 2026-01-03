use std::{collections::HashMap, sync::{Arc, Mutex}};

use serde::{Deserialize, Serialize};

use crate::core::project::plugin::KarbeatPlugin;

/// Define a plugin instance
/// # Example:
/// ```rust
/// let instance = PluginInstance {
///     name: "Basic Reverb".to_string(),
///     internal_type: "REVERB".to_string(),
///     bypass: false,
///     parameters: HashMap::new(),
///     instance: Some(Arc::new(Mutex::new(EffectWrapper::new(BasicReverb::default(), 48_000.0)))),
/// };
/// ```
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginInstance {
    /// Name of the plugin
    pub name: String,
    /// Internal Type name
    pub internal_type: String, // e.g., "EQ_3BAND", "COMPRESSOR"
    /// Whether this plugin is bypassed
    pub bypass: bool,
    /// Plugin parameters
    pub parameters: HashMap<u32, f32>, // Param ID -> Value

    /// Concrete Plugin Instance
    #[serde(skip)]
    pub instance: Option<Arc<Mutex<KarbeatPlugin>>>,
}