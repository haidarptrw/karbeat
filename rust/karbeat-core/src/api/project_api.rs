use std::path::Path;
use std::sync::Arc;
use std::time::{ Duration, Instant };

use indexmap::IndexMap;

use crate::audio::event::PluginTarget;
use crate::audio::exporter::{
    export_project as export_project_internal,
    AudioExportError,
    TailHandling,
};

use crate::audio::writer::{ AudioFormat, BitPerSample, create_writer };
use crate::commands::{ AudioCommand, AudioFeedback };
use crate::context::utils::{ broadcast_state_change, send_audio_command };
use crate::core::file_manager::project_loader::{ load_karbeat_project, save_karbeat_project };
use crate::core::project::{
    generator::GeneratorInstance,
    transport::TransportState,
    ProjectMetadata,
};
use crate::core::project::{ ApplicationState, GeneratorInstanceType };
use crate::lock::{ get_app_read, get_app_write };

pub fn get_project_metadata<T, F>(mapper: F) -> anyhow::Result<T> where F: Fn(&ProjectMetadata) -> T {
    let app = get_app_read();
    Ok(mapper(&app.metadata))
}

pub fn get_transport_state<T, F>(mapper: F) -> anyhow::Result<T> where F: Fn(&TransportState) -> T {
    let app = get_app_read();
    Ok(mapper(&app.transport))
}

pub fn get_generator_list<C, U, M>(mapper: M) -> anyhow::Result<C>
    where M: Fn(u32, &GeneratorInstance) -> U, C: FromIterator<U>
{
    let app = get_app_read();
    Ok(
        app.generator_pool
            .iter()
            .map(|(&id, gen)| mapper(id.to_u32(), gen.as_ref()))
            .collect()
    )
}

pub fn get_max_sample_index() -> anyhow::Result<u32> {
    let app = get_app_read();
    Ok(app.max_sample_index)
}

