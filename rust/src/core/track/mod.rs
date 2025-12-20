pub mod midi;
// src/core/track/mod.rs

use std::{collections::HashMap, sync::{Arc, Mutex, RwLock}};

use crate::{core::{plugin::KarbeatPlugin, project::{ApplicationState, GeneratorInstance, GeneratorInstanceType, KarbeatTrack, PluginInstance, TrackType}}, plugin::{generator::factory::KarbeatGeneratorFactory, registry::PLUGIN_REGISTRY}};

pub mod audio_waveform;

impl ApplicationState {
    pub fn add_new_track(&mut self, track_type: TrackType) {
        // increment track_counter
        self.track_counter += 1;
        let new_track_id = self.track_counter;
        let new_track = KarbeatTrack {
            track_type,
            id: new_track_id,
            ..Default::default()
        };
        self.tracks.insert(new_track_id, Arc::new(new_track));

    }

    pub fn add_new_midi_track_with_generator(&mut self, generator_name: &str) -> anyhow::Result<()> {
        self.generator_counter += 1;
        let gen_id = self.generator_counter;

        self.track_counter += 1;
        let track_id = self.track_counter;
        let plugin_runtime = {
            let registry = PLUGIN_REGISTRY.read().expect("Failed to lock registry");
            
            if let Some(generator_box) = registry.create_generator(&generator_name) {
                // Wrap the Box<dyn Generator> into our Runtime Enum and Mutex
                Arc::new(Mutex::new(KarbeatPlugin::Generator(generator_box)))
            } else {
                let message = format!("Generator '{}' not found in registry", generator_name);
                log::error!("{}", message);
                // Decrement counters if failed to prevent gaps/orphans (optional)
                self.generator_counter -= 1;
                self.track_counter -= 1;
                return Err(anyhow::anyhow!("{}", message));
            }
        };

        let default_params = if let Ok(guard) = plugin_runtime.lock() {
            guard.default_parameters()
        } else {
            HashMap::new()
        };

        let plugin_instance = PluginInstance {
            name: generator_name.to_string(),
            internal_type: generator_name.to_string(),
            bypass: false,
            parameters: default_params,
            instance: Some(plugin_runtime),
        };

        let generator = GeneratorInstance {
            id: gen_id,
            effects: Arc::new(Vec::new()),
            instance_type: GeneratorInstanceType::Plugin(plugin_instance),
        };
        self.generator_pool.insert(gen_id, Arc::new(RwLock::new(generator.clone())));

        let new_track = KarbeatTrack {
            track_type: TrackType::Midi,
            id: track_id,
            name: format!("{} {}", generator_name, track_id),
            color: "#FF8A65".to_string(),
            generator: Some(generator),
            ..Default::default()
        };
        
        self.tracks.insert(track_id, Arc::new(new_track));

        log::info!("New MIDI track with generator {} is successfully created", generator_name);
        Ok(())
    }
}