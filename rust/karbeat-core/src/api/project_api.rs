use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use indexmap::IndexMap;

use crate::audio::event::PluginTarget;
use crate::audio::exporter::{export_project as export_project_internal, TailHandling};

use crate::audio::writer::AudioExportConfig;
use crate::commands::{AudioCommand, AudioFeedback, MixerChannelSeed, MixerChannelTarget};
use crate::context::utils::{broadcast_full_graph, send_audio_command};
use crate::core::file_manager::project_loader::{load_daw_project, save_daw_project};
use crate::core::project::{
    generator::GeneratorInstance, transport::TransportState, ProjectMetadata,
};
use crate::core::project::{ApplicationState, GeneratorInstanceType};
use crate::lock::{get_app_read, get_app_write, get_plugin_registry_read};
pub fn get_project_metadata<T, F>(mapper: F) -> anyhow::Result<T>
where
    F: Fn(&ProjectMetadata) -> T,
{
    let app = get_app_read();
    Ok(mapper(&app.metadata))
}

pub fn get_transport_state<T, F>(mapper: F) -> anyhow::Result<T>
where
    F: Fn(&TransportState) -> T,
{
    let app = get_app_read();
    Ok(mapper(&app.transport))
}

pub fn get_generator_list<C, U, M>(mapper: M) -> anyhow::Result<C>
where
    M: Fn(u32, &GeneratorInstance) -> U,
    C: FromIterator<U>,
{
    let app = get_app_read();
    Ok(app
        .generator_pool
        .iter()
        .map(|(&id, gen)| mapper(id.to_u32(), gen.as_ref()))
        .collect())
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
    let mut mixer_channel_targets: Vec<MixerChannelTarget> = Vec::new();
    {
        let app = get_app_read();

        for (&gen_id, _) in &app.generator_pool {
            pending_requests.push(PluginTarget::Generator(gen_id));
            expected_responses += 1;
        }
        for (&track_id, channel) in &app.mixer.channels {
            for effect in &channel.channel.effects {
                pending_requests.push(PluginTarget::TrackEffect(track_id, effect.id));
                expected_responses += 1;
            }
            // Also query mixer channel DSP state
            mixer_channel_targets.push(MixerChannelTarget::Track(track_id));
        }
        for (&bus_id, bus) in &app.mixer.buses {
            for effect in &bus.channel.effects {
                pending_requests.push(PluginTarget::BusEffect(bus_id, effect.id));
                expected_responses += 1;
            }
            // Also query bus channel DSP state
            mixer_channel_targets.push(MixerChannelTarget::Bus(bus_id));
        }
        for effect in &app.mixer.master_bus.effects {
            pending_requests.push(PluginTarget::MasterEffect(effect.id));
            expected_responses += 1;
        } // Query master channel DSP state
        mixer_channel_targets.push(MixerChannelTarget::Master);
    }

    let expected_mixer_snapshots = mixer_channel_targets.len();

    // Fire off all Query Commands to the audio thread
    for (i, target) in pending_requests.into_iter().enumerate() {
        send_audio_command(AudioCommand::QueryPluginState {
            target,
            request_id: i as u32,
        });
    }
    for target in mixer_channel_targets {
        send_audio_command(AudioCommand::QueryMixerChannel { target });
    }

    // Wait for the audio thread to return the snapshots
    let mut updated_states: Vec<(PluginTarget, Vec<u8>)> = Vec::with_capacity(expected_responses);
    let mut mixer_snapshots: Vec<crate::commands::MixerChannelSnapshot> =
        Vec::with_capacity(expected_mixer_snapshots);
    let timeout = Duration::from_secs(2);
    let start_time = Instant::now();

    let mut feedback_rx = crate::context::ctx().feedback_consumer.lock();

    while updated_states.len() < expected_responses
        || mixer_snapshots.len() < expected_mixer_snapshots
    {
        if start_time.elapsed() > timeout {
            log::warn!(
                "Timed out waiting for plugin/mixer states from audio engine. Saving with available state."
            );
            break;
        }

        if let Some(consumer) = feedback_rx.as_mut() {
            while let Ok(feedback) = consumer.pop() {
                match feedback {
                    AudioFeedback::PluginStateSnapshot { target, state, .. } => {
                        updated_states.push((target, state));
                    }
                    AudioFeedback::MixerChannelSnapshot(snap) => {
                        mixer_snapshots.push(snap);
                    }
                    AudioFeedback::GeneratorParameterSnapshot(_snap) => {}
                    AudioFeedback::EffectParameterSnapshot(_snap) => {}
                    _ => {}
                }
            }
        } else {
            log::error!("Feedback consumer is None. Aborting state synchronization.");
            break;
        }

        std::thread::sleep(Duration::from_millis(2));
    }

    // Lock the app state for writing, apply the binary blobs, and save!
    {
        let mut app = get_app_write();

        let registry = get_plugin_registry_read();

        for (target, state_blob) in updated_states {
            match target {
                PluginTarget::Generator(gen_id) => {
                    if let Some(gen_arc) = app.generator_pool.get_mut(&gen_id) {
                        let gen_mut = Arc::make_mut(gen_arc);
                        if let GeneratorInstanceType::Plugin(plugin_instance) =
                            &mut gen_mut.instance_type
                        {
                            plugin_instance.plugin_state = state_blob;

                            let specs = if plugin_instance.registry_id > 0 {
                                registry.get_generator_parameter_specs_by_id(
                                    plugin_instance.registry_id,
                                )
                            } else {
                                registry
                                    .get_generator_id_by_name(&plugin_instance.name)
                                    .and_then(|id| registry.get_generator_parameter_specs_by_id(id))
                            };
                            if let Some(specs) = specs {
                                plugin_instance.parameter_specs = specs;
                            }
                        }
                    }
                }
                PluginTarget::TrackEffect(track_id, effect_id) => {
                    if let Some(channel) = app.mixer.channels.get_mut(&track_id) {
                        let ch_mut = Arc::make_mut(channel);
                        if let Some(effect) = ch_mut
                            .channel
                            .effects
                            .iter_mut()
                            .find(|e| e.id == effect_id)
                        {
                            let instance_mut = Arc::make_mut(&mut effect.instance);
                            instance_mut.plugin_state = state_blob;

                            let specs = if instance_mut.registry_id > 0 {
                                registry.get_effect_parameter_specs_by_id(instance_mut.registry_id)
                            } else {
                                registry
                                    .get_effect_id_by_name(&instance_mut.name)
                                    .and_then(|id| registry.get_effect_parameter_specs_by_id(id))
                            };
                            if let Some(specs) = specs {
                                instance_mut.parameter_specs = specs;
                            }
                        }
                    }
                }
                PluginTarget::BusEffect(bus_id, effect_id) => {
                    if let Some(bus) = app.mixer.buses.get_mut(&bus_id) {
                        let bus_mut = Arc::make_mut(bus);
                        if let Some(effect) = bus_mut
                            .channel
                            .effects
                            .iter_mut()
                            .find(|e| e.id == effect_id)
                        {
                            let instance_mut = Arc::make_mut(&mut effect.instance);
                            instance_mut.plugin_state = state_blob;

                            let specs = if instance_mut.registry_id > 0 {
                                registry.get_effect_parameter_specs_by_id(instance_mut.registry_id)
                            } else {
                                registry
                                    .get_effect_id_by_name(&instance_mut.name)
                                    .and_then(|id| registry.get_effect_parameter_specs_by_id(id))
                            };
                            if let Some(specs) = specs {
                                instance_mut.parameter_specs = specs;
                            }
                        }
                    }
                }
                PluginTarget::MasterEffect(effect_id) => {
                    let master_bus_mut = Arc::make_mut(&mut app.mixer.master_bus);
                    if let Some(effect) = master_bus_mut
                        .effects
                        .iter_mut()
                        .find(|e| e.id == effect_id)
                    {
                        let instance_mut = Arc::make_mut(&mut effect.instance);
                        instance_mut.plugin_state = state_blob;

                        let specs = if instance_mut.registry_id > 0 {
                            registry.get_effect_parameter_specs_by_id(instance_mut.registry_id)
                        } else {
                            registry
                                .get_effect_id_by_name(&instance_mut.name)
                                .and_then(|id| registry.get_effect_parameter_specs_by_id(id))
                        };
                        if let Some(specs) = specs {
                            instance_mut.parameter_specs = specs;
                        }
                    }
                }
            }
        }

        // Apply mixer channel DSP snapshots from the audio thread
        for snap in mixer_snapshots {
            match snap.target {
                MixerChannelTarget::Track(track_id) => {
                    if let Some(channel_arc) = app.mixer.channels.get_mut(&track_id) {
                        let ch = std::sync::Arc::make_mut(channel_arc);
                        ch.channel.volume.set_base(snap.volume);
                        ch.channel.pan.set_base(snap.pan);
                        ch.channel.mute = snap.mute;
                        ch.channel.solo = snap.solo;
                        ch.channel.inverted_phase = snap.inverted_phase;
                    }
                }
                MixerChannelTarget::Bus(bus_id) => {
                    if let Some(bus_arc) = app.mixer.buses.get_mut(&bus_id) {
                        let bus = std::sync::Arc::make_mut(bus_arc);
                        bus.channel.volume.set_base(snap.volume);
                        bus.channel.pan.set_base(snap.pan);
                        bus.channel.mute = snap.mute;
                        bus.channel.solo = snap.solo;
                        bus.channel.inverted_phase = snap.inverted_phase;
                    }
                }
                MixerChannelTarget::Master => {
                    let master = std::sync::Arc::make_mut(&mut app.mixer.master_bus);
                    master.volume.set_base(snap.volume);
                    master.pan.set_base(snap.pan);
                    master.mute = snap.mute;
                    master.solo = snap.solo;
                    master.inverted_phase = snap.inverted_phase;
                }
            }
        }

        // Finally write the fully synchronized AppState to disk
        save_daw_project(Path::new(path_name), &app)?;
    }

    log::info!("Project saved successfully.");
    Ok(())
}

