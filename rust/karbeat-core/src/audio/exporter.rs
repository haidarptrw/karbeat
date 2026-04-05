use indexmap::IndexMap;
use karbeat_plugin_api::traits::{ KarbeatEffect, KarbeatGenerator };
use rtrb::RingBuffer;
use thiserror::Error;

use crate::{
    audio::{
        engine::AudioEngine,
        render_state::AudioRenderState,
        writer::{ AudioFormatBuilder, AudioWriter, BitPerSample },
    },
    commands::AudioCommand,
    context::ctx,
    core::project::{ ApplicationState, GeneratorId, TrackId, mixer::{ BusId, EffectId } },
};

#[derive(Debug, Clone, Error)]
#[error("Audio export failed ({error_source}): {message}")]
pub struct AudioExportError {
    pub error_source: String,
    pub message: String,
}

impl AudioExportError {
    pub fn new(source: &str, message: impl Into<String>) -> Self {
        Self {
            error_source: source.to_string(),
            message: message.into(),
        }
    }
}

pub fn export_project(
    app_state: &ApplicationState,
    output_path: &str,
    sample_rate: u32,
    bit_per_sample: BitPerSample,
    mut writer: impl AudioWriter
) -> Result<(), AudioExportError> {
    log::info!("Starting offline render to: {}", output_path);

    let channels = 2; // Stereo
    let block_size = 4096; // Faster offline rendering

    let _audio_format = AudioFormatBuilder::default()
        .channels(channels)
        .bit_per_sample(bit_per_sample)
        .sample_rate(sample_rate)
        .build()
        .map_err(|e| AudioExportError::new("Format", format!("Builder error: {}", e)))?;

    // Create a static snapshot of the Render State
    let render_state = AudioRenderState::from(app_state);

    // Set up Dummy Communication Channels
    let (mut _state_in, state_out) = triple_buffer::TripleBuffer::new(&render_state).split();
    let (mut cmd_producer, cmd_consumer) = RingBuffer::<AudioCommand>::new(1024);
    let (pos_producer, mut _pos_consumer) = RingBuffer::new(1024);
    let (feedback_producer, mut _feedback_consumer) = RingBuffer::new(1024);

    // Instantiate the Headless Audio Engine
    let mut offline_engine = AudioEngine::new(
        state_out,
        cmd_consumer,
        pos_producer,
        feedback_producer,
        sample_rate,
        channels as u16,
        app_state.transport.bpm,
        render_state.clone()
    );

    // Hydrate the Engine (Load fresh plugin clones)
    let registry = ctx().plugin_registry.read();

    let mut generators: IndexMap<
        GeneratorId,
        Box<dyn KarbeatGenerator + Send + Sync>
    > = IndexMap::new();
    let mut track_effects: IndexMap<
        TrackId,
        IndexMap<EffectId, Box<dyn KarbeatEffect + Send + Sync>>
    > = IndexMap::new();
    let mut bus_effects: IndexMap<
        BusId,
        IndexMap<EffectId, Box<dyn KarbeatEffect + Send + Sync>>
    > = IndexMap::new();
    let mut master_effects: IndexMap<
        EffectId,
        Box<dyn KarbeatEffect + Send + Sync>
    > = IndexMap::new();

    // Instantiate Generators
    for (gen_id, gen_arc) in &app_state.generator_pool {
        if
            let crate::core::project::GeneratorInstanceType::Plugin(plugin_instance) =
                &gen_arc.instance_type
        {
            if
                let Some((mut plugin, _)) = registry.create_generator_by_id(
                    plugin_instance.registry_id
                )
            {
                for (&param_id, &val) in &plugin_instance.parameters {
                    plugin.set_parameter(param_id, val);
                }
                generators.insert(*gen_id, plugin);
            }
        }
    }

    // Instantiate Track Effects
    for (track_id, channel) in &app_state.mixer.channels {
        let mut track_chain = IndexMap::new();
        for effect in &channel.effects {
            if
                let Some((mut plugin, _)) = registry.create_effect_by_id(
                    effect.instance.registry_id
                )
            {
                for (&param_id, &val) in &effect.instance.parameters {
                    plugin.set_parameter(param_id, val);
                }
                track_chain.insert(effect.id, plugin);
            }
        }
        if !track_chain.is_empty() {
            track_effects.insert(*track_id, track_chain);
        }
    }

    // Instantiate Bus Effects
    for (bus_id, bus) in &app_state.mixer.buses {
        let mut bus_chain = IndexMap::new();
        for effect in &bus.channel.effects {
            if
                let Some((mut plugin, _)) = registry.create_effect_by_id(
                    effect.instance.registry_id
                )
            {
                for (&param_id, &val) in &effect.instance.parameters {
                    plugin.set_parameter(param_id, val);
                }
                bus_chain.insert(effect.id, plugin);
            }
        }
        if !bus_chain.is_empty() {
            bus_effects.insert(*bus_id, bus_chain);
        }
    }

    // Instantiate Master Effects
    for effect in &app_state.mixer.master_bus.effects {
        if let Some((mut plugin, _)) = registry.create_effect_by_id(effect.instance.registry_id) {
            for (&param_id, &val) in &effect.instance.parameters {
                plugin.set_parameter(param_id, val);
            }
            master_effects.insert(effect.id, plugin);
        }
    }

    // Send Setup Commands to the Engine
    cmd_producer
        .push(AudioCommand::PreparePlugin {
            generators,
            track_effects,
            bus_effects,
            master_effects,
        })
        .map_err(|_| AudioExportError::new("Engine", "Failed to send PreparePlugin command"))?;

    cmd_producer
        .push(AudioCommand::SetPlaybackMode(crate::audio::engine::PlaybackMode::Song))
        .map_err(|_| AudioExportError::new("Engine", "Command queue full"))?;
    cmd_producer
        .push(AudioCommand::SetPlayhead(0))
        .map_err(|_| AudioExportError::new("Engine", "Command queue full"))?;
    cmd_producer
        .push(AudioCommand::SetPlaying(true))
        .map_err(|_| AudioExportError::new("Engine", "Command queue full"))?;

    // Determine exact render length
    let tail_seconds = 3.0; // Wait 3 seconds after the last clip for reverb/delays to fade
    let total_samples =
        render_state.graph.max_sample_index + (((sample_rate as f32) * tail_seconds) as u32);
    let mut processed_samples: u32 = 0;

    let mut mix_buffer = vec![0.0; block_size * channels as usize];

    // The "Faster-Than-Realtime" Loop
    while processed_samples < total_samples {
        let remaining = total_samples - processed_samples;
        let frames_to_process = std::cmp::min(block_size as u32, remaining) as usize;
        let samples_to_process = frames_to_process * (channels as usize);

        // Process the exact slice needed
        let active_slice = &mut mix_buffer[..samples_to_process];
        offline_engine.process(active_slice);

        // Delegate encoding and writing entirely to the generic writer interface
        writer
            .write(active_slice)
            .map_err(|e| AudioExportError::new("Writer", format!("Write error: {}", e)))?;

        // Keep the position/feedback queues from filling up and blocking
        while let Ok(_) = _pos_consumer.pop() {}
        while let Ok(_) = _feedback_consumer.pop() {}

        processed_samples += frames_to_process as u32;
    }

    writer
        .finalize()
        .map_err(|e| AudioExportError::new("Writer", format!("Finalize error: {}", e)))?;

    log::info!("Offline render successfully completed!");
    Ok(())
}
