// src/core/plugin/mod.rs

use std::{any::Any, collections::HashMap, fmt::Debug};

/// Trait that indicates an Effect plugin
/// Requires Send + Sync so it can be moved between UI and Audio threads
pub trait KarbeatEffect: Send + Sync {
    /// Returns the unique name of the plugin
    fn name(&self) -> &str;

    /// Prepare the plugin for playback (allocate buffers, set sample rate)
    fn prepare(&mut self, sample_rate: f32, max_buffer_size: usize);

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
    fn default_parameters(&self) -> HashMap<u32, f32>;

    // Helper for downcasting if you need concrete access later
    fn as_any(&self) -> &dyn Any;
}

pub trait KarbeatGenerator: Send + Sync {
    fn name(&self) -> &str;

    fn prepare(&mut self, sample_rate: f32, max_buffer_size: usize);

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
    fn default_parameters(&self) -> HashMap<u32, f32>;
    
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
    Generator(Box<dyn KarbeatGenerator + Send + Sync>)
}

impl KarbeatPlugin {
    /// Helper to run the process loop regardless of type
    pub fn process_audio(&mut self, buffer: &mut [f32], events: &[MidiEvent]) {
        match self {
            KarbeatPlugin::Effect(e) => {
                // Effects ignore MIDI usually, just process audio
                e.process(buffer);
            },
            KarbeatPlugin::Generator(g) => {
                // Generators overwrite the buffer with new sound
                // (Or add to it if you implement mixing logic inside)
                g.process(buffer, events);
            }
        }
    }

    pub fn default_parameters(&self) -> std::collections::HashMap<u32, f32> {
        match self {
            KarbeatPlugin::Effect(e) => e.default_parameters(),
            KarbeatPlugin::Generator(g) => g.default_parameters(),
        }
    }
}

// ============================================================================
// DEBUG IMPLEMENTATIONS
// ============================================================================

impl Debug for KarbeatPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KarbeatPlugin::Effect(e) => e.fmt(f),
            KarbeatPlugin::Generator(g) => g.fmt(f),
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