// src/core/plugin/registry.rs

use hashbrown::HashMap;
use karbeat_plugin_types::ParameterSpec;

// use crate::effect::compressor::create_compressor;
use karbeat_plugin_api::traits::{KarbeatEffect, KarbeatGenerator};

use crate::{effect::parametric_eq::KarbeatParametricEQ, generator::karbeatzer_v2::KarbeatzerV2};

/// A function pointer type that creates a new Generator instance
type GeneratorFactory = Box<dyn Fn() -> Box<dyn KarbeatGenerator + Send + Sync> + Send + Sync>;

/// A function pointer type that creates a new Effect instance
type EffectFactory = Box<dyn Fn() -> Box<dyn KarbeatEffect + Send + Sync> + Send + Sync>;

/// Metadata stored for each registered generator
struct RegisteredGenerator {
    name: String,
    factory: GeneratorFactory,
    parameter_specs: Vec<ParameterSpec>,
}

/// Metadata stored for each registered effect
struct RegisteredEffect {
    name: String,
    factory: EffectFactory,
    parameter_specs: Vec<ParameterSpec>,
}

/// Information about a registered plugin (for UI display)
#[derive(Clone, Debug)]
pub struct PluginInfo {
    pub id: u32,
    pub name: String,
}

pub struct PluginRegistry {
    /// Generators stored by ID
    generators: HashMap<u32, RegisteredGenerator>,
    /// Effects stored by ID
    effects: HashMap<u32, RegisteredEffect>,
    /// Counter for assigning generator IDs
    generator_id_counter: u32,
    /// Counter for assigning effect IDs
    effect_id_counter: u32,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            generators: HashMap::new(),
            effects: HashMap::new(),
            generator_id_counter: 0,
            effect_id_counter: 0,
        }
    }

    /// Create a new registry with default built-in plugins registered
    pub fn new_with_defaults() -> Self {
        let mut registry = Self::new();

        // Karbeatzer V2 - our main synth
        registry.register_generator("Karbeatzer V2", || Box::new(KarbeatzerV2::build()));

        // Parametric EQ
        registry.register_effect("Parametric EQ", || Box::new(KarbeatParametricEQ::build()));

        registry
    }

    /// Register a new generator/synth plugin factory.
    /// Returns the assigned registry ID.
    pub fn register_generator<F>(&mut self, name: &str, factory: F) -> u32
    where
        F: Fn() -> Box<dyn KarbeatGenerator + Send + Sync> + Send + Sync + 'static,
    {
        let id = self.generator_id_counter;
        self.generator_id_counter += 1;

        let temp_plugin = factory();
        let parameter_specs = temp_plugin.get_parameter_specs();

        self.generators.insert(
            id,
            RegisteredGenerator {
                name: name.to_string(),
                factory: Box::new(factory),
                parameter_specs,
            },
        );
        id
    }

    /// Register a new effect plugin factory.
    /// Returns the assigned registry ID.
    pub fn register_effect<F>(&mut self, name: &str, factory: F) -> u32
    where
        F: Fn() -> Box<dyn KarbeatEffect + Send + Sync> + Send + Sync + 'static,
    {
        let id = self.effect_id_counter;
        self.effect_id_counter += 1;

        let temp_plugin = factory();
        let parameter_specs = temp_plugin.get_parameter_specs();

        self.effects.insert(
            id,
            RegisteredEffect {
                name: name.to_string(),
                factory: Box::new(factory),
                parameter_specs,
            },
        );
        id
    }

    // =========================================================================
    // ID-based creation (preferred)
    // =========================================================================

    /// Create a generator by its registry ID
    pub fn create_generator_by_id(
        &self,
        id: u32,
    ) -> Option<(Box<dyn KarbeatGenerator + Send + Sync>, String)> {
        self.generators.get(&id).map(|reg| {
            let plugin = (reg.factory)();
            (plugin, reg.name.clone())
        })
    }

    /// Create an effect by its registry ID
    pub fn create_effect_by_id(
        &self,
        id: u32,
    ) -> Option<(Box<dyn KarbeatEffect + Send + Sync>, String)> {
        self.effects.get(&id).map(|reg| {
            let plugin = (reg.factory)();
            (plugin, reg.name.clone())
        })
    }

    // =========================================================================
    // Cached Parameter Specs
    // =========================================================================

    /// Get cached parameter specs for a generator by registry ID
    pub fn get_generator_parameter_specs_by_id(&self, id: u32) -> Option<Vec<ParameterSpec>> {
        self.generators
            .get(&id)
            .map(|reg| reg.parameter_specs.clone())
    }

    /// Get cached parameter specs for an effect by registry ID
    pub fn get_effect_parameter_specs_by_id(&self, id: u32) -> Option<Vec<ParameterSpec>> {
        self.effects.get(&id).map(|reg| reg.parameter_specs.clone())
    }

    // =========================================================================
    // Name-based lookup (for backwards compatibility)
    // =========================================================================

    /// Get the registry ID for a generator by name
    pub fn get_generator_id_by_name(&self, name: &str) -> Option<u32> {
        self.generators
            .iter()
            .find(|(_, reg)| reg.name == name)
            .map(|(id, _)| *id)
    }

    /// Get the registry ID for an effect by name
    pub fn get_effect_id_by_name(&self, name: &str) -> Option<u32> {
        self.effects
            .iter()
            .find(|(_, reg)| reg.name == name)
            .map(|(id, _)| *id)
    }

    /// Create an instance of a generator by name (backwards compatibility).
    /// Returns the plugin instance and its name.
    pub fn create_generator(&self, name: &str) -> Option<Box<dyn KarbeatGenerator + Send + Sync>> {
        self.get_generator_id_by_name(name)
            .and_then(|id| self.create_generator_by_id(id))
            .map(|(plugin, _)| plugin)
    }

    /// Create an instance of an effect by name (backwards compatibility).
    pub fn create_effect(&self, name: &str) -> Option<Box<dyn KarbeatEffect + Send + Sync>> {
        self.get_effect_id_by_name(name)
            .and_then(|id| self.create_effect_by_id(id))
            .map(|(plugin, _)| plugin)
    }

    // =========================================================================
    // Listing plugins (for UI)
    // =========================================================================

    /// Get list of all available generator names (for UI) - backwards compatible
    pub fn list_generators(&self) -> Vec<String> {
        self.generators
            .values()
            .map(|reg| reg.name.clone())
            .collect()
    }

    /// Get list of all available effect names (for UI) - backwards compatible
    pub fn list_effects(&self) -> Vec<String> {
        self.effects.values().map(|reg| reg.name.clone()).collect()
    }

    /// Get list of all available generators with their IDs (for UI)
    pub fn list_generators_with_ids(&self) -> Vec<PluginInfo> {
        self.generators
            .iter()
            .map(|(id, reg)| PluginInfo {
                id: *id,
                name: reg.name.clone(),
            })
            .collect()
    }

    /// Get list of all available effects with their IDs (for UI)
    pub fn list_effects_with_ids(&self) -> Vec<PluginInfo> {
        self.effects
            .iter()
            .map(|(id, reg)| PluginInfo {
                id: *id,
                name: reg.name.clone(),
            })
            .collect()
    }

    /// Get a generator's name by its ID
    pub fn get_generator_name(&self, id: u32) -> Option<String> {
        self.generators.get(&id).map(|reg| reg.name.clone())
    }

    /// Get an effect's name by its ID
    pub fn get_effect_name(&self, id: u32) -> Option<String> {
        self.effects.get(&id).map(|reg| reg.name.clone())
    }
}
