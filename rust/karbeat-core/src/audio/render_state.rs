use std::{ ops::Deref, sync::Arc };

use crate::{
    commands::AudioCommand,
    context::utils::send_audio_command,
    core::project::{
        ApplicationState,
        AssetLibrary,
        GeneratorId,
        GeneratorInstanceType,
        TrackId,
        automation::{ AutomationPoint, AutomationTarget, CurveType },
        mixer::{ MixerState },
        plugin::{ KarbeatEffect, KarbeatGenerator },
        track::{ KarbeatTrack, midi::{ Pattern} },
    },
    lock::{ get_app_read, get_plugin_registry_read },
};
use indexmap::IndexMap;
use karbeat_utils::math::is_power_of_two;
use crate::shared::id::*;

// =============================================================================
// Audio Thread Owned Plugin State
// =============================================================================

/// A generator plugin instance owned by the audio thread
pub struct AudioGeneratorInstance {
    pub id: GeneratorId,
    pub track_id: TrackId,
    pub plugin: Box<dyn KarbeatGenerator + Send + Sync>,
}

pub struct AudioEffectInstance {
    pub id: EffectId,
    pub plugin: Box<dyn KarbeatEffect + Send + Sync>,
}

/// Audio thread's owned plugin instances - NO locks required for access
/// This is managed via AudioCommand, NOT cloned from ApplicationState
#[derive(Default)]
pub struct AudioPluginState {
    /// Generator plugins stored as an arena. Index = GeneratorId as usize.
    /// `Option` allows us to "remove" generators without shifting the indices of others.
    pub generators: Vec<Option<AudioGeneratorInstance>>,

    /// Effect chain per track. Index = TrackId as usize.
    /// Empty tracks simply hold an empty Vec, avoiding `Option` overhead.
    pub track_effects: Vec<Vec<AudioEffectInstance>>,

    /// Master effect chain
    pub master_effects: Vec<AudioEffectInstance>,

    /// Bus effect chains. Index = BusId as usize.
    pub bus_effects: Vec<Vec<AudioEffectInstance>>,
}

impl AudioPluginState {
    // ==========================================
    // Generators
    // ==========================================

    /// Safely insert a generator, expanding the vector if the ID is out of bounds
    pub fn insert_generator(&mut self, id_index: usize, instance: AudioGeneratorInstance) {
        if id_index >= self.generators.len() {
            self.generators.resize_with(id_index + 1, || None);
        }
        self.generators[id_index] = Some(instance);
    }

    /// Remove a generator without shifting other elements
    pub fn remove_generator(&mut self, id_index: usize) {
        if let Some(slot) = self.generators.get_mut(id_index) {
            *slot = None;
        }
    }

    /// Get a mutable reference to a specific generator
    #[inline]
    pub fn get_generator_mut(&mut self, id_index: usize) -> Option<&mut AudioGeneratorInstance> {
        self.generators.get_mut(id_index).and_then(|g| g.as_mut())
    }

    /// Get an immutable reference to a specific generator
    #[inline]
    pub fn get_generator(&self, id_index: usize) -> Option<&AudioGeneratorInstance> {
        self.generators.get(id_index).and_then(|g| g.as_ref())
    }

    // ==========================================
    // Track Effects
    // ==========================================

    /// Add an effect to a track's chain, resizing the tracks array if needed
    pub fn add_track_effect(&mut self, track_id_index: usize, effect: AudioEffectInstance) {
        if track_id_index >= self.track_effects.len() {
            self.track_effects.resize_with(track_id_index + 1, Vec::new);
        }
        self.track_effects[track_id_index].push(effect);
    }

    #[inline]
    pub fn get_track_effects_mut(
        &mut self,
        track_id_index: usize
    ) -> Option<&mut Vec<AudioEffectInstance>> {
        self.track_effects.get_mut(track_id_index)
    }

    #[inline]
    pub fn get_track_effects(&self, track_id_index: usize) -> Option<&Vec<AudioEffectInstance>> {
        self.track_effects.get(track_id_index)
    }

    // ==========================================
    // Bus Effects
    // ==========================================

    /// Add an effect to a bus's chain, resizing the buses array if needed
    pub fn add_bus_effect(&mut self, bus_id_index: usize, effect: AudioEffectInstance) {
        if bus_id_index >= self.bus_effects.len() {
            self.bus_effects.resize_with(bus_id_index + 1, Vec::new);
        }
        self.bus_effects[bus_id_index].push(effect);
    }

    pub fn add_bus(&mut self, bus_id_index: usize) {
        if bus_id_index >= self.bus_effects.len() {
            self.bus_effects.resize_with(bus_id_index + 1, Vec::new);
        }
        self.bus_effects[bus_id_index] = Vec::new();
    }

