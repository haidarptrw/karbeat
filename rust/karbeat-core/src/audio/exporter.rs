use hound::{SampleFormat, WavSpec, WavWriter};
use indexmap::IndexMap;
use karbeat_plugin_api::traits::{KarbeatEffect, KarbeatGenerator};
use rtrb::RingBuffer;

use crate::{
    audio::{engine::AudioEngine, render_state::AudioRenderState},
    commands::AudioCommand,
    context::ctx, // Assuming you need this to access the plugin registry
    core::project::{ApplicationState, GeneratorId, TrackId, mixer::{BusId, EffectId}},
};

pub fn export_project_to_wav(
    app_state: &ApplicationState,
    output_path: &str,
    sample_rate: u32,
) -> Result<(), String> {
    log::info!("Starting offline render to: {}", output_path);

    let channels = 2;
    let block_size = 4096; // Much larger block size for faster offline rendering

    // 1. Create a static snapshot of the Render State
    let render_state = AudioRenderState::from(app_state);

    // 2. Set up Dummy Communication Channels
    let (mut _state_in, state_out) = triple_buffer::TripleBuffer::new(&render_state).split();
    let (mut cmd_producer, cmd_consumer) = RingBuffer::<AudioCommand>::new(1024);
    let (pos_producer, mut _pos_consumer) = RingBuffer::new(1024);
    let (feedback_producer, mut _feedback_consumer) = RingBuffer::new(1024);

    // 3. Instantiate the Headless Audio Engine
    let mut offline_engine = AudioEngine::new(
        state_out,
        cmd_consumer,
        pos_producer,
        feedback_producer,
        sample_rate,
        channels as u16,
        app_state.transport.bpm,
        render_state.clone(),
    );

    // 4. Hydrate the Engine (Load fresh plugin clones)
    // You MUST instantiate fresh copies of all plugins from the registry so they have independent state.
    let registry = ctx().plugin_registry.read();

    let mut generators: IndexMap<GeneratorId, Box<dyn KarbeatGenerator + Send>> = IndexMap::new();
    let mut track_effects: IndexMap<TrackId, IndexMap<EffectId, Box<dyn KarbeatEffect + Send>>> =
        IndexMap::new();
    let mut bus_effects: IndexMap<BusId, IndexMap<EffectId, Box<dyn KarbeatEffect + Send>>> =
        IndexMap::new();
    let mut master_effects: IndexMap<EffectId, Box<dyn KarbeatEffect + Send>> = IndexMap::new();

    // Iterate through app_state.generator_pool, track effects, etc., and instantiate them.
    // Example for generators:
    for (gen_id, gen_arc) in &app_state.generator_pool {
        if let crate::core::project::GeneratorInstanceType::Plugin(plugin_instance) =
            &gen_arc.instance_type
        {
            if let Some((mut plugin, _)) =
                registry.create_generator_by_id(plugin_instance.registry_id)
            {
                // Apply saved parameters
                for (&param_id, &val) in &plugin_instance.parameters {
                    plugin.set_parameter(param_id, val);
                }
                generators.insert(*gen_id, plugin);
            }
        }
    }
    // (You will need to repeat the above block for track_effects, bus_effects, and master_effects
    // by iterating through app_state.mixer.channels, buses, and master_bus).

    // TODO: DO the same for track_effects, bus_effects, and master_effects

    // 5. Send Setup Commands to the Engine
    cmd_producer
        .push(AudioCommand::PreparePlugin {
            generators,
            track_effects,
            bus_effects,
            master_effects,
        })
        .map_err(|_| "Failed to send PreparePlugin command")?;

    cmd_producer
        .push(AudioCommand::SetPlaybackMode(
            crate::audio::engine::PlaybackMode::Song,
        ))
        .map_err(|_| "Queue full")?;
    cmd_producer
        .push(AudioCommand::SetPlayhead(0))
        .map_err(|_| "Queue full")?;
    cmd_producer
        .push(AudioCommand::SetPlaying(true))
        .map_err(|_| "Queue full")?;

    // 6. Setup the WAV Writer
    let spec = WavSpec {
        channels: channels as u16,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut writer = WavWriter::create(output_path, spec)
        .map_err(|e| format!("Failed to create WAV file: {}", e))?;

    // 7. Determine exact render length (with Reverb Tail)
    let tail_seconds = 3.0; // Wait 3 seconds after the last clip for reverb/delays to fade
    let total_samples =
        render_state.graph.max_sample_index + (sample_rate as f32 * tail_seconds) as u32;
    let mut processed_samples: u32 = 0;

    let mut mix_buffer = vec![0.0; block_size * channels];

    // 8. The "Faster-Than-Realtime" Loop
    while processed_samples < total_samples {
        let remaining = total_samples - processed_samples;
        let frames_to_process = std::cmp::min(block_size as u32, remaining) as usize;
        let samples_to_process = frames_to_process * channels;

        // Process the exact slice needed
        let active_slice = &mut mix_buffer[..samples_to_process];
        offline_engine.process(active_slice);

        // Write to WAV file
        for &sample in active_slice.iter() {
            // Hard clip to prevent file corruption
            let clipped = sample.clamp(-1.0, 1.0);
            writer
                .write_sample(clipped)
                .map_err(|e| format!("WAV write error: {}", e))?;
        }

        // Keep the position/feedback queues from filling up and blocking
        while let Ok(_) = _pos_consumer.pop() {}
        while let Ok(_) = _feedback_consumer.pop() {}

        processed_samples += frames_to_process as u32;
    }

    writer
        .finalize()
        .map_err(|e| format!("Failed to finalize WAV: {}", e))?;

    log::info!("Offline render successfully completed!");
    Ok(())
}