pub fn save_project(path_name: &str) -> anyhow::Result<()> {
    log::info!("Initiating project save. Syncing plugin states from audio engine...");

    let mut pending_requests = Vec::new();
    let mut expected_responses = 0;

    // Gather all targets we need to query (using a tight read lock)
    {
        let app = get_app_read();

        for (&gen_id, _) in &app.generator_pool {
            pending_requests.push(PluginTarget::Generator(gen_id));
            expected_responses += 1;
        }
        for (&track_id, channel) in &app.mixer.channels {
            for effect in &channel.effects {
                pending_requests.push(PluginTarget::TrackEffect(track_id, effect.id));
                expected_responses += 1;
            }
        }
        for (&bus_id, bus) in &app.mixer.buses {
            for effect in &bus.channel.effects {
                pending_requests.push(PluginTarget::BusEffect(bus_id, effect.id));
                expected_responses += 1;
            }
        }
        for effect in &app.mixer.master_bus.effects {
            pending_requests.push(PluginTarget::MasterEffect(effect.id));
            expected_responses += 1;
        }
    }

    // Fire off all Query Commands to the audio thread
    for (i, target) in pending_requests.into_iter().enumerate() {
        send_audio_command(AudioCommand::QueryPluginState {
            target,
            request_id: i as u32,
        });
    }

    // Wait for the audio thread to return the snapshots
    let mut updated_states: Vec<(PluginTarget, Vec<u8>)> = Vec::with_capacity(expected_responses);
    let timeout = Duration::from_secs(2);
    let start_time = Instant::now();

    let mut feedback_rx = crate::context::ctx().feedback_consumer.lock();

    while updated_states.len() < expected_responses {
        if start_time.elapsed() > timeout {
            log::warn!(
                "Timed out waiting for plugin states from audio engine. Saving with available state."
            );
            break;
        }

       if let Some(consumer) = feedback_rx.as_mut() {
            while let Ok(feedback) = consumer.pop() {
                if let AudioFeedback::PluginStateSnapshot { target, state, .. } = feedback {
                    updated_states.push((target, state));
                } else {
                    // If it's a different feedback type, you might want to process it or push it back
                }
            }
        } else {
            // The Option is None. We can't read feedback, so we must abort the wait to prevent a deadlock.
            log::error!("Feedback consumer is None. Aborting state synchronization.");
            break; 
        }

        // Prevent pegging the CPU while waiting
        std::thread::sleep(Duration::from_millis(2));
    }

    // Lock the app state for writing, apply the binary blobs, and save!
    {
        let mut app = get_app_write();

        for (target, state_blob) in updated_states {
            match target {
                PluginTarget::Generator(gen_id) => {
                    if let Some(gen_arc) = app.generator_pool.get_mut(&gen_id) {
                        let gen_mut = Arc::make_mut(gen_arc);
                        if
                            let GeneratorInstanceType::Plugin(plugin_instance) =
                                &mut gen_mut.instance_type
                        {
                            plugin_instance.plugin_state = state_blob;
                        }
                    }
                }
                PluginTarget::TrackEffect(track_id, effect_id) => {
                    if let Some(channel) = app.mixer.channels.get_mut(&track_id) {
                        let ch_mut = Arc::make_mut(channel);
                        if let Some(effect) = ch_mut.effects.iter_mut().find(|e| e.id == effect_id) {
                            let instance_mut = Arc::make_mut(&mut effect.instance);
                            instance_mut.plugin_state = state_blob;
                        }
                    }
                }
                PluginTarget::BusEffect(bus_id, effect_id) => {
                    if let Some(bus) = app.mixer.buses.get_mut(&bus_id) {
                        let bus_mut = Arc::make_mut(bus);
                        if
                            let Some(effect) = bus_mut.channel.effects
                                .iter_mut()
                                .find(|e| e.id == effect_id)
                        {
                            let instance_mut = Arc::make_mut(&mut effect.instance);
                            instance_mut.plugin_state = state_blob;
                        }
                    }
                }
                PluginTarget::MasterEffect(effect_id) => {
                    let master_bus_mut = Arc::make_mut(&mut app.mixer.master_bus);
                    if
                        let Some(effect) = master_bus_mut.effects
                            .iter_mut()
                            .find(|e| e.id == effect_id)
                    {
                        let instance_mut = Arc::make_mut(&mut effect.instance);
                        instance_mut.plugin_state = state_blob;
                    }
                }
            }
        }

        // Finally write the fully synchronized AppState to disk
        save_karbeat_project(Path::new(path_name), &app)?;
    }

    log::info!("Project saved successfully.");
    Ok(())
}

pub fn load_project<T, F>(path_name: &str, mapper: F) -> anyhow::Result<T>
    where F: FnOnce(&ApplicationState) -> T
{
    // 1. Load the project from disk
    let loaded_app = load_karbeat_project(Path::new(path_name))?;

    // Extract the BPM before we move the loaded app into the global lock
    let bpm = loaded_app.transport.bpm;

    // 2. Halt the audio engine immediately to prevent reading partially overwritten state
    send_audio_command(AudioCommand::StopAndReset);

    // 3. Lock, mutate the global state, and map to the UI DTO
    let mapped_ui_state = {
        let mut app = get_app_write();
        *app = loaded_app;
        mapper(&app)
    };

    // 4. Fire all necessary sync and loading events to the engine/UI
    broadcast_state_change();
    send_audio_command(AudioCommand::SetBPM(bpm));
    let _ = hydrate_live_audio_engine();

    Ok(mapped_ui_state)
}

pub fn export_project<F>(
    output_path: &String,
    sample_rate: u32,
    bit_per_sample: BitPerSample,
    channels: u32,
    tail_handling: TailHandling,
    progress_callback: F
) -> anyhow::Result<()>
    where F: FnMut(f32) -> bool
{
    // Use read lock since the internal function only requires an immutable &ApplicationState
    let app_state = get_app_read();
    let path = Path::new(output_path);

    // 1. Construct the AudioFormat object once
    let audio_format = AudioFormat {
        bit_per_sample,
        sample_rate,
        channels: channels as u16,
    };

    let writer = create_writer(path, audio_format).map_err(|e| {
        anyhow::anyhow!(
            "{}",
            AudioExportError::new("WriterInit", format!("Failed to create writer: {}", e))
        )
    })?;

    export_project_internal(
        &app_state,
        output_path,
        audio_format,
        writer,
        tail_handling,
        progress_callback
    ).map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(())
}