    pub fn remove_bus(&mut self, bus_id_index: usize) {
        if let Some(bus) = self.bus_effects.get_mut(bus_id_index) {
            bus.clear();
        }
    }

    #[inline]
    pub fn get_bus_effects_mut(
        &mut self,
        bus_id_index: usize
    ) -> Option<&mut Vec<AudioEffectInstance>> {
        self.bus_effects.get_mut(bus_id_index)
    }

    #[inline]
    pub fn get_bus_effects(&self, bus_id_index: usize) -> Option<&Vec<AudioEffectInstance>> {
        self.bus_effects.get(bus_id_index)
    }
}

// =============================================================================
// Cloneable Graph State (metadata only, no plugin instances)
// =============================================================================

/// Lightweight automation lane snapshot for the audio thread.
/// Contains only the data needed for real-time interpolation.
#[derive(Clone, Debug)]
pub struct AudioAutomationLane {
    pub target: AutomationTarget,
    pub points: Vec<AutomationPoint>,
    pub enabled: bool,
    pub min: f32,
    pub max: f32,
    pub default_value: f32,
}

impl AudioAutomationLane {
    /// Get the denormalized value at a given time in ticks.
    /// Returns `default_value` (denormalized) if disabled or no points.
    #[inline]
    pub fn value_at_ticks(&self, time_ticks: u32) -> f32 {
        if !self.enabled || self.points.is_empty() {
            return self.denormalize(self.default_value);
        }
        let normalized = interpolate_points(&self.points, time_ticks);
        self.denormalize(normalized)
    }

    #[inline]
    fn denormalize(&self, normalized: f32) -> f32 {
        self.min + normalized * (self.max - self.min)
    }
}

/// Interpolate sorted automation points at the given time in ticks.
/// Returns a normalized value (0.0–1.0).
#[inline]
fn interpolate_points(points: &[AutomationPoint], time_ticks: u32) -> f32 {
    // Before first point
    if time_ticks <= points[0].time_ticks {
        return points[0].value;
    }

    // After last point
    let last = &points[points.len() - 1];
    if time_ticks >= last.time_ticks {
        return last.value;
    }

    // Binary search for the surrounding pair
    let idx = points.binary_search_by(|p| p.time_ticks.cmp(&time_ticks)).unwrap_or_else(|i| i);

    if idx == 0 {
        return points[0].value;
    }

    let p1 = &points[idx - 1];
    let p2 = &points[idx];
    let duration = p2.time_ticks.saturating_sub(p1.time_ticks);
    if duration == 0 {
        return p1.value;
    }

    let t = ((time_ticks - p1.time_ticks) as f32) / (duration as f32);

    match p1.curve_type {
        CurveType::Linear => p1.value + (p2.value - p1.value) * t,
        CurveType::Exponential => {
            let v1 = p1.value.max(0.0001);
            let v2 = p2.value.max(0.0001);
            v1 * (v2 / v1).powf(t)
        }
        CurveType::Step => p1.value,
    }
}

/// Structural State: Tracks, Patterns, Mixer, Assets (Heavy, changes rarely)
#[derive(Default, Clone)]
pub struct AudioGraphState {
    pub tracks: Arc<[Arc<KarbeatTrack>]>,
    pub patterns: IndexMap<PatternId, Arc<Pattern>>,
    pub mixer_state: MixerState,
    pub asset_library: Arc<AssetLibrary>,
    /// Automation lanes for real-time parameter modulation
    pub automation_lanes: IndexMap<AutomationId, AudioAutomationLane>,
    pub max_sample_index: u32,
    pub sample_rate: u32,
    pub buffer_size: usize,
}

impl From<&ApplicationState> for AudioGraphState {
    fn from(app: &ApplicationState) -> Self {
        let mut tracks_vec: Vec<Arc<KarbeatTrack>> = app.tracks.values().cloned().collect();
        tracks_vec.sort_by_key(|t| t.id);

        // Convert automation pool to lightweight audio-thread snapshots
        let automation_lanes: IndexMap<AutomationId, AudioAutomationLane> = app.automation_pool
            .iter()
            .filter(|(_, lane)| lane.enabled && !lane.points.is_empty())
            .map(|(&id, lane)| {
                (
                    id,
                    AudioAutomationLane {
                        target: lane.target.clone(),
                        points: lane.points.clone(),
                        enabled: lane.enabled,
                        min: lane.min,
                        max: lane.max,
                        default_value: lane.default_value,
                    },
                )
            })
            .collect();

        Self {
            tracks: Arc::from(tracks_vec),
            patterns: app.pattern_pool.clone(),
            mixer_state: app.mixer.clone(),
            asset_library: app.asset_library.clone(),
            automation_lanes,
            max_sample_index: app.max_sample_index,
            sample_rate: app.audio_config.sample_rate,
            buffer_size: if is_power_of_two(app.audio_config.buffer_size.into()) {
                app.audio_config.buffer_size as usize
            } else {
                64
            },
        }
    }
}

