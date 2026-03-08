// Copyright (C) 2026 Haidar Wibowo
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 3.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.

/* src/audio/engine.rs */

use dasp::slice;
use rtrb::{Consumer, Producer};
use std::{collections::HashMap, sync::Arc};
use triple_buffer::Output;

use crate::{
    audio::{
        event::PlaybackPosition,
        render_state::{
            AudioEffectInstance, AudioGeneratorInstance, AudioPluginState, AudioRenderState,
        },
    },
    commands::{
        AudioCommand, AudioFeedback, EffectParameterSnapshot, EffectTarget,
        GeneratorParameterSnapshot,
    },
    core::project::{
        mixer::{BusId, MixerChannel, RoutingNode},
        plugin::{MidiEvent, MidiMessage},
        AudioWaveform, Clip, GeneratorId, GeneratorInstance, KarbeatSource, KarbeatTrack, Pattern,
        PatternId, TrackId,
    },
    utils::audio::db_to_linear,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackMode {
    Song,
    Pattern {
        pattern_id: PatternId,
        generator_id: GeneratorId,
    },
}

pub struct AudioEngine {
    // Comms
    state_consumer: Output<AudioRenderState>,
    position_producer: Producer<PlaybackPosition>,
    feedback_producer: Producer<AudioFeedback>,
    current_state: AudioRenderState,

    // Timeline (Song mode)
    sample_rate: u32,
    playhead_samples: u32,
    current_beat: usize,
    current_bar: usize,

    // Timeline (Pattern mode - independent from song)
    pattern_playhead_samples: u32,
    pattern_beat: usize,
    pattern_bar: usize,
    last_emitted_pattern_samples: u32,

    // Active Voices (lightweight references to plugins in plugin_state)
    active_generators: Vec<GeneratorVoice>,
    active_oneshots: Vec<AudioVoice>,
    preview_voices: Vec<PreviewVoice>,

    // Audio thread's owned plugins - NO locks required
    plugin_state: AudioPluginState,

    // Real-time Command Queue (UI → Audio)
    command_consumer: Consumer<AudioCommand>,

    // Update emit scheduler
    last_emitted_samples: u32,

    mix_buffer: Vec<f32>,

    /// Intermediate buffers for each bus (for routing matrix)
    bus_buffers: HashMap<BusId, Vec<f32>>,

    /// Temporary buffer for bus processing (avoids allocation in audio thread)
    bus_temp_buffer: Vec<f32>,

    /// Cached routing order (updated only when state changes, not every callback)
    cached_routing_order: Vec<RoutingNode>,

    /// Song playback vs Pattern playback
    playback_mode: PlaybackMode,
}

/// Lightweight voice reference - the actual plugin lives in AudioPluginState
pub struct GeneratorVoice {
    pub id: GeneratorId,
    pub track_id: TrackId,
    // Events queued for the CURRENT buffer block only
    pub events: Vec<MidiEvent>,
    // Track if this generator is persistent or temporary
    pub active: bool,
}

pub struct AudioVoice {
    pub track_id: TrackId,
    pub waveform: AudioWaveform,
    /// Where in the output buffer do we start writing? (0 to buffer_len)
    pub output_offset_samples: usize,
    /// Where in the source WAV file do we start reading?
    pub source_read_index: f64,
    /// The specific start point in the source (from clip.trim_start)
    pub start_boundary: f64,
    /// The specific end point in the source (from clip.trim_start)
    pub end_boundary: f64,
}

pub struct PreviewVoice {
    pub waveform: AudioWaveform,
    pub current_frame: f64,
    pub is_finished: bool,
    pub volume: f32,
}

impl PreviewVoice {
    pub fn new(waveform: AudioWaveform, volume: f32) -> Self {
        Self {
            waveform,
            current_frame: 0.0,
            is_finished: false,
            volume,
        }
    }
}

impl AudioEngine {
    pub fn new(
        state_consumer: Output<AudioRenderState>,
        command_consumer: Consumer<AudioCommand>,
        position_producer: Producer<PlaybackPosition>,
        feedback_producer: Producer<AudioFeedback>,
        sample_rate: u32,
        initial_state: AudioRenderState,
    ) -> Self {
        let mix_buffer = Vec::with_capacity(2048);
        Self {
            state_consumer,
            command_consumer,
            position_producer,
            feedback_producer,
            current_state: initial_state,
            sample_rate,
            playhead_samples: 0,
            active_generators: Vec::with_capacity(32),
            active_oneshots: Vec::with_capacity(32),
            preview_voices: Vec::with_capacity(4),
            plugin_state: AudioPluginState::default(),
            current_beat: 1,
            current_bar: 1,
            pattern_playhead_samples: 0,
            pattern_beat: 1,
            pattern_bar: 1,
            last_emitted_pattern_samples: 0,
            last_emitted_samples: 0,
            mix_buffer,
            bus_buffers: HashMap::new(),
            bus_temp_buffer: Vec::with_capacity(2048),
            cached_routing_order: Vec::new(),
            playback_mode: PlaybackMode::Song,
        }
    }

    pub fn process(&mut self, output_buffer: &mut [f32]) {
        // Sync State
        if self.state_consumer.update() {
            let new_state = self.state_consumer.read().clone();

            // Check if we switched from Playing -> Stopped via heavy update
            if self.current_state.transport.is_playing && !new_state.transport.is_playing {
                self.stop_all_active_generators();
            }

            // Update cached routing order only when state changes (not every callback)
            self.cached_routing_order = new_state.graph.mixer_state.get_routing_order();

            self.current_state = new_state;
        }

        // Process Commands (Play, Stop, Seek)
        while let Ok(cmd) = self.command_consumer.pop() {
            self.process_command(cmd);
        }

        // Clear Buffer
        output_buffer.fill(0.0);
        let channels = 2;
        let frame_count = output_buffer.len() / channels;

        // Transport Logic
        if self.current_state.transport.is_playing {
            match self.playback_mode {
                PlaybackMode::Song => {
                    // log::info!("Song mode");
                    self.process_song_mode(frame_count, output_buffer, channels);
                }
                PlaybackMode::Pattern {
                    pattern_id,
                    generator_id,
                } => {
                    // log::info!("Pattern mode");
                    self.process_pattern_mode(
                        pattern_id,
                        generator_id,
                        frame_count,
                        output_buffer,
                        channels,
                    );
                }
            }
        } else {
            // When transport is stopped, still render any active voices
            // (e.g., preview notes with sustain, ADSR tails)
            self.render_voices_to_buffer(output_buffer, channels);

            // Clean up audio voices (oneshots), keep generator voices active if they have pending audio tail
            self.cleanup_finished_voices();

            self.emit_static_position();
        }

        // Always Render Previews (Metronome, Browser Preview)
        self.render_previews_to_buffer(output_buffer, channels);
    }

    fn advance_playhead(&mut self, frame_count: usize) {
        self.playhead_samples += frame_count as u32;
        self.recalculate_beat_bar();
        self.emit_playback_position();
        self.cleanup_finished_voices();
    }

    fn advance_pattern_playhead(&mut self, frame_count: usize) {
        self.pattern_playhead_samples += frame_count as u32;
        self.recalculate_pattern_beat_bar();
        self.emit_playback_position();
        self.cleanup_finished_voices();
    }

    /// Recalculates pattern beat/bar based on pattern_playhead_samples
    fn recalculate_pattern_beat_bar(&mut self) {
        let tempo = self.current_state.transport.bpm;
        if tempo <= 0.0 {
            return;
        }

        let samples_per_beat = ((60.0 / tempo) * (self.sample_rate as f32)) as usize;
        if samples_per_beat == 0 {
            return;
        }

        // Pattern beat/bar are 1-indexed within the pattern
        self.pattern_beat = (self.pattern_playhead_samples as usize) / samples_per_beat + 1;
        self.pattern_bar = (self.pattern_beat - 1) / 4 + 1;
    }

    fn process_song_mode(
        &mut self,
        frame_count: usize,
        output_buffer: &mut [f32],
        channels: usize,
    ) {
        if self.playhead_samples > self.current_state.graph.max_sample_index {
            if self.current_state.transport.is_looping {
                // Reset playhead back to 0 without changing `is_playing` state
                self.playhead_samples = 0;
                self.recalculate_beat_bar();
                self.last_emitted_samples = 0;

                // Kill trailing notes/audio to prevent a massive wall of sound
                // from release tails accumulating when jumping back to bar 1
                self.stop_all_active_generators();
                self.active_oneshots.clear();

                // Immediately process the first block of the new loop
                self.process_block_song_mode(frame_count, output_buffer, channels);

                // Force a UI update to snap the playhead back visually
                self.emit_current_playback_position();
            } else {
                // If not looping, stop playback normally
                self.stop_playback();
            }
        } else {
            self.process_block_song_mode(frame_count, output_buffer, channels);
        }
    }

    // Process a block of frame rendering in SONG mode (normal playback)
    fn process_block_song_mode(
        &mut self,
        buffer_size: usize,
        output_buffer: &mut [f32],
        channels: usize,
    ) {
        // Schedule Events (MIDI / Audio Clips)
        self.resolve_sequencer_events(buffer_size);

        // Render Active Voices
        self.render_voices_to_buffer(output_buffer, channels);

        // Advance Playhead
        self.advance_playhead(buffer_size);
    }

    fn process_pattern_mode(
        &mut self,
        pattern_id: PatternId,
        generator_id: GeneratorId,
        frame_count: usize,
        output_buffer: &mut [f32],
        channels: usize,
    ) {
        let pattern = match self.current_state.graph.patterns.get(&pattern_id) {
            Some(p) => p,
            None => {
                // Pattern deleted? Stop.
                self.stop_playback();
                return;
            }
        };

        // Verify the generator exists in plugin_state
        if !self.plugin_state.generators.contains_key(&generator_id) {
            log::warn!("Pattern preview: Generator {:?} not found", generator_id);
            self.stop_playback();
            return;
        }

        let tempo = self.current_state.transport.bpm;
        let sample_rate = self.sample_rate as f32;

        let samples_per_beat = (60.0 / tempo) * sample_rate;
        let loop_len_samples = (((pattern.length_ticks as f32) / 960.0) * samples_per_beat) as u32;

        if loop_len_samples == 0 {
            return;
        }

        // Use PATTERN playhead (independent from song)
        if self.pattern_playhead_samples >= loop_len_samples {
            self.pattern_playhead_samples = 0;
            self.last_emitted_pattern_samples = 0;
            Self::stop_all_active_generators_impl(
                &mut self.active_generators,
                &mut self.plugin_state,
            ); // Kill notes at loop boundary to prevent hangs
        }

        let start_time = self.pattern_playhead_samples;
        let end_time = start_time + (frame_count as u32);

        // Find or create voice for this generator
        let voice_idx = self
            .active_generators
            .iter()
            .position(|g| g.id == generator_id)
            .unwrap_or_else(|| {
                // Get the track_id from plugin_state if available
                let track_id = self
                    .plugin_state
                    .generators
                    .get(&generator_id)
                    .map(|g| g.track_id)
                    .unwrap_or(TrackId::from(0));

                self.active_generators.push(GeneratorVoice {
                    id: generator_id,
                    track_id,
                    events: Vec::new(),
                    active: true,
                });
                self.active_generators.len() - 1
            });

        let gen_voice = &mut self.active_generators[voice_idx];
        gen_voice.active = true;

        Self::schedule_pattern_notes_raw(
            &mut gen_voice.events,
            &pattern.notes,
            self.sample_rate,
            tempo,
            start_time,
            end_time,
        );

        // Render voices to buffer
        self.render_voices_to_buffer(output_buffer, channels);

        // Advance PATTERN playhead (not song playhead)
        self.advance_pattern_playhead(frame_count);
    }

    fn stop_playback(&mut self) {
        self.reset_playhead();
    }

    fn stop_all_active_generators(&mut self) {
        Self::stop_all_active_generators_impl(&mut self.active_generators, &mut self.plugin_state);
    }

    fn stop_all_active_generators_impl(
        active_generators: &mut Vec<GeneratorVoice>,
        plugin_state: &mut AudioPluginState,
    ) {
        for voice in active_generators.iter_mut() {
            // Reset the plugin via plugin_state (no lock needed)
            if let Some(gen_instance) = plugin_state.generators.get_mut(&voice.id) {
                gen_instance.plugin.reset();
            }
            // Clear any pending MIDI events that might have been queued
            voice.events.clear();
        }
    }

    fn process_command(&mut self, cmd: AudioCommand) {
        match cmd {
            AudioCommand::PlayOneShot(waveform) => {
                self.preview_voices.clear();
                self.preview_voices.push(PreviewVoice::new(waveform, 1.0));
            }
            AudioCommand::StopAllPreviews => self.preview_voices.clear(),
            AudioCommand::ResetPlayhead => self.reset_playhead(),
            AudioCommand::SetPlayhead(samples) => {
                log::info!("[AudioEngine] Seek: {}", samples);
                self.playhead_samples = samples as u32;
                self.recalculate_beat_bar();
                self.last_emitted_samples = self.playhead_samples;
                self.emit_current_playback_position(); // Snap UI immediately
            }
            AudioCommand::PlayPreviewNote {
                note_key,
                generator_id,
                velocity,
                is_note_on,
            } => {
                // this should push preview voice in the shape of note pressed connected to generator.
                // e.g Note placing on piano roll, hold press from a keyboard,
                // or a press at the piano tile on the left of piano roll screen
                // it also requires the logic to handle input based on the ADSR of the voice generator
                self.trigger_live_note(generator_id.into(), note_key, velocity, is_note_on);
            }
            AudioCommand::SetBPM(bpm) => {
                self.current_state.transport.bpm = bpm;
                self.emit_current_playback_position();
            }
            AudioCommand::SetPlaybackMode(playback_mode) => {
                // Silence everything to prevent hanging notes from the previous mode
                self.stop_all_active_generators();

                // Reset the specific playhead for the new mode
                match (self.playback_mode, playback_mode) {
                    (PlaybackMode::Song, PlaybackMode::Pattern { .. }) => {
                        self.playhead_samples = 0;
                        self.recalculate_beat_bar();
                        self.last_emitted_samples = 0;
                    }
                    (PlaybackMode::Pattern { .. }, PlaybackMode::Song) => {
                        self.pattern_playhead_samples = 0;
                        self.last_emitted_pattern_samples = 0;
                        self.recalculate_pattern_beat_bar();
                    }
                    _ => {} // Same mode, do nothing
                }

                // update with new playback mode
                self.playback_mode = playback_mode;

                // Snap UI to the beginning immediately
                self.emit_current_playback_position();
            }
            AudioCommand::AddGenerator {
                generator_id,
                track_id,
                mut plugin,
            } => {
                // Prepare the plugin with current sample rate and buffer size
                let buf_size = self.current_state.graph.buffer_size.max(512);
                plugin.prepare(self.sample_rate as f32, buf_size);

                self.plugin_state.generators.insert(
                    generator_id,
                    AudioGeneratorInstance {
                        id: generator_id,
                        track_id,
                        plugin,
                    },
                );
                log::info!(
                    "[AudioEngine] Added generator {:?} for track {:?}",
                    generator_id,
                    track_id
                );
            }
            AudioCommand::RemoveGenerator { generator_id } => {
                self.plugin_state.generators.remove(&generator_id);
                // Also remove any active voice referencing it
                self.active_generators.retain(|v| v.id != generator_id);
                log::info!("[AudioEngine] Removed generator {:?}", generator_id);
            }
            AudioCommand::SetGeneratorParameter {
                generator_id,
                param_id,
                value,
            } => {
                if let Some(gen_instance) = self.plugin_state.generators.get_mut(&generator_id) {
                    gen_instance.plugin.set_parameter(param_id, value);
                }
            }
            AudioCommand::UpdateGeneratorTrack {
                generator_id,
                track_id,
            } => {
                if let Some(gen_instance) = self.plugin_state.generators.get_mut(&generator_id) {
                    gen_instance.track_id = track_id;
                }
                // Update active voice track association
                for voice in &mut self.active_generators {
                    if voice.id == generator_id {
                        voice.track_id = track_id;
                    }
                }
            }
            AudioCommand::AddTrackEffect {
                track_id,
                effect_id,
                mut effect,
            } => {
                // Prepare the effect
                let buf_size = self.current_state.graph.buffer_size.max(512);
                effect.prepare(self.sample_rate as f32, buf_size);

                self.plugin_state
                    .track_effects
                    .entry(track_id)
                    .or_default()
                    .push(AudioEffectInstance {
                        id: effect_id,
                        plugin: effect,
                    });
                log::info!("[AudioEngine] Added effect to track {:?}", track_id);
            }
            AudioCommand::RemoveTrackEffect {
                track_id,
                effect_id,
            } => {
                if let Some(effects) = self.plugin_state.track_effects.get_mut(&track_id) {
                    if let Some(effect) = effects.iter().position(|e| e.id == effect_id) {
                        effects.remove(effect);
                    }
                }
            }
            AudioCommand::SetTrackEffectParameter {
                track_id,
                effect_id,
                param_id,
                value,
            } => {
                if let Some(effects) = self.plugin_state.track_effects.get_mut(&track_id) {
                    if let Some(effect) = effects.iter().position(|e| e.id == effect_id) {
                        effects[effect].plugin.set_parameter(param_id, value);
                    }
                }
            }
            AudioCommand::QueryGeneratorParameters { generator_id } => {
                // Get all parameter values from the generator and send back
                if let Some(gen_instance) = self.plugin_state.generators.get(&generator_id) {
                    let specs = gen_instance.plugin.get_parameter_specs();
                    let parameters: Vec<(u32, f32)> = specs
                        .iter()
                        .map(|spec| (spec.id, gen_instance.plugin.get_parameter(spec.id)))
                        .collect();

                    let snapshot = GeneratorParameterSnapshot {
                        generator_id,
                        parameters,
                    };

                    // Best-effort push (don't block audio thread)
                    let _ = self
                        .feedback_producer
                        .push(AudioFeedback::GeneratorParameterSnapshot(snapshot));
                }
            }
            #[allow(unused_variables)]
            AudioCommand::AddMasterEffect {
                effect_id,
                mut effect,
            } => {
                let buf_size = self.current_state.graph.buffer_size;
                effect.prepare(self.sample_rate as f32, buf_size);
                self.plugin_state.master_effects.push(AudioEffectInstance {
                    id: effect_id,
                    plugin: effect,
                });
            }
            #[allow(unused_variables)]
            AudioCommand::RemoveMasterEffect { effect_id } => {
                if let Some(effects) = self
                    .plugin_state
                    .master_effects
                    .iter()
                    .position(|e| e.id == effect_id)
                {
                    self.plugin_state.master_effects.remove(effects);
                }
            }
            #[allow(unused_variables)]
            AudioCommand::SetMasterEffectParameter {
                effect_id,
                param_id,
                value,
            } => {
                if let Some(effects) = self
                    .plugin_state
                    .master_effects
                    .iter()
                    .position(|e| e.id == effect_id)
                {
                    self.plugin_state.master_effects[effects]
                        .plugin
                        .set_parameter(param_id, value);
                }
            }
            AudioCommand::AddBus { bus_id, name } => {
                // Initialize bus buffer and effects chain
                self.plugin_state.bus_effects.insert(bus_id, Vec::new());
                self.bus_buffers.insert(bus_id, Vec::new());
                log::info!("[AudioEngine] Added bus {:?} ({})", bus_id, name);
            }
            AudioCommand::RemoveBus { bus_id } => {
                self.plugin_state.bus_effects.remove(&bus_id);
                self.bus_buffers.remove(&bus_id);
                log::info!("[AudioEngine] Removed bus {:?}", bus_id);
            }
            AudioCommand::SetBusParams {
                bus_id,
                volume,
                pan,
                mute,
            } => {
                // Bus params are stored in current_state.graph.mixer_state
                // They get synced via triple buffer, so we don't need to do
                // anything special here. Log for debugging.
                log::debug!(
                    "[AudioEngine] SetBusParams for {:?}: vol={:?}, pan={:?}, mute={:?}",
                    bus_id,
                    volume,
                    pan,
                    mute
                );
            }
            AudioCommand::AddBusEffect {
                bus_id,
                effect_id,
                mut effect,
            } => {
                let buf_size = self.current_state.graph.buffer_size.max(512);
                effect.prepare(self.sample_rate as f32, buf_size);

                self.plugin_state
                    .bus_effects
                    .entry(bus_id)
                    .or_default()
                    .push(AudioEffectInstance {
                        id: effect_id,
                        plugin: effect,
                    });
                log::info!(
                    "[AudioEngine] Added effect {:?} to bus {:?}",
                    effect_id,
                    bus_id
                );
            }
            AudioCommand::RemoveBusEffect { bus_id, effect_id } => {
                if let Some(effects) = self.plugin_state.bus_effects.get_mut(&bus_id) {
                    if let Some(pos) = effects.iter().position(|e| e.id == effect_id) {
                        effects.remove(pos);
                    }
                }
            }
            AudioCommand::SetBusEffectParameter {
                bus_id,
                effect_id,
                param_id,
                value,
            } => {
                if let Some(effects) = self.plugin_state.bus_effects.get_mut(&bus_id) {
                    if let Some(effect) = effects.iter_mut().find(|e| e.id == effect_id) {
                        effect.plugin.set_parameter(param_id, value);
                    }
                }
            }
            AudioCommand::UpdateRouting { routing } => {
                // Routing is stored in mixer_state and synced via triple buffer
                // Log for debugging
                log::info!(
                    "[AudioEngine] Received routing update with {} connections",
                    routing.len()
                );
            }
            AudioCommand::QueryTrackEffectParameters {
                track_id,
                effect_id,
            } => {
                if let Some(effects) = self.plugin_state.track_effects.get(&track_id) {
                    if let Some(effect_instance) = effects.iter().find(|e| e.id == effect_id) {
                        let specs = effect_instance.plugin.get_parameter_specs();
                        let parameters: Vec<(u32, f32)> = specs
                            .iter()
                            .map(|spec| (spec.id, effect_instance.plugin.get_parameter(spec.id)))
                            .collect();

                        let snapshot = EffectParameterSnapshot {
                            target: EffectTarget::Track(track_id),
                            effect_id,
                            parameters,
                        };

                        let _ = self
                            .feedback_producer
                            .push(AudioFeedback::EffectParameterSnapshot(snapshot));
                    }
                }
            }
            AudioCommand::QueryMasterEffectParameters { effect_id } => {
                if let Some(effect_instance) = self
                    .plugin_state
                    .master_effects
                    .iter()
                    .find(|e| e.id == effect_id)
                {
                    let specs = effect_instance.plugin.get_parameter_specs();
                    let parameters: Vec<(u32, f32)> = specs
                        .iter()
                        .map(|spec| (spec.id, effect_instance.plugin.get_parameter(spec.id)))
                        .collect();

                    let snapshot = EffectParameterSnapshot {
                        target: EffectTarget::Master,
                        effect_id,
                        parameters,
                    };

                    let _ = self
                        .feedback_producer
                        .push(AudioFeedback::EffectParameterSnapshot(snapshot));
                }
            }
            AudioCommand::QueryBusEffectParameters { bus_id, effect_id } => {
                if let Some(effects) = self.plugin_state.bus_effects.get(&bus_id) {
                    if let Some(effect_instance) = effects.iter().find(|e| e.id == effect_id) {
                        let specs = effect_instance.plugin.get_parameter_specs();
                        let parameters: Vec<(u32, f32)> = specs
                            .iter()
                            .map(|spec| (spec.id, effect_instance.plugin.get_parameter(spec.id)))
                            .collect();

                        let snapshot = EffectParameterSnapshot {
                            target: EffectTarget::Bus(bus_id),
                            effect_id,
                            parameters,
                        };

                        let _ = self
                            .feedback_producer
                            .push(AudioFeedback::EffectParameterSnapshot(snapshot));
                    }
                }
            }
        }
    }

    /// Recalculates current Beat and Bar based on playhead_samples
    /// Uses 1-based indexing for musical time.
    fn recalculate_beat_bar(&mut self) {
        let tempo = self.current_state.transport.bpm;
        if tempo <= 0.0 {
            return;
        }

        let samples_per_beat = ((60.0 / tempo) * (self.sample_rate as f32)) as usize;
        if samples_per_beat == 0 {
            return;
        }

        self.current_beat = (self.playhead_samples as usize) / samples_per_beat + 1;
        self.current_bar = (self.current_beat - 1) / 4 + 1;
    }

    fn reset_playhead(&mut self) {
        log::info!("[AudioEngine] Reset Playhead");
        self.playhead_samples = 0;
        self.current_beat = 1;
        self.current_bar = 1;
        self.last_emitted_samples = 0;
        self.current_state.transport.is_playing = false;
        self.emit_static_position();
    }

    fn emit_playback_position(&mut self) {
        let emission_interval = self.sample_rate / 60; // ~60fps
        let (current, last) = match self.playback_mode {
            PlaybackMode::Song => (self.playhead_samples, self.last_emitted_samples),
            PlaybackMode::Pattern { .. } => (
                self.pattern_playhead_samples,
                self.last_emitted_pattern_samples,
            ),
        };
        if current >= last + emission_interval {
            if !self.position_producer.is_full() {
                let _ = self
                    .position_producer
                    .push(self.build_position_struct(Some(true)));
            }
            match self.playback_mode {
                PlaybackMode::Song => {
                    self.last_emitted_samples = self.playhead_samples;
                }
                PlaybackMode::Pattern { .. } => {
                    self.last_emitted_pattern_samples = self.pattern_playhead_samples;
                }
            }
        }
    }

    fn emit_static_position(&mut self) {
        if !self.position_producer.is_full() {
            let _ = self
                .position_producer
                .push(self.build_position_struct(Some(false)));
        }
    }

    fn build_position_struct(&self, is_playing: Option<bool>) -> PlaybackPosition {
        let is_playing = is_playing.unwrap_or(self.current_state.transport.is_playing);
        let is_pattern_mode = matches!(self.playback_mode, PlaybackMode::Pattern { .. });

        PlaybackPosition {
            // Song position
            samples: self.playhead_samples,
            beat: self.current_beat,
            bar: self.current_bar,
            tempo: self.current_state.transport.bpm,
            sample_rate: self.current_state.graph.sample_rate,
            is_playing,
            // Pattern position (independent)
            is_pattern_mode,
            pattern_samples: self.pattern_playhead_samples,
            pattern_beat: self.pattern_beat,
            pattern_bar: self.pattern_bar,
        }
    }

    fn emit_current_playback_position(&mut self) {
        if !self.position_producer.is_full() {
            let _ = self
                .position_producer
                .push(self.build_position_struct(None));
        }
    }

    fn cleanup_finished_voices(&mut self) {
        // Generators stay alive (persistent), just clear their MIDI events for the next frame
        self.active_generators.retain(|g| g.active);
        for gen in self.active_generators.iter_mut() {
            gen.events.clear();
        }

        // Audio voices are One-Shot per buffer (cleared every frame)
        self.active_oneshots.clear();
    }

    fn trigger_live_note(&mut self, generator_id: GeneratorId, key: u8, velocity: u8, is_on: bool) {
        // Try to find the track that has this generator from current_state
        let target_info = self.current_state.graph.tracks.iter().find_map(|t| {
            if let Some(gen) = &t.generator {
                if gen.id == generator_id {
                    return Some((t.id, gen.clone()));
                }
            }
            None
        });

        // If we found the track info, use it
        if let Some((track_id, gen_instance)) = target_info {
            if let Some(voice_idx) = Self::ensure_generator_voice(
                &mut self.active_generators,
                &self.plugin_state,
                track_id,
                &gen_instance,
            ) {
                let gen_voice = &mut self.active_generators[voice_idx];
                let message = if is_on {
                    MidiMessage::NoteOn { key, velocity }
                } else {
                    MidiMessage::NoteOff { key }
                };

                gen_voice.events.push(MidiEvent {
                    sample_offset: 0,
                    data: message,
                });
                gen_voice.active = true;
                return;
            }
        }

        // Fallback: If triple buffer hasn't synced yet, check plugin_state directly
        // This handles the case where AudioCommand::AddGenerator was received but
        // the UI hasn't updated current_state via triple buffer yet
        if let Some(gen_instance) = self.plugin_state.generators.get(&generator_id) {
            let track_id = gen_instance.track_id;

            // Find or create voice
            let voice_idx = self
                .active_generators
                .iter()
                .position(|g| g.id == generator_id)
                .unwrap_or_else(|| {
                    self.active_generators.push(GeneratorVoice {
                        id: generator_id,
                        track_id,
                        events: Vec::new(),
                        active: true,
                    });
                    self.active_generators.len() - 1
                });

            let gen_voice = &mut self.active_generators[voice_idx];
            let message = if is_on {
                MidiMessage::NoteOn { key, velocity }
            } else {
                MidiMessage::NoteOff { key }
            };

            gen_voice.events.push(MidiEvent {
                sample_offset: 0,
                data: message,
            });
            gen_voice.active = true;
        } else {
            log::warn!(
                "PlayPreviewNote: Generator ID {:?} not found in plugin_state or graph",
                generator_id
            );
        }
    }

    fn render_voices_to_buffer(&mut self, output: &mut [f32], channels: usize) {
        let buf_len = output.len();

        // Ensure bus buffers are properly sized
        for (_bus_id, buf) in self.bus_buffers.iter_mut() {
            if buf.len() != buf_len {
                buf.resize(buf_len, 0.0);
            }
            buf.fill(0.0);
        }

        // Check for solo state
        let is_any_solo = self
            .current_state
            .graph
            .mixer_state
            .channels
            .values()
            .any(|ch| ch.solo);

        // Get routing info
        let routing = &self.current_state.graph.mixer_state.routing;

        // ==== Phase 1: Render all tracks and route to destinations ====
        for track in self.current_state.graph.tracks.iter() {
            let track_id = track.id;

            let default_channel = Arc::new(MixerChannel {
                volume: 1.0,
                pan: 0.0,
                mute: false,
                solo: false,
                inverted_phase: false,
                effects: Vec::new(),
                ..Default::default()
            });

            let channel = self
                .current_state
                .graph
                .mixer_state
                .channels
                .get(&track_id)
                .unwrap_or(&default_channel);

            // Check mute/solo
            if channel.mute {
                continue;
            }
            if is_any_solo && !channel.solo {
                continue;
            }

            // Ensure mix_buffer is sized correctly
            if self.mix_buffer.len() != buf_len {
                self.mix_buffer.resize(buf_len, 0.0);
            }
            self.mix_buffer.fill(0.0);

            let mut has_signal = false;

            // Generator Voice - use plugin_state directly (no lock!)
            if let Some(gen_voice) = self
                .active_generators
                .iter()
                .find(|g| g.track_id == track_id && g.active)
            {
                let gen_id = gen_voice.id;
                let events = &gen_voice.events;

                // Access the generator from plugin_state (owned, no lock)
                if let Some(gen_instance) = self.plugin_state.generators.get_mut(&gen_id) {
                    gen_instance.plugin.process(&mut self.mix_buffer, events);
                    has_signal = true;
                }
            }

            // Audio Voice
            if Self::render_oneshots(
                &mut self.active_oneshots,
                self.sample_rate,
                track_id,
                &mut self.mix_buffer,
                channels,
            ) {
                has_signal = true;
            }

            if !has_signal {
                continue;
            }

            // Apply track mixer channel (volume/pan/phase) and effects
            Self::apply_mixer_channel_with_effects(
                channel,
                &mut self.plugin_state.track_effects,
                track_id,
                &mut self.mix_buffer,
                channels,
            );

            // Route the track signal to destinations based on routing matrix
            let track_routes: Vec<_> = routing
                .iter()
                .filter(|c| c.source == RoutingNode::Track(track_id))
                .collect();

            if track_routes.is_empty() {
                // No explicit routing: go directly to master (backward compatibility)
                for i in 0..buf_len {
                    output[i] += self.mix_buffer[i];
                }
            } else {
                // Route to each destination with appropriate send level
                for conn in track_routes {
                    match conn.destination {
                        RoutingNode::Master => {
                            for i in 0..buf_len {
                                output[i] += self.mix_buffer[i] * conn.send_level;
                            }
                        }
                        RoutingNode::Bus(bus_id) => {
                            if let Some(bus_buf) = self.bus_buffers.get_mut(&bus_id) {
                                for i in 0..buf_len {
                                    bus_buf[i] += self.mix_buffer[i] * conn.send_level;
                                }
                            }
                        }
                        RoutingNode::Track(_) => {
                            // Invalid: can't route to a track
                        }
                    }
                }
            }
        }

        // ==== Phase 2: Process buses in topological order ====
        // Use cached routing order (computed only on state update, not every callback)
        for node in self.cached_routing_order.clone().iter() {
            if let RoutingNode::Bus(bus_id) = node {
                // Copy bus audio to temp buffer (avoid clone allocation)
                let bus_buf = match self.bus_buffers.get(bus_id) {
                    Some(buf) => buf,
                    None => {
                        continue;
                    }
                };

                // Resize temp buffer if needed and copy
                if self.bus_temp_buffer.len() != buf_len {
                    self.bus_temp_buffer.resize(buf_len, 0.0);
                }
                self.bus_temp_buffer.copy_from_slice(bus_buf);

                // Get bus channel settings
                let bus_channel = self
                    .current_state
                    .graph
                    .mixer_state
                    .buses
                    .get(bus_id)
                    .map(|b| &b.channel);

                let Some(bus_settings) = bus_channel else {
                    continue;
                };

                // Skip if muted
                if bus_settings.mute {
                    continue;
                }

                // Copy to mix_buffer for processing
                if self.mix_buffer.len() != buf_len {
                    self.mix_buffer.resize(buf_len, 0.0);
                }
                self.mix_buffer.copy_from_slice(&self.bus_temp_buffer);

                // Apply bus effects
                if let Some(effects) = self.plugin_state.bus_effects.get_mut(bus_id) {
                    for effect in effects.iter_mut() {
                        effect.plugin.process(&mut self.mix_buffer);
                    }
                }

                // Apply volume and pan (volume is stored in dB)
                let volume = db_to_linear(bus_settings.volume);
                let pan = bus_settings.pan.clamp(-1.0, 1.0);
                let (left_gain, right_gain) = if channels == 2 {
                    let p = (pan + 1.0) * 0.5;
                    ((1.0 - p).sqrt() * volume, p.sqrt() * volume)
                } else {
                    (volume, volume)
                };

                // Use dasp Frame abstraction for clean channel math
                if channels == 2 {
                    if let Some(frames) =
                        slice::from_sample_slice_mut::<&mut [[f32; 2]], f32>(&mut self.mix_buffer)
                    {
                        for frame in frames {
                            frame[0] *= left_gain;
                            frame[1] *= right_gain;
                        }
                    }
                } else {
                    for sample in self.mix_buffer.iter_mut() {
                        *sample *= left_gain;
                    }
                }

                // Route bus output to destinations
                let bus_routes: Vec<_> = routing
                    .iter()
                    .filter(|c| c.source == RoutingNode::Bus(*bus_id))
                    .collect();

                for conn in bus_routes {
                    match conn.destination {
                        RoutingNode::Master => {
                            for i in 0..buf_len {
                                output[i] += self.mix_buffer[i] * conn.send_level;
                            }
                        }
                        RoutingNode::Bus(dest_bus_id) => {
                            if let Some(dest_buf) = self.bus_buffers.get_mut(&dest_bus_id) {
                                for i in 0..buf_len {
                                    dest_buf[i] += self.mix_buffer[i] * conn.send_level;
                                }
                            }
                        }
                        RoutingNode::Track(_) => {}
                    }
                }
            }
        }

        // ==== Phase 3: Apply master bus effects ====
        let master_bus = self.current_state.graph.mixer_state.master_bus.clone();
        Self::apply_master_bus_with_effects(
            &master_bus,
            &mut self.plugin_state.master_effects,
            output,
            channels,
        );
    }

    fn render_oneshots(
        active_oneshots: &mut [AudioVoice],
        sample_rate: u32,
        track_id: TrackId,
        output: &mut [f32],
        channels: usize,
    ) -> bool {
        let mut did_render = false;
        let buffer_frames = output.len() / channels;
        for voice in active_oneshots
            .iter_mut()
            .filter(|v| v.track_id == track_id)
        {
            did_render = true;
            let src_channels = voice.waveform.channels as usize;
            let step = (voice.waveform.sample_rate as f64) / (sample_rate as f64);

            // Pre-calculate Loop Bounds to hoist out of the loop
            let max_len = (voice.waveform.buffer.len() / src_channels) as f64;
            let trim_end = if voice.end_boundary > 0.0 && voice.end_boundary < max_len {
                voice.end_boundary
            } else {
                max_len
            };
            let loop_len = trim_end - voice.start_boundary;
            let is_looping = voice.waveform.is_looping && loop_len > 0.0;

            if channels == 2 {
                if let Some(out_frames) =
                    slice::from_sample_slice_mut::<&mut [[f32; 2]], f32>(output)
                {
                    for frame_idx in voice.output_offset_samples..buffer_frames {
                        let frames_written = (frame_idx - voice.output_offset_samples) as u32;
                        let mut read_pos = voice.source_read_index + (frames_written as f64) * step;

                        if is_looping {
                            if read_pos >= trim_end {
                                let remainder = read_pos - trim_end;
                                read_pos = voice.start_boundary + (remainder % loop_len);
                            }
                        } else if read_pos >= trim_end - 1.0 {
                            break;
                        }

                        let sample_frame =
                            sample_waveform_dasp(&voice.waveform, read_pos, src_channels);
                        out_frames[frame_idx][0] += sample_frame[0];
                        out_frames[frame_idx][1] += sample_frame[1];
                    }
                }
            } else {
                // Fallback for non-stereo output
                for frame_idx in voice.output_offset_samples..buffer_frames {
                    let frames_written = (frame_idx - voice.output_offset_samples) as u32;
                    let mut read_pos = voice.source_read_index + (frames_written as f64) * step;

                    if is_looping {
                        if read_pos >= trim_end {
                            let remainder = read_pos - trim_end;
                            read_pos = voice.start_boundary + (remainder % loop_len);
                        }
                    } else if read_pos >= trim_end - 1.0 {
                        break;
                    }

                    let sample_frame =
                        sample_waveform_dasp(&voice.waveform, read_pos, src_channels);
                    output[frame_idx * channels] += sample_frame[0];
                }
            }
        }
        did_render
    }

    /// Apply mixer channel settings (volume, pan, phase) and effects from plugin_state
    fn apply_mixer_channel_with_effects(
        mixer_channel: &MixerChannel,
        track_effects: &mut std::collections::HashMap<TrackId, Vec<AudioEffectInstance>>,
        track_id: TrackId,
        buffer: &mut [f32],
        channels: usize,
    ) {
        // Invert Phase
        if mixer_channel.inverted_phase {
            for sample in buffer.iter_mut() {
                *sample = -*sample;
            }
        }

        // Effects chain from plugin_state
        if let Some(effects) = track_effects.get_mut(&track_id) {
            for effect in effects.iter_mut() {
                effect.plugin.process(buffer);
            }
        }

        // Volume and Pan (volume is stored in dB)
        let pan = mixer_channel.pan.clamp(-1.0, 1.0);
        let volume = db_to_linear(mixer_channel.volume);
        let (left_gain, right_gain) = if channels == 2 {
            let p = (pan + 1.0) * 0.5;
            ((1.0 - p).sqrt() * volume, p.sqrt() * volume)
        } else {
            (volume, volume)
        };

        // Apply gain
        if channels == 2 {
            if let Some(frames) = slice::to_frame_slice_mut::<&mut [f32], [f32; 2]>(buffer) {
                for frame in frames {
                    frame[0] *= left_gain;
                    frame[1] *= right_gain;
                }
            }
        } else {
            for sample in buffer.iter_mut() {
                *sample *= left_gain;
            }
        }
    }

    /// Apply master bus settings (volume, pan, phase) and effects from plugin_state
    ///
    /// # Parameters
    ///
    /// * `master_bus` - The master bus settings
    /// * `master_effects` - The master bus effects chain
    /// * `buffer` - The buffer to apply the master bus settings to
    /// * `channels` - The number of channels in the buffer
    fn apply_master_bus_with_effects(
        master_bus: &MixerChannel,
        master_effects: &mut [AudioEffectInstance],
        buffer: &mut [f32],
        channels: usize,
    ) {
        // Invert Phase
        if master_bus.inverted_phase {
            for sample in buffer.iter_mut() {
                *sample = -*sample;
            }
        }

        // Master effects chain
        for effect in master_effects.iter_mut() {
            effect.plugin.process(buffer);
        }

        // Volume and Pan (volume is stored in dB)
        let pan = master_bus.pan.clamp(-1.0, 1.0);
        let volume = db_to_linear(master_bus.volume);
        let (left_gain, right_gain) = if channels == 2 {
            let p = (pan + 1.0) * 0.5;
            ((1.0 - p).sqrt() * volume, p.sqrt() * volume)
        } else {
            (volume, volume)
        };

        // Apply gain
        if channels == 2 {
            if let Some(frames) = slice::to_frame_slice_mut::<&mut [f32], [f32; 2]>(buffer) {
                for frame in frames {
                    frame[0] *= left_gain;
                    frame[1] *= right_gain;
                }
            }
        } else {
            for sample in buffer.iter_mut() {
                *sample *= left_gain;
            }
        }
    }

    fn resolve_sequencer_events(&mut self, buffer_size: usize) {
        let start_time = self.playhead_samples;
        let end_time = start_time + (buffer_size as u32);

        // Use the tracks from the current audio graph state
        let tracks = self.current_state.graph.tracks.clone();

        for track in tracks.iter() {
            self.process_track(track.as_ref(), start_time, end_time);
        }
    }

    fn process_track(&mut self, track: &KarbeatTrack, start_time: u32, end_time: u32) {
        let track_id = track.id;

        // Ensure Generator Voice exists
        let mut gen_voice_idx = None;
        if let Some(gen_instance) = &track.generator {
            gen_voice_idx = Self::ensure_generator_voice(
                &mut self.active_generators,
                &self.plugin_state,
                track_id,
                gen_instance,
            );
        }

        // Process Clips
        for clip in track.clips() {
            if clip.start_time > end_time {
                break;
            } // Optimization: Clips are sorted? If not, remove break.
            let clip_end = clip.start_time + clip.loop_length;
            if clip_end < start_time {
                continue;
            }

            match &clip.source {
                KarbeatSource::Audio(source_id) => {
                    // Look up the actual waveform from asset library
                    let waveform_opt = self
                        .current_state
                        .graph
                        .asset_library
                        .source_map
                        .get(source_id)
                        .cloned();
                    if let Some(waveform) = waveform_opt {
                        self.prepare_audio_voice(track.id, clip, &waveform, start_time, end_time);
                    }
                }
                KarbeatSource::Midi(id) => {
                    // Look up the FRESH pattern from the pool using the ID.
                    let fresh_pattern = self.current_state.graph.patterns.get(&id);

                    if let Some(pattern) = fresh_pattern {
                        if let Some(idx) = gen_voice_idx {
                            let gen_voice = &mut self.active_generators[idx];
                            Self::schedule_midi_events(
                                &mut gen_voice.events,
                                self.sample_rate,
                                self.current_state.transport.bpm,
                                clip,
                                pattern,
                                start_time,
                                end_time,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Ensure that the generator voice is active
    fn ensure_generator_voice(
        active_generators: &mut Vec<GeneratorVoice>,
        plugin_state: &AudioPluginState,
        track_id: TrackId,
        gen_instance: &GeneratorInstance,
    ) -> Option<usize> {
        // Find existing generator voice by ID
        if let Some(idx) = active_generators
            .iter()
            .position(|g| g.id == gen_instance.id)
        {
            return Some(idx);
        }

        // Check if the plugin exists in our owned state
        if plugin_state.generators.contains_key(&gen_instance.id) {
            // Create lightweight voice reference (actual plugin is in plugin_state)
            active_generators.push(GeneratorVoice {
                id: gen_instance.id,
                track_id,
                events: Vec::new(),
                active: true,
            });
            return Some(active_generators.len() - 1);
        }

        None
    }

    /// Render preview voices to the output buffer
    fn render_previews_to_buffer(&mut self, output: &mut [f32], channels: usize) {
        let buffer_frames = output.len() / channels;

        for voice in &mut self.preview_voices {
            if voice.is_finished {
                continue;
            }

            let src_channels = voice.waveform.channels as usize;
            let buffer_len = voice.waveform.buffer.len();
            let step = (voice.waveform.sample_rate as f64) / (self.sample_rate as f64);

            if channels == 2 {
                if let Some(out_frames) = slice::to_frame_slice_mut::<&mut [f32], [f32; 2]>(output)
                {
                    for i in 0..buffer_frames {
                        let current_pos_f64 =
                            voice.current_frame + (voice.waveform.trim_start as f64);
                        let trim_end = voice.waveform.trim_end as f64;
                        let max_len = (buffer_len / src_channels) as f64;
                        let end_bound = if trim_end > 0.0 && trim_end < max_len {
                            trim_end
                        } else {
                            max_len
                        };

                        if current_pos_f64 >= end_bound - 1.0 {
                            voice.is_finished = true;
                            break;
                        }

                        let sample_frame =
                            sample_waveform_dasp(&voice.waveform, current_pos_f64, src_channels);
                        out_frames[i][0] += sample_frame[0] * voice.volume;
                        out_frames[i][1] += sample_frame[1] * voice.volume;

                        voice.current_frame += step;
                    }
                }
            } else {
                for i in 0..buffer_frames {
                    let current_pos_f64 = voice.current_frame + (voice.waveform.trim_start as f64);
                    let trim_end = voice.waveform.trim_end as f64;
                    let max_len = (buffer_len / src_channels) as f64;
                    let end_bound = if trim_end > 0.0 && trim_end < max_len {
                        trim_end
                    } else {
                        max_len
                    };

                    if current_pos_f64 >= end_bound - 1.0 {
                        voice.is_finished = true;
                        break;
                    }

                    let sample_frame =
                        sample_waveform_dasp(&voice.waveform, current_pos_f64, src_channels);
                    output[i * channels] += sample_frame[0] * voice.volume;

                    voice.current_frame += step;
                }
            }
        }

        self.preview_voices.retain(|v| !v.is_finished);
    }

    /// Prepare audio voice from Audio Waveform that will be rendered
    fn prepare_audio_voice(
        &mut self,
        track_id: TrackId,
        clip: &Clip,
        waveform: &AudioWaveform,
        buffer_start: u32,
        buffer_end: u32,
    ) {
        let clip_timeline_start = clip.start_time;
        let render_start = std::cmp::max(buffer_start, clip_timeline_start);
        let render_end = std::cmp::min(buffer_end, clip_timeline_start + clip.loop_length);

        if render_end <= render_start {
            return;
        }

        let output_offset = (render_start - buffer_start) as usize;
        let samples_elapsed = render_start - clip_timeline_start;
        let effective_pos = samples_elapsed + clip.offset_start;

        let ratio = (waveform.sample_rate as f64) / (self.sample_rate as f64);
        let source_elapsed_frames = (effective_pos as f64) * ratio;

        let trim_start = waveform.trim_start as f64;
        let trim_end = if waveform.trim_end > 0 {
            waveform.trim_end as f64
        } else {
            (waveform.buffer.len() / (waveform.channels as usize)) as f64
        };
        let loop_len = trim_end - trim_start;

        let source_read_idx = if waveform.is_looping && loop_len > 0.0 {
            trim_start + (source_elapsed_frames % loop_len)
        } else {
            let idx = trim_start + source_elapsed_frames;
            if idx >= trim_end {
                return;
            }
            idx
        };

        self.active_oneshots.push(AudioVoice {
            track_id,
            waveform: waveform.clone(),
            output_offset_samples: output_offset,
            source_read_index: source_read_idx,
            start_boundary: trim_start,
            end_boundary: trim_end,
        });
    }

    fn schedule_midi_events(
        events: &mut Vec<MidiEvent>,
        sample_rate: u32,
        tempo: f32,
        clip: &Clip,
        pattern: &Pattern,
        buffer_start: u32,
        buffer_end: u32,
    ) {
        let samples_per_beat = ((60.0 / tempo) * (sample_rate as f32)) as u32;
        if samples_per_beat == 0 {
            return;
        }

        let pattern_len_samples =
            (((pattern.length_ticks as f64) / 960.0) * (samples_per_beat as f64)) as u32;
        if pattern_len_samples == 0 {
            return;
        }

        // Calculate the clip's actual end boundary on the timeline
        let clip_end = clip.start_time + clip.loop_length;

        let start_iter = 0;
        let end_iter = 0;

        for i in start_iter..=end_iter {
            let pattern_offset = i * pattern_len_samples;

            for note in &pattern.notes {
                let note_start =
                    (((note.start_tick as f64) / 960.0) * (samples_per_beat as f64)) as u32;
                let note_dur =
                    (((note.duration as f64) / 960.0) * (samples_per_beat as f64)) as u32;

                // Note position within the pattern (in samples from pattern start)
                let note_pos_in_pattern = pattern_offset + note_start;

                // Skip notes that start before the clip's trim offset
                if note_pos_in_pattern < clip.offset_start {
                    continue;
                }

                // Calculate absolute timeline position: clip start + (note position - trim offset)
                let abs_start = clip.start_time + note_pos_in_pattern - clip.offset_start;
                let abs_end = abs_start + note_dur;

                // Skip notes that start at or after the clip end (outside trimmed region)
                if abs_start >= clip_end {
                    continue;
                }

                // Schedule NoteOn if it falls within the buffer
                if abs_start >= buffer_start && abs_start < buffer_end {
                    events.push(MidiEvent {
                        sample_offset: (abs_start - buffer_start) as usize,
                        data: MidiMessage::NoteOn {
                            key: note.key,
                            velocity: note.velocity,
                        },
                    });
                }

                // Clamp note-off to clip boundary if it would extend past the clip end
                // This prevents hanging notes when clips are trimmed
                let effective_end = abs_end.min(clip_end);

                // Schedule NoteOff if it falls within the buffer
                if effective_end >= buffer_start && effective_end < buffer_end {
                    events.push(MidiEvent {
                        sample_offset: (effective_end - buffer_start) as usize,
                        data: MidiMessage::NoteOff { key: note.key },
                    });
                }
            }
        }
        events.sort_by_key(|e| e.sample_offset);
    }

    // Helper to schedule notes without a Clip wrapper
    fn schedule_pattern_notes_raw(
        events: &mut Vec<MidiEvent>,
        notes: &[crate::core::project::Note],
        sample_rate: u32,
        tempo: f32,
        buffer_start: u32,
        buffer_end: u32,
    ) {
        let samples_per_tick = ((60.0 / tempo) * (sample_rate as f32)) / 960.0;

        for note in notes {
            let note_start = ((note.start_tick as f32) * samples_per_tick) as u32;
            let note_end = note_start + (((note.duration as f32) * samples_per_tick) as u32);

            if note_start >= buffer_start && note_start < buffer_end {
                events.push(MidiEvent {
                    sample_offset: (note_start - buffer_start) as usize,
                    data: MidiMessage::NoteOn {
                        key: note.key,
                        velocity: note.velocity,
                    },
                });
            }
            if note_end >= buffer_start && note_end < buffer_end {
                events.push(MidiEvent {
                    sample_offset: (note_end - buffer_start) as usize,
                    data: MidiMessage::NoteOff { key: note.key },
                });
            }
        }
        events.sort_by_key(|e| e.sample_offset);
    }
}

/// Sample a waveform at a specific position using dasp interpolation.
/// Handles fallback from 1-channel to 2-channel stereo.
#[inline(always)]
fn sample_waveform_dasp(waveform: &AudioWaveform, pos: f64, src_channels: usize) -> [f32; 2] {
    let idx = pos as usize;
    let alpha = (pos - (idx as f64)) as f32;

    if src_channels == 2 {
        let frames: &[[f32; 2]] = slice::from_sample_slice(&waveform.buffer).unwrap_or(&[]);
        if idx >= frames.len() {
            return [0.0, 0.0];
        }

        let curr = frames[idx];
        let next = if idx + 1 < frames.len() {
            frames[idx + 1]
        } else {
            curr
        };

        [
            curr[0] + (next[0] - curr[0]) * alpha,
            curr[1] + (next[1] - curr[1]) * alpha,
        ]
    } else {
        let frames: &[[f32; 1]] = slice::from_sample_slice(&waveform.buffer).unwrap_or(&[]);
        if idx >= frames.len() {
            return [0.0, 0.0];
        }

        let curr = frames[idx];
        let next = if idx + 1 < frames.len() {
            frames[idx + 1]
        } else {
            curr
        };

        let val = curr[0] + (next[0] - curr[0]) * alpha;
        [val, val]
    }
}
