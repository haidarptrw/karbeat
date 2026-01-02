use std::{collections::HashMap, sync::{Arc, Mutex}};

use serde::{Deserialize, Serialize};

use crate::core::project::plugin::KarbeatPlugin;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginInstance {
    pub name: String,
    pub internal_type: String, // e.g., "EQ_3BAND", "COMPRESSOR"
    pub bypass: bool,
    pub parameters: HashMap<u32, f32>, // Param ID -> Value

    #[serde(skip)]
    pub instance: Option<Arc<Mutex<KarbeatPlugin>>>,
}