/// Consolidated State wrapper for the Audio Thread.
/// Contains only structural/graph data. Runtime transport state
/// is owned directly by AudioEngine.
#[derive(Clone, Default)]
pub struct AudioRenderState {
    pub graph: AudioGraphState,
}

impl From<&ApplicationState> for AudioRenderState {
    fn from(app: &ApplicationState) -> Self {
        Self {
            graph: AudioGraphState::from(app),
        }
    }
}

/// Add current loaded plugin to the audio engine thread when loading a new project
pub fn broadcast_plugin_state_loading() {
    let app_state = get_app_read();
    let registry = get_plugin_registry_read();

    // get current generator
    let generators: IndexMap<
        GeneratorId,
        Box<dyn KarbeatGenerator + Send + Sync>
    > = app_state.generator_pool
        .iter()
        .filter_map(|(id, arc)| {
            let generator_instance = arc.deref().to_owned();

            let GeneratorInstanceType::Plugin(instance) = generator_instance.instance_type else {
                return None;
            };

            // get box plugin from registry

            let Some((box_plugin, _)) = registry.create_generator_by_id(instance.registry_id) else {
                return None;
            };

            Some((id.to_owned(), box_plugin))
        })
        .collect();

    // get track effects
    let mixer_state = app_state.get_mixer_state();
    let track_chan = &mixer_state.channels;

    // As usual, doing the same thing but for track_channels
    // Turn it to IndexMap<TrackId, IndexMap<EffectId, Box<dyn KarbeatEffect + Send + Sync>>>
    let track_effects: IndexMap<
        TrackId,
        IndexMap<EffectId, Box<dyn KarbeatEffect + Send + Sync>>
    > = track_chan
        .iter()
        .map(|(track_id, arc_mixer_chan)| {
            let mix_chan = arc_mixer_chan.deref().to_owned();

            // iterate through effects
            // Use filter_map here because the inner registry lookup can fail (return None)
            let effects_map: IndexMap<EffectId, Box<dyn KarbeatEffect + Send + Sync>> = mix_chan.effects
                .iter()
                .filter_map(|eff| {
                    let effect_id = eff.id;
                    let eff_instance = eff.instance.as_ref();

                    // Get the effect from registry.
                    // We map the result to a tuple (effect_id, plugin_box) if successful.
                    registry
                        .create_effect_by_id(eff_instance.registry_id)
                        .map(|(plugin_box, _)| (
                            effect_id,
                            plugin_box,
                        ))
                })
                .collect();

            // Return the tuple for the outer IndexMap
            (track_id.to_owned(), effects_map)
        })
        .collect();

    // Do the same for bus_channels
    let bus_chan = &mixer_state.buses;

    let bus_effects: IndexMap<BusId, IndexMap<EffectId, Box<dyn KarbeatEffect + Send + Sync>>> = bus_chan
        .iter()
        .map(|(id, arc_mixer_channel)| {
            let mix_bus = arc_mixer_channel.deref().to_owned();

            let effect_maps: IndexMap<
                EffectId,
                Box<dyn KarbeatEffect + Send + Sync>
            > = mix_bus.channel.effects
                .iter()
                .filter_map(|eff| {
                    let effect_id = eff.id;
                    let eff_instance = eff.instance.as_ref();

                    // Get the effect from registry.
                    // We map the result to a tuple (effect_id, plugin_box) if successful.
                    registry
                        .create_effect_by_id(eff_instance.registry_id)
                        .map(|(plugin_box, _)| (
                            effect_id,
                            plugin_box,
                        ))
                })
                .collect();

            (id.to_owned(), effect_maps)
        })
        .collect();

    let master_channel = mixer_state.master_bus.as_ref();
    let master_effects: IndexMap<EffectId, Box<dyn KarbeatEffect + Send + Sync>> = master_channel.effects
        .iter()
        .filter_map(|eff| {
            let effect_id = eff.id;
            let eff_instance = eff.instance.as_ref();

            // Get the effect from registry.
            // We map the result to a tuple (effect_id, plugin_box) if successful.
            registry
                .create_effect_by_id(eff_instance.registry_id)
                .map(|(plugin_box, _)| (effect_id, plugin_box))
        })
        .collect();

    send_audio_command(AudioCommand::PreparePlugin {
        track_effects,
        master_effects,
        bus_effects,
        generators,
    });
}
