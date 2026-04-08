use std::{any::Any, fmt::Debug};

use indexmap::IndexMap;

use karbeat_plugin_types::PluginParameter;
use serde_json::Value;

/// Trait that indicates an Effect plugin
/// Requires Send + Sync so it can be moved between UI and Audio threads
pub trait KarbeatEffect: Send + Sync {
    /// Returns the unique name of the plugin
    fn name(&self) -> &str;

    /// Prepare the plugin for playback (allocate buffers, set sample rate and number of channels)
    fn prepare(&mut self, sample_rate: f32, channels: usize, max_buffer_size: usize);

    /// Reset internal state (clear delay lines, reverb tails, etc.)
    /// Called when playback stops or seeks.
    fn reset(&mut self);

    /// Process a block of audio.
    /// Typically modifies the buffer in-place.
    ///
    /// * `buffer` - Interleaved stereo buffer [L, R, L, R...]
    fn process(&mut self, buffer: &mut [f32]);

    /// Set a parameter value (0.0 to 1.0)
    fn set_parameter(&mut self, id: u32, value: f32);

    /// Get a parameter value
    fn get_parameter(&self, id: u32) -> f32;

    /// Get the default values for all parameters supported by this plugin
    fn default_parameters(&self) -> IndexMap<u32, f32>;

    /// Get parameter specifications for UI generation
    fn get_parameter_specs(&self) -> Vec<PluginParameter>;

    /// Execute custom command if provided by implementer
    fn execute_custom_command(&mut self, _command: &str, _payload: &Value) -> Option<Value> {
        None
    }

    // Helper for downcasting if you need concrete access later
    fn as_any(&self) -> &dyn Any;
}

pub trait KarbeatGenerator: Send + Sync {
    /// Get the name of the generator
    fn name(&self) -> &str;

    /// Prepare the plugin for playback (allocate buffers, set sample rate and number of channels)
    fn prepare(&mut self, sample_rate: f32, channels: usize, max_buffer_size: usize);

    /// Reset the parameters of the plugin
    fn reset(&mut self);

    /// Process a block of audio.
    /// Unlike effects, this generates audio into an empty (or zeroed) buffer.
    ///
    /// * `output_buffer` - Interleaved stereo buffer to write to.
    /// * `midi_events` - List of events (Note On/Off) for this specific buffer block.
    fn process(&mut self, output_buffer: &mut [f32], midi_events: &[MidiEvent]);

    fn set_parameter(&mut self, id: u32, value: f32);
    fn get_parameter(&self, id: u32) -> f32;

    /// Get the default values for all parameters supported by this plugin
    fn default_parameters(&self) -> IndexMap<u32, f32>;

    /// Get parameter specifications for UI generation
    fn get_parameter_specs(&self) -> Vec<PluginParameter>;

    /// Execute custom command if provided by implementer
    fn execute_custom_command(&mut self, _command: &str, _payload: &Value) -> Option<Value> {
        None
    }

    fn as_any(&self) -> &dyn Any;
}

// Simple struct to pass midi to the generator process loop
pub struct MidiEvent {
    /// Offset in samples within the current buffer (0 to buffer_size)
    pub sample_offset: usize,
    pub data: MidiMessage,
}

pub enum MidiMessage {
    NoteOn { key: u8, velocity: u8 },
    NoteOff { key: u8 },
    ControlChange { controller: u8, value: u8 },
}

pub enum KarbeatPlugin {
    /// Effect plugin (DSP of audio properties)
    Effect(Box<dyn KarbeatEffect + Send + Sync>),
    /// Generator plugin (Wave generator or synth)
    Generator(Box<dyn KarbeatGenerator + Send + Sync>),
}

impl KarbeatPlugin {
    /// Helper to run the process loop regardless of type
    pub fn process_audio(&mut self, buffer: &mut [f32], events: &[MidiEvent]) {
        match self {
            KarbeatPlugin::Effect(e) => {
                // Effects ignore MIDI usually, just process audio
                e.process(buffer);
            }
            KarbeatPlugin::Generator(g) => {
                // Generators overwrite the buffer with new sound
                // (Or add to it if you implement mixing logic inside)
                g.process(buffer, events);
            }
        }
    }

    pub fn default_parameters(&self) -> IndexMap<u32, f32> {
        match self {
            KarbeatPlugin::Effect(e) => e.default_parameters(),
            KarbeatPlugin::Generator(g) => g.default_parameters(),
        }
    }

    /// Set a parameter on the plugin (works for both Effects and Generators)
    pub fn set_parameter(&mut self, id: u32, value: f32) {
        match self {
            KarbeatPlugin::Effect(e) => e.set_parameter(id, value),
            KarbeatPlugin::Generator(g) => g.set_parameter(id, value),
        }
    }

    /// Get a parameter from the plugin
    pub fn get_parameter(&self, id: u32) -> f32 {
        match self {
            KarbeatPlugin::Effect(e) => e.get_parameter(id),
            KarbeatPlugin::Generator(g) => g.get_parameter(id),
        }
    }

    /// Get parameter specifications for UI generation
    pub fn get_parameter_specs(&self) -> Vec<PluginParameter> {
        match self {
            KarbeatPlugin::Effect(e) => e.get_parameter_specs(),
            KarbeatPlugin::Generator(g) => g.get_parameter_specs(),
        }
    }

    /// Execute a custom command on the plugin (works for both Effects and Generators)
    pub fn execute_custom_command(&mut self, command: &str, payload: &Value) -> Option<Value> {
        match self {
            KarbeatPlugin::Effect(e) => e.execute_custom_command(command, payload),
            KarbeatPlugin::Generator(g) => g.execute_custom_command(command, payload),
        }
    }
}

// We implement Debug manually for the Trait Objects.
// We use the `name()` method (which every plugin must implement) to identify it.

impl Debug for dyn KarbeatEffect + Send + Sync {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KarbeatEffect")
            .field("name", &self.name())
            .finish()
    }
}

impl Debug for dyn KarbeatGenerator + Send + Sync {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KarbeatGenerator")
            .field("name", &self.name())
            .finish()
    }
}