/// Create a blank, new, default project.
pub fn new_blank_project() -> ApplicationState {
    let app = {
        let mut app = get_app_write();
        (&mut app).new_blank_project();
        app.clone()
    };

    // prepare plugin with this new state
    broadcast_state_change();
    send_audio_command(AudioCommand::SetBPM(app.transport.bpm));
    let _ = hydrate_live_audio_engine();

    app
}

/// Safely instantiates all DSP plugins from the current state,
/// explicitly applies their saved parameters, and sends them to the audio thread.
pub fn hydrate_live_audio_engine() -> anyhow::Result<()> {
    let app = crate::lock::get_app_read();
    let registry = crate::context::ctx().plugin_registry.read();

    let mut generators = IndexMap::new();
    let mut track_effects = IndexMap::new();
    let mut bus_effects = IndexMap::new();
    let mut master_effects = IndexMap::new();

    // 1. Hydrate Generators
    for (gen_id, gen_arc) in &app.generator_pool {
        if let GeneratorInstanceType::Plugin(plugin_instance) = &gen_arc.instance_type {
            if
                let Some((mut plugin, _)) = registry.create_generator_by_id(
                    plugin_instance.registry_id
                )
            {
                // Pass the binary state to the plugin
                if !plugin_instance.plugin_state.is_empty() {
                    plugin.set_state(&plugin_instance.plugin_state);
                } else {
                    // Fallback for older project files that only saved parameters
                    for (&param_id, &val) in &plugin_instance.parameters {
                        plugin.set_parameter(param_id, val);
                    }
                }
                generators.insert(*gen_id, plugin);
            }
        }
    }

    // 2. Hydrate Track Effects
    for (track_id, channel_arc) in &app.mixer.channels {
        let mut track_chain = IndexMap::new();
        for effect in &channel_arc.effects {
            if
                let Some((mut plugin, _)) = registry.create_effect_by_id(
                    effect.instance.registry_id
                )
            {
                if !effect.instance.plugin_state.is_empty() {
                    plugin.set_state(&effect.instance.plugin_state);
                }
                {
                    for (&param_id, &val) in &effect.instance.parameters {
                        plugin.set_parameter(param_id, val);
                    }
                }
                track_chain.insert(effect.id, plugin);
            }
        }
        if !track_chain.is_empty() {
            track_effects.insert(*track_id, track_chain);
        }
    }

    // 3. Hydrate Bus Effects
    for (bus_id, bus_arc) in &app.mixer.buses {
        let mut bus_chain = IndexMap::new();
        for effect in &bus_arc.channel.effects {
            if
                let Some((mut plugin, _)) = registry.create_effect_by_id(
                    effect.instance.registry_id
                )
            {
                if !effect.instance.plugin_state.is_empty() {
                    plugin.set_state(&effect.instance.plugin_state);
                }
                {
                    for (&param_id, &val) in &effect.instance.parameters {
                        plugin.set_parameter(param_id, val);
                    }
                }
                bus_chain.insert(effect.id, plugin);
            }
        }
        if !bus_chain.is_empty() {
            bus_effects.insert(*bus_id, bus_chain);
        }
    }

    // 4. Hydrate Master Effects
    for effect in &app.mixer.master_bus.effects {
        if let Some((mut plugin, _)) = registry.create_effect_by_id(effect.instance.registry_id) {
            if !effect.instance.plugin_state.is_empty() {
                plugin.set_state(&effect.instance.plugin_state);
            }
            {
                for (&param_id, &val) in &effect.instance.parameters {
                    plugin.set_parameter(param_id, val);
                }
            }
            master_effects.insert(effect.id, plugin);
        }
    }

    // Send the fully configured plugins to the Live Audio Engine
    send_audio_command(AudioCommand::PreparePlugin {
        generators,
        track_effects,
        bus_effects,
        master_effects,
    });

    Ok(())
}
