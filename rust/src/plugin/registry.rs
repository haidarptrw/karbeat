// src/core/plugin/registry.rs

use std::collections::HashMap;

use crate::core::project::plugin::KarbeatGenerator;
use crate::plugin::generator::karbeatzer_v2::create_karbeatzer;

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

    /// Create a new registry with default built-in plugins registered
    pub fn new_with_defaults() -> Self {
        let mut registry = Self::new();

        // Karbeatzer V2 - our main synth
        registry.register("Karbeatzer V2", || {
            // We pass None for sample_rate here because 'prepare()' will be called
            // by the engine later with the correct rate.
            Box::new(create_karbeatzer(None))
        });

        registry
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
