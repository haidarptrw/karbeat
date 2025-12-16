// src/core/plugin/registry.rs

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy; // Add 'once_cell' to Cargo.toml if needed

use crate::core::plugin::{KarbeatGenerator, KarbeatPlugin};
// Import your concrete plugins here
use crate::plugin::generator::karbeatzer::Karbeatzer;

/// A function pointer type that creates a new Generator instance
type GeneratorFactory = Box<dyn Fn() -> Box<dyn KarbeatGenerator + Send + Sync> + Send + Sync>;

pub struct PluginRegistry {
    generators: HashMap<String, GeneratorFactory>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            generators: HashMap::new(),
        }
    }

    /// Register a new plugin factory
    pub fn register<F>(&mut self, name: &str, factory: F)
    where
        F: Fn() -> Box<dyn KarbeatGenerator + Send + Sync> + Send + Sync + 'static,
    {
        self.generators.insert(name.to_string(), Box::new(factory));
    }

    /// Create an instance of a plugin by name
    pub fn create_generator(&self, name: &str) -> Option<Box<dyn KarbeatGenerator + Send + Sync>> {
        if let Some(factory) = self.generators.get(name) {
            Some(factory())
        } else {
            None
        }
    }

    /// Get list of all available generator names (for UI)
    pub fn list_generators(&self) -> Vec<String> {
        self.generators.keys().cloned().collect()
    }
}

// Global Static Registry
pub static PLUGIN_REGISTRY: Lazy<RwLock<PluginRegistry>> = Lazy::new(|| {
    let mut registry = PluginRegistry::new();

    // =========================================================
    // REGISTER YOUR PLUGINS HERE
    // This replaces the match statement in your old Factory
    // =========================================================
    
    // 1. Karbeatzer
    registry.register("Karbeatzer", || {
        // We pass None for sample_rate here because 'prepare()' will be called 
        // by the engine later with the correct rate.
        Box::new(Karbeatzer::new(None)) 
    });

    // 2. Add future plugins here...
    // registry.register("Sampler", || Box::new(Sampler::new()));

    RwLock::new(registry)
});