pub fn load_project<T, F>(path_name: &str, mapper: F) -> anyhow::Result<T>
where
    F: FnOnce(&ApplicationState) -> T,
{
    // 1. Load the project from disk
    let loaded_app = load_daw_project(Path::new(path_name))?;

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
    broadcast_full_graph();
    send_audio_command(AudioCommand::SetBPM(bpm));
    let _ = hydrate_live_audio_engine();

    Ok(mapped_ui_state)
}

pub fn export_project<F>(
    output_path: &str,
    config: AudioExportConfig,
    tail_handling: TailHandling,
    progress_callback: F,
) -> anyhow::Result<()>
where
    F: FnMut(f32) -> bool,
{
    // Use read lock since the internal function only requires an immutable &ApplicationState
    let app_state = get_app_read();

    export_project_internal(
        &app_state,
        output_path,
        config,
        tail_handling,
        progress_callback,
    )
    .map_err(|e| anyhow::anyhow!("{}", e))?;

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
    broadcast_full_graph();
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
            if let Some((mut plugin, _)) =
                registry.create_generator_by_id(plugin_instance.registry_id)
            {
                // Pass the binary state to the plugin
                if !plugin_instance.plugin_state.is_empty() {
                    plugin.set_state(&plugin_instance.plugin_state);
                } else {
                    // Fallback for older project files that only saved parameters
                    for spec in &plugin_instance.parameter_specs {
                        plugin.set_parameter(spec.id, spec.value as f32);
                    }
                }
                generators.insert(*gen_id, plugin);
            }
        }
    }

    // 2. Hydrate Track Effects
    for (track_id, channel_arc) in &app.mixer.channels {
        let mut track_chain = IndexMap::new();
        for effect in &channel_arc.channel.effects {
            if let Some((mut plugin, _)) = registry.create_effect_by_id(effect.instance.registry_id)
            {
                if !effect.instance.plugin_state.is_empty() {
                    plugin.set_state(&effect.instance.plugin_state);
                }
                {
                    for spec in &effect.instance.parameter_specs {
                        plugin.set_parameter(spec.id, spec.value as f32);
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
            if let Some((mut plugin, _)) = registry.create_effect_by_id(effect.instance.registry_id)
            {
                if !effect.instance.plugin_state.is_empty() {
                    plugin.set_state(&effect.instance.plugin_state);
                }
                {
                    for spec in &effect.instance.parameter_specs {
                        plugin.set_parameter(spec.id, spec.value as f32);
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
                for spec in &effect.instance.parameter_specs {
                    plugin.set_parameter(spec.id, spec.value as f32);
                }
            }
            master_effects.insert(effect.id, plugin);
        }
    }

    // Build mixer channel seeds from AppState (read before lock is dropped)
    let mut track_channels = IndexMap::new();
    let mut bus_channels = IndexMap::new();
    let master_channel;
    {
        let app = crate::lock::get_app_read();
        for (&track_id, channel_arc) in &app.mixer.channels {
            track_channels.insert(
                track_id,
                MixerChannelSeed {
                    volume: channel_arc.channel.volume.get(),
                    pan: channel_arc.channel.pan.get(),
                    mute: channel_arc.channel.mute,
                    solo: channel_arc.channel.solo,
                    inverted_phase: channel_arc.channel.inverted_phase,
                },
            );
        }
        for (&bus_id, bus_arc) in &app.mixer.buses {
            bus_channels.insert(
                bus_id,
                MixerChannelSeed {
                    volume: bus_arc.channel.volume.get(),
                    pan: bus_arc.channel.pan.get(),
                    mute: bus_arc.channel.mute,
                    solo: bus_arc.channel.solo,
                    inverted_phase: bus_arc.channel.inverted_phase,
                },
            );
        }
        master_channel = MixerChannelSeed {
            volume: app.mixer.master_bus.volume.get(),
            pan: app.mixer.master_bus.pan.get(),
            mute: app.mixer.master_bus.mute,
            solo: app.mixer.master_bus.solo,
            inverted_phase: app.mixer.master_bus.inverted_phase,
        };
    }

    // Send the fully configured plugins to the Live Audio Engine
    send_audio_command(AudioCommand::HydratePlugin {
        generators,
        track_effects,
        bus_effects,
        master_effects,
        track_channels,
        bus_channels,
        master_channel,
    });

    Ok(())
}
