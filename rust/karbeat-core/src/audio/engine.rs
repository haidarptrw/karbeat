// Copyright (C) 2026 Haidar Wibowo
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 3.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.

/* src/audio/engine.rs */

use dasp::slice;
use hashbrown::HashMap;
use rtrb::{Consumer, Producer};
use smallvec::SmallVec;
use std::sync::Arc;
use triple_buffer::Output;
use wide::f32x4;

use crate::{
    audio::{
        event::{
            BusAutomationEvent, GeneratorAutomationEvent, MasterAutomationEvent, PluginTarget,
            TrackAutomationEvent, TransportFeedback,
        },
        render_state::{
            AudioEffectInstance, AudioGeneratorInstance, AudioPluginState, AudioRenderState,
            SidechainRouteId,
        },
    },
    commands::{
        AudioCommand, AudioFeedback, EffectParameterSnapshot, EffectTarget,
        GeneratorParameterSnapshot,
    },
    core::project::{
        automation::AutomationTarget,
        mixer::{MixerChannel, RoutingNode},
        plugin::{MidiEvent, MidiMessage, ProcessContext},
        AudioTrack, AudioWaveform, Clip, DawSource, GeneratorId, GeneratorInstance, Pattern,
        PatternId, TrackId,
    },
    shared::id::*,
    utils::{apply_simd_mix, apply_simd_mix_gain, get_waveform_buffer},
};
use karbeat_utils::{audio::db_to_linear, math::hermite_interp};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackMode {
    Song,
    Pattern {
        pattern_id: PatternId,
        generator_id: GeneratorId,
    },
}

#[derive(Debug, Clone)]
pub struct SongPlaybackState {
    pub is_playing: bool,
    pub is_looping: bool,
    pub is_recording: bool,
    pub playhead_samples: u32,
    pub current_beat: usize,
    pub current_bar: usize,
    pub last_emitted_samples: u32,
}

impl Default for SongPlaybackState {
    fn default() -> Self {
        Self {
            is_playing: false,
            is_looping: false,
            is_recording: false,
            playhead_samples: 0,
            current_beat: 1,
            current_bar: 1,
            last_emitted_samples: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PatternPlaybackState {
    pub is_playing: bool,
    pub playhead_samples: u32,
    pub current_beat: usize,
    pub current_bar: usize,
    pub last_emitted_samples: u32,
}

impl Default for PatternPlaybackState {
    fn default() -> Self {
        Self {
            is_playing: false,
            playhead_samples: 0,
            current_beat: 1,
            current_bar: 1,
            last_emitted_samples: 0,
        }
    }
}

/// Simple delay ring buffer for Plugin Delay Compensation (PDC)
#[derive(Clone, Default)]
pub struct DelayLine {
    buffer: Vec<f32>,
    write_pos: usize,
    delay_samples: usize,
}

impl DelayLine {
    pub fn set_delay(&mut self, delay_samples: usize, channels: usize) {
        let required_len = delay_samples * channels;
        if self.delay_samples != delay_samples || self.buffer.len() != required_len {
            self.delay_samples = delay_samples;
            self.buffer.resize(required_len, 0.0);
            self.buffer.fill(0.0);
            self.write_pos = 0;
        }
    }

    #[inline(always)]
    pub fn process_block(&mut self, buffer: &mut [f32], channels: usize) {
        if self.delay_samples == 0 {
            return;
        }

        let buf_len = self.buffer.len();
        for i in (0..buffer.len()).step_by(channels) {
            for c in 0..channels {
                let delay_idx = self.write_pos + c;
                let out = self.buffer[delay_idx];
                self.buffer[delay_idx] = buffer[i + c];
                buffer[i + c] = out;
            }
            self.write_pos = (self.write_pos + channels) % buf_len;
        }
    }
}

pub struct AudioEngine {
    // Comms
    state_consumer: Output<AudioRenderState>,
    position_producer: Producer<TransportFeedback>,
    feedback_producer: Producer<AudioFeedback>,
    current_state: AudioRenderState,

    // ======================================
    // Transport State (owned by audio thread)
    // ======================================
    bpm: f32,
    sample_rate: u32,
    num_channels: u16,

    song_state: SongPlaybackState,
    pattern_state: PatternPlaybackState,

    // Active Voices (lightweight references to plugins in plugin_state)
    active_generators: Vec<GeneratorVoice>,
    active_oneshots: Vec<AudioVoice>,
    preview_voices: Vec<PreviewVoice>,

    // Audio thread's owned plugins - NO locks required
    plugin_state: AudioPluginState,

    // Real-time Command Queue (UI → Audio)
    command_consumer: Consumer<AudioCommand>,

    mix_buffer: Vec<f32>,

    /// Intermediate buffers for each bus (for routing matrix)
    bus_buffers: HashMap<BusId, Vec<f32>>,

    /// Temporary buffer for bus processing (avoids allocation in audio thread)
    bus_temp_buffer: Vec<f32>,

    /// Cached routing order (updated only when state changes, not every callback)
    /// This routing is a unified Directed Acyclic Graph for all tracks and buses
    cached_routing_order: Vec<RoutingNode>,

    /// Song playback vs Pattern playback
    playback_mode: PlaybackMode,

    // =========================
    // Automation Event Queue
    // =========================
    track_automation_events: SmallVec<[(TrackId, Vec<TrackAutomationEvent>); 4]>,
    bus_automation_events: SmallVec<[(BusId, Vec<BusAutomationEvent>); 4]>,
    master_automation_events: SmallVec<[MasterAutomationEvent; 4]>,

    //////////////////////////////////////////////////
    /// Metronome state
    /////////////////////////////////////////////////
    metronome_state: MetronomeState,

    //////////////////////////////////////////////////
    /// Tail & Signal Flow Handling
    //////////////////////////////////////////////////
    track_tails: HashMap<TrackId, u32>,
    bus_tails: HashMap<BusId, u32>,
    master_tail: u32,
    /// Cleared every frame; tracks which nodes received audio this block
    node_has_signal: HashMap<RoutingNode, bool>,

    // ======================================
    // Plugin Delay Compensation (PDC)
    // ======================================
    compensation_delays: HashMap<RoutingNode, u32>,
    track_delay_lines: HashMap<TrackId, DelayLine>,
    bus_delay_lines: HashMap<BusId, DelayLine>,

    // =============== Auxiliary/Sidechain buffer
    aux_buffers: HashMap<SidechainRouteId, Vec<f32>>,
}

/// Lightweight voice reference - the actual plugin lives in AudioPluginState
pub struct GeneratorVoice {
    pub id: GeneratorId,
    pub track_id: TrackId,
    // Events queued for the CURRENT buffer block only
    pub midi_events: SmallVec<[MidiEvent; 4]>,
    pub automation_events: SmallVec<[GeneratorAutomationEvent; 4]>,
    // Track if this generator is persistent or temporary
    pub active: bool,
    pub playing_keys: Vec<u8>,
    //////////////////////////////////////////////////
    /// Tail Handling
    //////////////////////////////////////////////////
    pub tail_remaining: Option<u32>,
}

impl GeneratorVoice {
    pub fn new(id: GeneratorId, track_id: TrackId, active: bool) -> Self {
        Self {
            id,
            track_id,
            midi_events: SmallVec::new(),
            automation_events: SmallVec::new(),
            active,
            playing_keys: Vec::new(),
            tail_remaining: None,
        }
    }
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
    pub clip_elapsed_samples: u32,
    pub clip_loop_length: u32,
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

struct MetronomeState {
    is_active: bool,
    downbeat_buffer: Vec<f32>,
    offbeat_buffer: Vec<f32>,
    play_index: usize,
    is_playing: bool,
    is_downbeat: bool,
}

impl Default for MetronomeState {
    fn default() -> Self {
        let downbeat_bytes = include_bytes!("../../../../assets/audio/metronome_downbeat.wav");
        let offbeat_bytes = include_bytes!("../../../../assets/audio/metronome_offbeat.wav");

        Self {
            is_active: false,
            downbeat_buffer: load_internal_wav(downbeat_bytes),
            offbeat_buffer: load_internal_wav(offbeat_bytes),
            play_index: 0,
            is_playing: false,
            is_downbeat: true,
        }
    }
}

impl AudioEngine {
    pub fn new(
        state_consumer: Output<AudioRenderState>,
        command_consumer: Consumer<AudioCommand>,
        position_producer: Producer<TransportFeedback>,
        feedback_producer: Producer<AudioFeedback>,
        sample_rate: u32,
        num_channels: u16,
        initial_bpm: f32,
        initial_state: AudioRenderState,
    ) -> Self {
        let mix_buffer = Vec::with_capacity(2048);
        Self {
            state_consumer,
            command_consumer,
            position_producer,
            feedback_producer,
            current_state: initial_state,
            bpm: initial_bpm,
            sample_rate,
            num_channels,
            song_state: SongPlaybackState::default(),
            pattern_state: PatternPlaybackState::default(),
            active_generators: Vec::with_capacity(32),
            active_oneshots: Vec::with_capacity(16),
            preview_voices: Vec::with_capacity(4),
            plugin_state: AudioPluginState::default(),
            mix_buffer,
            bus_buffers: HashMap::new(),
            bus_temp_buffer: Vec::with_capacity(2048),
            cached_routing_order: Vec::new(),
            playback_mode: PlaybackMode::Song,
            track_automation_events: SmallVec::new(),
            bus_automation_events: SmallVec::new(),
            master_automation_events: SmallVec::new(),
            metronome_state: MetronomeState::default(),
            track_tails: HashMap::new(),
            bus_tails: HashMap::new(),
            master_tail: 0,
            node_has_signal: HashMap::new(),
            compensation_delays: HashMap::new(),
            track_delay_lines: HashMap::new(),
            bus_delay_lines: HashMap::new(),
            aux_buffers: HashMap::new(),
        }
    }

    pub fn plugin_state(&self) -> &AudioPluginState {
        &self.plugin_state
    }

    pub fn process(&mut self, output_buffer: &mut [f32]) {
        let mut pdc_dirty = false;
        // Sync graph state (transport no longer comes via triple buffer)
        if self.state_consumer.update() {
            let new_state = self.state_consumer.read().clone();

            // Update cached routing order only when state changes (not every callback)
            self.cached_routing_order = new_state.graph.mixer_state.get_routing_order();

            self.current_state = new_state;
            pdc_dirty = true;
        }

        // Process Commands (Play, Stop, Seek)
        while let Ok(cmd) = self.command_consumer.pop() {
            if self.process_command(cmd) {
                // Make process_command return bool
                pdc_dirty = true;
            }
        }

        // Fire recalculation if topology changed
        if pdc_dirty {
            self.recalculate_latencies();
        }

        // Clear Buffer
        output_buffer.fill(0.0);
        let channels = 2;
        let frame_count = output_buffer.len() / channels;

        // Transport Logic
        let is_currently_playing = match self.playback_mode {
            PlaybackMode::Song => self.song_state.is_playing,
            PlaybackMode::Pattern { .. } => self.pattern_state.is_playing,
        };

        if is_currently_playing {
            match self.playback_mode {
                PlaybackMode::Song => {
                    self.process_song_mode(frame_count, output_buffer, channels);
                }
                PlaybackMode::Pattern {
                    pattern_id,
                    generator_id,
                } => {
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
            self.render_voices_to_buffer(output_buffer, channels, false);
            self.cleanup_finished_voices(frame_count);
            self.emit_static_position();
        }

        // Always Render Previews (Metronome, Browser Preview)
        self.render_previews_to_buffer(output_buffer, channels);
    }

    fn advance_song_playhead(&mut self, frame_count: usize) {
        self.song_state.playhead_samples += frame_count as u32;
        self.recalculate_beat_bar();
        self.emit_playback_position();
        self.cleanup_finished_voices(frame_count);
    }

    fn advance_pattern_playhead(&mut self, frame_count: usize) {
        self.pattern_state.playhead_samples += frame_count as u32;
        self.recalculate_pattern_beat_bar();
        self.emit_playback_position();
        self.cleanup_finished_voices(frame_count);
    }

    /// Recalculates pattern beat/bar based on pattern_playhead_samples
    fn recalculate_pattern_beat_bar(&mut self) {
        let tempo = self.bpm;
        if tempo <= 0.0 {
            return;
        }

        let samples_per_beat = ((60.0 / tempo) * (self.sample_rate as f32)) as usize;
        if samples_per_beat == 0 {
            return;
        }

        // Pattern beat/bar are 1-indexed within the pattern
        self.pattern_state.current_beat =
            (self.pattern_state.playhead_samples as usize) / samples_per_beat + 1;
        self.pattern_state.current_bar = (self.pattern_state.current_beat - 1) / 4 + 1;
    }

    fn process_song_mode(
        &mut self,
        frame_count: usize,
        output_buffer: &mut [f32],
        channels: usize,
    ) {
        if self.song_state.playhead_samples > self.current_state.graph.max_sample_index {
            if self.song_state.is_looping {
                // Reset playhead back to 0 without changing `is_playing` state
                self.song_state.playhead_samples = 0;
                self.recalculate_beat_bar();
                self.song_state.last_emitted_samples = 0;

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

        // Apply Automation (parameter modulation)
        self.evaluate_automation_lanes();

        // Render Active Voices
        self.render_voices_to_buffer(output_buffer, channels, true);

        self.render_metronome(output_buffer, channels, self.song_state.playhead_samples);

        // Advance Playhead
        self.advance_song_playhead(buffer_size);
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
        if self
            .plugin_state
            .get_generator(generator_id.to_u32() as usize)
            .is_none()
        {
            log::warn!("Pattern preview: Generator {:?} not found", generator_id);
            self.stop_playback();
            return;
        }

        let tempo = self.bpm;
        let sample_rate = self.sample_rate as f32;

        let samples_per_beat = (60.0 / tempo) * sample_rate;
        let loop_len_samples = (((pattern.length_ticks as f32) / 960.0) * samples_per_beat) as u32;

        if loop_len_samples == 0 {
            return;
        }

        // Use PATTERN playhead (independent from song)
        if self.pattern_state.playhead_samples >= loop_len_samples {
            self.pattern_state.playhead_samples = 0;
            self.pattern_state.last_emitted_samples = 0;

            // This safely clears tracked keys to prevent hang on pattern loop
            Self::stop_all_active_generators_impl(
                &mut self.active_generators,
                &mut self.plugin_state,
                self.sample_rate,
            );
        }

        let start_time = self.pattern_state.playhead_samples;
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
                    .get_generator(generator_id.to_u32() as usize)
                    .map(|g| g.track_id)
                    .unwrap_or(TrackId::from(0));

                self.active_generators
                    .push(GeneratorVoice::new(generator_id, track_id, true));
                self.active_generators.len() - 1
            });

        let gen_voice = &mut self.active_generators[voice_idx];
        gen_voice.active = true;

        Self::schedule_pattern_notes_raw(
            &mut gen_voice.midi_events,
            &pattern.notes,
            self.sample_rate,
            tempo,
            start_time,
            end_time,
        );

        gen_voice.midi_events.sort_by_key(|e| e.sample_offset);

        // Render voices to buffer
        self.render_voices_to_buffer(output_buffer, channels, true);

        self.render_metronome(output_buffer, channels, start_time);

        // Advance PATTERN playhead (not song playhead)
        self.advance_pattern_playhead(frame_count);
    }

    /// Stop and reset the playhead to 0
    fn stop_playback(&mut self) {
        self.song_state.is_playing = false;
        self.pattern_state.is_playing = false;
        self.stop_all_active_generators();
        self.stop_all_automation_events();
        self.reset_playhead();
    }

    fn stop_all_active_generators(&mut self) {
        Self::stop_all_active_generators_impl(
            &mut self.active_generators,
            &mut self.plugin_state,
            self.sample_rate,
        );
    }

    fn stop_all_automation_events(&mut self) {
        self.track_automation_events.clear();
        self.bus_automation_events.clear();
        self.master_automation_events.clear();
    }

    fn stop_all_active_generators_impl(
        active_generators: &mut Vec<GeneratorVoice>,
        plugin_state: &mut AudioPluginState,
        sample_rate: u32,
    ) {
        for voice in active_generators.iter_mut() {
            for &key in &voice.playing_keys {
                voice.midi_events.push(MidiEvent {
                    sample_offset: 0,
                    data: MidiMessage::NoteOff { key },
                });
            }

            voice.playing_keys.clear();

            // Query plugin for its tail length
            if let Some(gen_instance) = plugin_state.get_generator(voice.id.to_u32() as usize) {
                // clamp tail to save CPU because who the hell is gonna have more than 20 seconds of reverb tail?
                let tail = gen_instance.plugin.tail_samples().min(20 * sample_rate);
                voice.tail_remaining = Some(tail);
            }
        }
    }

    /// Process incoming commands from command queue buffer
    fn process_command(&mut self, cmd: AudioCommand) -> bool {
        let mut pdc_dirty = false;
        match cmd {
            AudioCommand::PlayOneShot(waveform) => {
                self.preview_voices.clear();
                self.preview_voices.push(PreviewVoice::new(waveform, 1.0));
            }
            AudioCommand::StopAllPreviews => self.preview_voices.clear(),
            AudioCommand::SetPlaying(val) => {
                let was_playing = match self.playback_mode {
                    PlaybackMode::Song => self.song_state.is_playing,
                    PlaybackMode::Pattern { .. } => self.pattern_state.is_playing,
                };

                if was_playing && !val {
                    // Stopping: silence all active generators
                    self.stop_all_active_generators();
                }

                match self.playback_mode {
                    PlaybackMode::Song => self.song_state.is_playing = val,
                    PlaybackMode::Pattern { .. } => self.pattern_state.is_playing = val,
                }

                self.emit_current_playback_position();
            }
            AudioCommand::SetLooping(val) => {
                self.song_state.is_looping = val;
                self.emit_current_playback_position();
            }
            AudioCommand::StopAndReset => {
                if matches!(self.playback_mode, PlaybackMode::Song) {
                    self.stop_playback();
                }
            }
            AudioCommand::SetPlayhead(samples) => {
                log::info!("[AudioEngine] Seek: {}", samples);
                self.stop_all_active_generators();
                self.song_state.playhead_samples = samples;
                self.recalculate_beat_bar();
                self.song_state.last_emitted_samples = self.song_state.playhead_samples;
                self.emit_current_playback_position();
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
                self.trigger_live_note(generator_id, note_key, velocity, is_note_on);
            }
            AudioCommand::SetBPM(bpm) => {
                self.bpm = bpm;
                self.emit_current_playback_position();
            }
            AudioCommand::SetPlaybackMode(playback_mode) => {
                // Silence everything to prevent hanging notes from the previous mode
                self.stop_all_active_generators();

                // Reset the specific playhead for the new mode
                match (self.playback_mode, playback_mode) {
                    (PlaybackMode::Song, PlaybackMode::Pattern { .. }) => {
                        self.pattern_state.playhead_samples = 0;
                        self.pattern_state.last_emitted_samples = 0;
                        self.recalculate_pattern_beat_bar();
                        self.pattern_state.is_playing = true;
                    }
                    (PlaybackMode::Pattern { .. }, PlaybackMode::Song) => {
                        self.pattern_state.playhead_samples = 0;
                        self.pattern_state.last_emitted_samples = 0;
                        self.recalculate_pattern_beat_bar();
                        self.pattern_state.is_playing = false;
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
                plugin.prepare(
                    self.sample_rate as f32,
                    self.num_channels as usize,
                    buf_size,
                );

                let id_index = generator_id.to_u32() as usize;

                self.plugin_state.insert_generator(
                    id_index,
                    AudioGeneratorInstance {
                        id: generator_id,
                        track_id,
                        plugin,
                    },
                );
                pdc_dirty = true;
                log::info!(
                    "[AudioEngine] Added generator {:?} for track {:?}",
                    generator_id,
                    track_id
                );
            }
            AudioCommand::RemoveGenerator { generator_id } => {
                let id_index = generator_id.to_u32() as usize;
                self.plugin_state.remove_generator(id_index);
                // Also remove any active voice referencing it
                self.active_generators.retain(|v| v.id != generator_id);
                pdc_dirty = true;
                log::info!("[AudioEngine] Removed generator {:?}", generator_id);
            }
            AudioCommand::SetGeneratorParameter {
                generator_id,
                param_id,
                value,
            } => {
                if let Some(gen_instance) = self
                    .plugin_state
                    .get_generator_mut(generator_id.to_u32() as usize)
                {
                    gen_instance.plugin.set_parameter(param_id, value);
                }

                // This should Send updated param specifications to ApplicationState in business logic thread
                // Since they are also needs to be updated to reflect this change.
                // However because the logic in FFI assume that this is handled, we don't have to do it
            }
            AudioCommand::UpdateGeneratorTrack {
                generator_id,
                track_id,
            } => {
                if let Some(gen_instance) = self
                    .plugin_state
                    .get_generator_mut(generator_id.to_u32() as usize)
                {
                    gen_instance.track_id = track_id;
                }
                // Update active voice track association
                for voice in &mut self.active_generators {
                    if voice.id == generator_id {
                        voice.track_id = track_id;
                    }
                }

                // This should Send updated param specifications to ApplicationState in business logic thread
                // Since they are also needs to be updated to reflect this change.
                // However because the logic in FFI assume that this is handled, we don't have to do it
            }
            AudioCommand::AddEffect {
                target,
                effect_id,
                mut effect,
            } => {
                let buf_size = self.current_state.graph.buffer_size.max(512);
                effect.prepare(
                    self.sample_rate as f32,
                    self.num_channels as usize,
                    buf_size,
                );
                let instance = AudioEffectInstance {
                    id: effect_id,
                    plugin: effect,
                };
                match target {
                    EffectTarget::Track(track_id) => {
                        self.plugin_state
                            .add_track_effect(track_id.to_u32() as usize, instance);
                        log::info!("[AudioEngine] Added effect to track {:?}", track_id);
                    }
                    EffectTarget::Bus(bus_id) => {
                        self.plugin_state
                            .add_bus_effect(bus_id.to_u32() as usize, instance);
                        log::info!(
                            "[AudioEngine] Added effect {:?} to bus {:?}",
                            effect_id,
                            bus_id
                        );
                    }
                    EffectTarget::Master => {
                        self.plugin_state.master_effects.push(instance);
                        log::info!("[AudioEngine] Added effect {:?} to master", effect_id);
                    }
                }
            }
            AudioCommand::RemoveEffect { target, effect_id } => match target {
                EffectTarget::Track(track_id) => {
                    if let Some(effects) = self
                        .plugin_state
                        .get_track_effects_mut(track_id.to_u32() as usize)
                    {
                        if let Some(pos) = effects.iter().position(|e| e.id == effect_id) {
                            effects.remove(pos);
                        }
                    }
                }
                EffectTarget::Bus(bus_id) => {
                    if let Some(effects) = self
                        .plugin_state
                        .get_bus_effects_mut(bus_id.to_u32() as usize)
                    {
                        if let Some(pos) = effects.iter().position(|e| e.id == effect_id) {
                            effects.remove(pos);
                        }
                    }
                }
                EffectTarget::Master => {
                    if let Some(pos) = self
                        .plugin_state
                        .master_effects
                        .iter()
                        .position(|e| e.id == effect_id)
                    {
                        self.plugin_state.master_effects.remove(pos);
                    }
                }
            },
            AudioCommand::SetEffectParameter {
                target,
                effect_id,
                param_id,
                value,
            } => match target {
                EffectTarget::Track(track_id) => {
                    if let Some(effects) = self
                        .plugin_state
                        .get_track_effects_mut(track_id.to_u32() as usize)
                    {
                        if let Some(effect) = effects.iter_mut().find(|e| e.id == effect_id) {
                            effect.plugin.set_parameter(param_id, value);
                        }
                    }
                }
                EffectTarget::Bus(bus_id) => {
                    if let Some(effects) = self
                        .plugin_state
                        .get_bus_effects_mut(bus_id.to_u32() as usize)
                    {
                        if let Some(effect) = effects.iter_mut().find(|e| e.id == effect_id) {
                            effect.plugin.set_parameter(param_id, value);
                        }
                    }
                }
                EffectTarget::Master => {
                    if let Some(effect) = self
                        .plugin_state
                        .master_effects
                        .iter_mut()
                        .find(|e| e.id == effect_id)
                    {
                        effect.plugin.set_parameter(param_id, value);
                    }
                }
            },
            AudioCommand::QueryGeneratorParameters { generator_id } => {
                // Get all parameter values from the generator and send back
                if let Some(gen_instance) = self
                    .plugin_state
                    .get_generator(generator_id.to_u32() as usize)
                {
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

            AudioCommand::AddBus { bus_id, name } => {
                // Initialize bus buffer and effects chain
                let id_index = bus_id.to_u32() as usize;
                self.plugin_state.add_bus(id_index);
                self.bus_buffers.insert(bus_id, Vec::new());
                log::info!("[AudioEngine] Added bus {:?} ({})", bus_id, name);
            }
            AudioCommand::RemoveBus { bus_id } => {
                let id_index = bus_id.to_u32() as usize;
                self.plugin_state.remove_bus(id_index);
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

            AudioCommand::UpdateRouting { routing } => {
                // Routing is stored in mixer_state and synced via triple buffer
                // Log for debugging
                log::info!(
                    "[AudioEngine] Received routing update with {} connections",
                    routing.len()
                );

                pdc_dirty = true;
            }
            AudioCommand::QueryEffectParameters { target, effect_id } => {
                let effect_instance_opt = match &target {
                    EffectTarget::Track(track_id) => self
                        .plugin_state
                        .get_track_effects(track_id.to_u32() as usize)
                        .and_then(|effects| effects.iter().find(|e| e.id == effect_id)),
                    EffectTarget::Bus(bus_id) => self
                        .plugin_state
                        .get_bus_effects(bus_id.to_u32() as usize)
                        .and_then(|effects| effects.iter().find(|e| e.id == effect_id)),
                    EffectTarget::Master => self
                        .plugin_state
                        .master_effects
                        .iter()
                        .find(|e| e.id == effect_id),
                };

                if let Some(effect_instance) = effect_instance_opt {
                    let specs = effect_instance.plugin.get_parameter_specs();
                    let parameters: Vec<(u32, f32)> = specs
                        .iter()
                        .map(|spec| (spec.id, effect_instance.plugin.get_parameter(spec.id)))
                        .collect();

                    let snapshot = EffectParameterSnapshot {
                        target,
                        effect_id,
                        parameters,
                    };

                    let _ = self
                        .feedback_producer
                        .push(AudioFeedback::EffectParameterSnapshot(snapshot));
                }
            }
            AudioCommand::PreparePlugin {
                track_effects,
                master_effects,
                bus_effects,
                generators,
            } => {
                let buf_size = self.current_state.graph.buffer_size.max(512);
                let sample_rate = self.sample_rate as f32;
                let channels = self.num_channels as usize;

                // Completely clear the previous project's plugin state, voices, and tails
                self.plugin_state.generators.clear();
                self.plugin_state.track_effects.clear();
                self.plugin_state.master_effects.clear();
                self.plugin_state.bus_effects.clear();
                self.active_generators.clear();
                self.bus_buffers.clear();
                self.track_tails.clear();
                self.bus_tails.clear();
                self.master_tail = 0;

                // Batch load Generators
                for (gen_id, mut plugin) in generators.into_iter() {
                    plugin.prepare(sample_rate, channels, buf_size);

                    // Since PreparePlugin doesn't pass track_ids directly, we find the
                    // associated track from the newly synced current_state graph.
                    let track_id = self
                        .current_state
                        .graph
                        .tracks
                        .iter()
                        .find(|t| t.generator.as_ref().map_or(false, |g| g.id == gen_id))
                        .map(|t| t.id)
                        .unwrap_or_else(|| TrackId::from(0));

                    self.plugin_state.insert_generator(
                        gen_id.to_u32() as usize,
                        AudioGeneratorInstance {
                            id: gen_id,
                            track_id,
                            plugin,
                        },
                    );
                }

                // Batch load Track Effects
                for (track_id, effects_map) in track_effects.into_iter() {
                    for (effect_id, mut plugin) in effects_map.into_iter() {
                        plugin.prepare(sample_rate, channels, buf_size);
                        self.plugin_state.add_track_effect(
                            track_id.to_u32() as usize,
                            AudioEffectInstance {
                                id: effect_id,
                                plugin,
                            },
                        );
                    }
                }

                // Batch load Bus Effects & Initialize Bus Buffers
                for (bus_id, effects_map) in bus_effects.into_iter() {
                    let bus_id_index = bus_id.to_u32() as usize;
                    self.plugin_state.add_bus(bus_id_index);
                    self.bus_buffers.insert(bus_id, Vec::new());

                    for (effect_id, mut plugin) in effects_map.into_iter() {
                        plugin.prepare(sample_rate, channels, buf_size);
                        self.plugin_state.add_bus_effect(
                            bus_id_index,
                            AudioEffectInstance {
                                id: effect_id,
                                plugin,
                            },
                        );
                    }
                }

                // Ensure all buses present in the mixer state have buffers allocated,
                // even if they don't have any effects loaded on them yet.
                for &bus_id in self.current_state.graph.mixer_state.buses.keys() {
                    if !self.bus_buffers.contains_key(&bus_id) {
                        self.plugin_state.add_bus(bus_id.to_u32() as usize);
                        self.bus_buffers.insert(bus_id, Vec::new());
                    }
                }

                // Batch load Master Effects
                for (effect_id, mut plugin) in master_effects.into_iter() {
                    plugin.prepare(sample_rate, channels, buf_size);
                    self.plugin_state.master_effects.push(AudioEffectInstance {
                        id: effect_id,
                        plugin,
                    });
                }

                log::info!("[AudioEngine] Prepared all plugins for the newly loaded project.");
            }
            AudioCommand::SetMetronomeActive(active) => {
                self.metronome_state.is_active = active;
                log::info!("[AudioEngine] Metronome Active: {}", active);
            }
            AudioCommand::TogglePlayingWithPlaybackMode(playback_mode) => {
                // check if the mode is same as before. if it is, then check the is playing
                if self.playback_mode == playback_mode {
                    let is_currently_playing = match self.playback_mode {
                        PlaybackMode::Song => self.song_state.is_playing,
                        PlaybackMode::Pattern { .. } => self.pattern_state.is_playing,
                    };
                    if is_currently_playing {
                        match self.playback_mode {
                            PlaybackMode::Song => self.song_state.is_playing = false,
                            PlaybackMode::Pattern { .. } => self.pattern_state.is_playing = false,
                        }
                        self.stop_all_active_generators();
                    } else {
                        match self.playback_mode {
                            PlaybackMode::Song => self.song_state.is_playing = true,
                            PlaybackMode::Pattern { .. } => self.pattern_state.is_playing = true,
                        }
                    }
                } else {
                    // stop and reset first when the playback mode changes
                    self.stop_playback();
                    self.playback_mode = playback_mode;
                    match self.playback_mode {
                        PlaybackMode::Song => self.song_state.is_playing = true,
                        PlaybackMode::Pattern { .. } => {
                            self.pattern_state.playhead_samples = 0;
                            self.pattern_state.last_emitted_samples = 0;
                            self.recalculate_pattern_beat_bar();
                            self.pattern_state.is_playing = true;
                        }
                    }
                }
                self.emit_current_playback_position();
            }
            AudioCommand::TogglePatternPlayback {
                pattern_id,
                generator_id,
            } => {
                let is_pattern_mode = matches!(self.playback_mode, PlaybackMode::Pattern { .. });

                if !is_pattern_mode {
                    // If not in pattern mode: Stop song, change mode, and immediately play from start
                    self.stop_playback();
                    self.playback_mode = PlaybackMode::Pattern {
                        pattern_id,
                        generator_id,
                    };
                    self.pattern_state.playhead_samples = 0;
                    self.pattern_state.last_emitted_samples = 0;
                    self.recalculate_pattern_beat_bar();
                    self.pattern_state.is_playing = true;
                } else {
                    // Else we check current state
                    if self.pattern_state.is_playing {
                        // If it is playing: stop, silence voices, and go to first tick
                        self.pattern_state.is_playing = false;
                        self.stop_all_active_generators();

                        self.pattern_state.playhead_samples = 0;
                        self.pattern_state.last_emitted_samples = 0;
                        self.recalculate_pattern_beat_bar();
                    } else {
                        // If it is stopped: just play
                        self.pattern_state.is_playing = true;
                    }
                }

                self.emit_current_playback_position();
            }
            AudioCommand::SwitchPatternGenerator(new_gen_id) => {
                if let PlaybackMode::Pattern { generator_id, .. } = &mut self.playback_mode {
                    if *generator_id != new_gen_id {
                        // Silence the old generator so ADSR tails/notes don't hang forever
                        if let Some(old_voice) = self
                            .active_generators
                            .iter_mut()
                            .find(|g| g.id == *generator_id)
                        {
                            if let Some(gen_instance) = self
                                .plugin_state
                                .get_generator_mut(old_voice.id.to_u32() as usize)
                            {
                                gen_instance.plugin.reset();
                            }
                            old_voice.midi_events.clear();
                            old_voice.playing_keys.clear();
                        }

                        // Hot-swap the ID. The `process_pattern_mode` function will
                        // automatically route the next batch of MIDI notes to the new generator.
                        *generator_id = new_gen_id;
                    }
                }
            }
            AudioCommand::ExecutePluginCommand {
                target,
                command,
                payload,
                request_id,
            } => {
                let response = match target {
                    PluginTarget::Generator(generator_id) => self
                        .plugin_state
                        .get_generator_mut(generator_id.to_u32() as usize)
                        .and_then(|instance| {
                            instance.plugin.execute_custom_command(&command, &payload)
                        }),
                    PluginTarget::TrackEffect(track_id, effect_id) => self
                        .plugin_state
                        .get_track_effects_mut(track_id.to_u32() as usize)
                        .and_then(|effects| effects.iter_mut().find(|e| e.id == effect_id))
                        .and_then(|instance| {
                            instance.plugin.execute_custom_command(&command, &payload)
                        }),
                    PluginTarget::BusEffect(bus_id, effect_id) => self
                        .plugin_state
                        .get_bus_effects_mut(bus_id.to_u32() as usize)
                        .and_then(|effects| effects.iter_mut().find(|e| e.id == effect_id))
                        .and_then(|instance| {
                            instance.plugin.execute_custom_command(&command, &payload)
                        }),
                    PluginTarget::MasterEffect(effect_id) => self
                        .plugin_state
                        .master_effects
                        .iter_mut()
                        .find(|e| e.id == effect_id)
                        .and_then(|instance| {
                            instance.plugin.execute_custom_command(&command, &payload)
                        }),
                };
                // If the plugin returned a response, send it back to the UI
                if let Some(res) = response {
                    let _ = self
                        .feedback_producer
                        .push(AudioFeedback::PluginCommandResponse {
                            request_id,
                            response: res,
                        });
                }
            }
            AudioCommand::SetPluginState { target, state } => match target {
                PluginTarget::Generator(id) => {
                    if let Some(instance) =
                        self.plugin_state.get_generator_mut(id.to_u32() as usize)
                    {
                        instance.plugin.set_state(&state);
                    }
                }
                PluginTarget::TrackEffect(track_id, effect_id) => {
                    if let Some(effects) = self
                        .plugin_state
                        .get_track_effects_mut(track_id.to_u32() as usize)
                    {
                        if let Some(instance) = effects.iter_mut().find(|e| e.id == effect_id) {
                            instance.plugin.set_state(&state);
                        }
                    }
                }
                PluginTarget::BusEffect(bus_id, effect_id) => {
                    if let Some(effects) = self
                        .plugin_state
                        .get_bus_effects_mut(bus_id.to_u32() as usize)
                    {
                        if let Some(instance) = effects.iter_mut().find(|e| e.id == effect_id) {
                            instance.plugin.set_state(&state);
                        }
                    }
                }
                PluginTarget::MasterEffect(effect_id) => {
                    if let Some(instance) = self
                        .plugin_state
                        .master_effects
                        .iter_mut()
                        .find(|e| e.id == effect_id)
                    {
                        instance.plugin.set_state(&state);
                    }
                }
            },
            AudioCommand::QueryPluginState { target, request_id } => {
                let state_blob = match target {
                    PluginTarget::Generator(id) => self
                        .plugin_state
                        .get_generator(id.to_u32() as usize)
                        .map(|i| i.plugin.get_state()),
                    PluginTarget::TrackEffect(track_id, effect_id) => self
                        .plugin_state
                        .get_track_effects(track_id.to_u32() as usize)
                        .and_then(|effects| effects.iter().find(|e| e.id == effect_id))
                        .map(|i| i.plugin.get_state()),
                    PluginTarget::BusEffect(bus_id, effect_id) => self
                        .plugin_state
                        .get_bus_effects(bus_id.to_u32() as usize)
                        .and_then(|effects| effects.iter().find(|e| e.id == effect_id))
                        .map(|i| i.plugin.get_state()),
                    PluginTarget::MasterEffect(effect_id) => self
                        .plugin_state
                        .master_effects
                        .iter()
                        .find(|e| e.id == effect_id)
                        .map(|i| i.plugin.get_state()),
                };

                // If we successfully got the state, send it back to the UI
                if let Some(state) = state_blob {
                    let _ = self
                        .feedback_producer
                        .push(AudioFeedback::PluginStateSnapshot {
                            target,
                            state,
                            request_id,
                        });
                }
            }

            AudioCommand::QueryZeroCopyBuffer {target, name, request_id}  => {
                let buffer_opt = match target {
                    PluginTarget::Generator(id) => self
                        .plugin_state
                        .get_generator(id.to_u32() as usize)
                        .and_then(|i| i.plugin.get_zero_copy_buffer(&name)),
                    PluginTarget::TrackEffect(track_id, effect_id) => self
                        .plugin_state
                        .get_track_effects(track_id.to_u32() as usize)
                        .and_then(|effects| effects.iter().find(|e| e.id == effect_id))
                        .and_then(|i| i.plugin.get_zero_copy_buffer(&name)),
                    PluginTarget::BusEffect(bus_id, effect_id) => self
                        .plugin_state
                        .get_bus_effects(bus_id.to_u32() as usize)
                        .and_then(|effects| effects.iter().find(|e| e.id == effect_id))
                        .and_then(|i| i.plugin.get_zero_copy_buffer(&name)),
                    PluginTarget::MasterEffect(effect_id) => self
                        .plugin_state
                        .master_effects
                        .iter()
                        .find(|e| e.id == effect_id)
                        .and_then(|i| i.plugin.get_zero_copy_buffer(&name)),
                };

                let _ = self
                    .feedback_producer
                    .push(AudioFeedback::ZeroCopyBufferResponse {
                        request_id,
                        buffer: buffer_opt,
                    });
            }
            
        }

        pdc_dirty
    }

    /// Recalculates current Beat and Bar based on playhead_samples
    /// Uses 1-based indexing for musical time.
    fn recalculate_beat_bar(&mut self) {
        let tempo = self.bpm;
        if tempo <= 0.0 {
            return;
        }

        let samples_per_beat = ((60.0 / tempo) * (self.sample_rate as f32)) as usize;
        if samples_per_beat == 0 {
            return;
        }

        self.song_state.current_beat =
            (self.song_state.playhead_samples as usize) / samples_per_beat + 1;
        self.song_state.current_bar = (self.song_state.current_beat - 1) / 4 + 1;
    }

    fn reset_playhead(&mut self) {
        log::info!("[AudioEngine] Reset Playhead");
        self.song_state.playhead_samples = 0;
        self.song_state.current_beat = 1;
        self.song_state.current_bar = 1;
        self.song_state.last_emitted_samples = 0;
        self.emit_static_position();
    }

    fn emit_playback_position(&mut self) {
        let emission_interval = self.sample_rate / 60; // ~60fps
        let (current, last) = match self.playback_mode {
            PlaybackMode::Song => (
                self.song_state.playhead_samples,
                self.song_state.last_emitted_samples,
            ),
            PlaybackMode::Pattern { .. } => (
                self.pattern_state.playhead_samples,
                self.pattern_state.last_emitted_samples,
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
                    self.song_state.last_emitted_samples = self.song_state.playhead_samples;
                }
                PlaybackMode::Pattern { .. } => {
                    self.pattern_state.last_emitted_samples = self.pattern_state.playhead_samples;
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

    fn build_position_struct(&self, is_playing: Option<bool>) -> TransportFeedback {
        let is_playing = is_playing.unwrap_or_else(|| match self.playback_mode {
            PlaybackMode::Song => self.song_state.is_playing,
            PlaybackMode::Pattern { .. } => self.pattern_state.is_playing,
        });
        let is_pattern_mode = matches!(self.playback_mode, PlaybackMode::Pattern { .. });

        let ticks = if self.bpm > 0.0 && self.sample_rate > 0 {
            (self.song_state.playhead_samples as f64
                * (self.bpm as f64 / 60.0)
                * (960.0 / self.sample_rate as f64)) as u32
        } else {
            0
        };

        let pattern_ticks = if self.bpm > 0.0 && self.sample_rate > 0 {
            (self.pattern_state.playhead_samples as f64
                * (self.bpm as f64 / 60.0)
                * (960.0 / self.sample_rate as f64)) as u32
        } else {
            0
        };

        TransportFeedback {
            // Song position
            samples: self.song_state.playhead_samples,
            ticks,
            beat: self.song_state.current_beat,
            bar: self.song_state.current_bar,
            tempo: self.bpm,
            sample_rate: self.current_state.graph.sample_rate,
            // Transport state
            is_playing,
            is_looping: self.song_state.is_looping,
            is_recording: self.song_state.is_recording,
            is_pattern_playing: self.pattern_state.is_playing,
            // Pattern position (independent)
            is_pattern_mode,
            pattern_samples: self.pattern_state.playhead_samples,
            pattern_ticks,
            pattern_beat: self.pattern_state.current_beat,
            pattern_bar: self.pattern_state.current_bar,
        }
    }

    fn emit_current_playback_position(&mut self) {
        if !self.position_producer.is_full() {
            let _ = self
                .position_producer
                .push(self.build_position_struct(None));
        }
    }

    fn cleanup_finished_voices(&mut self, frame_count: usize) {
        // Generators stay alive (persistent), just clear their MIDI events for the next frame
        for gen_voice in self.active_generators.iter_mut() {
            // DYNAMICALLY UPDATE PLAYING KEYS based on what just happened in this audio block
            for event in &gen_voice.midi_events {
                match event.data {
                    MidiMessage::NoteOn { key, velocity } => {
                        if velocity > 0 {
                            if !gen_voice.playing_keys.contains(&key) {
                                gen_voice.playing_keys.push(key);
                            }
                        } else {
                            gen_voice.playing_keys.retain(|&k| k != key);
                        }
                    }
                    MidiMessage::NoteOff { key } => {
                        gen_voice.playing_keys.retain(|&k| k != key);
                    }
                    _ => {}
                }
            }

            // Now it's safe to clear events for the next block
            gen_voice.midi_events.clear();
            gen_voice.automation_events.clear();

            // SAFE TAIL HANDLING
            if gen_voice.playing_keys.is_empty() {
                // Initialize the tail tracker if normal playback just ended a note
                if gen_voice.tail_remaining.is_none() {
                    if let Some(gen_instance) = self
                        .plugin_state
                        .get_generator(gen_voice.id.to_u32() as usize)
                    {
                        let tail = gen_instance
                            .plugin
                            .tail_samples()
                            .min(20 * self.sample_rate);
                        gen_voice.tail_remaining = Some(tail);
                    }
                }

                // Decrement the tail
                if let Some(tail) = gen_voice.tail_remaining {
                    let new_tail = tail.saturating_sub(frame_count as u32);

                    if new_tail == 0 {
                        if let Some(gen_instance) = self
                            .plugin_state
                            .get_generator_mut(gen_voice.id.to_u32() as usize)
                        {
                            gen_instance.plugin.reset();
                        }

                        // clear the tail and flag for culling
                        gen_voice.tail_remaining = None;
                        gen_voice.active = false;
                    } else {
                        gen_voice.tail_remaining = Some(new_tail);
                    }
                }
            } else {
                // If a new key is pressed, abort any lingering tail countdown
                gen_voice.tail_remaining = None;
            }
        }

        self.active_generators.retain(|g| g.active);

        self.track_automation_events
            .iter_mut()
            .for_each(|(_, v)| v.clear());
        self.bus_automation_events
            .iter_mut()
            .for_each(|(_, v)| v.clear());
        self.master_automation_events.clear();

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

                gen_voice.midi_events.push(MidiEvent {
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
        if let Some(gen_instance) = self
            .plugin_state
            .get_generator(generator_id.to_u32() as usize)
        {
            let track_id = gen_instance.track_id;

            // Find or create voice
            let voice_idx = self
                .active_generators
                .iter()
                .position(|g| g.id == generator_id)
                .unwrap_or_else(|| {
                    self.active_generators
                        .push(GeneratorVoice::new(generator_id, track_id, true));
                    self.active_generators.len() - 1
                });

            let gen_voice = &mut self.active_generators[voice_idx];
            let message = if is_on {
                MidiMessage::NoteOn { key, velocity }
            } else {
                MidiMessage::NoteOff { key }
            };

            gen_voice.midi_events.push(MidiEvent {
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

    fn render_voices_to_buffer(&mut self, output: &mut [f32], channels: usize, is_playing: bool) {
        let buf_len = output.len();

        // Clear signal flow tracker for this new frame
        self.node_has_signal.clear();

        // ======================================
        // Pre-copy transport state for ProcessContext construction
        // (must happen before any borrows of self fields are taken)
        // ======================================
        let bpm = self.bpm;
        let sample_position = match self.playback_mode {
            PlaybackMode::Song => self.song_state.playhead_samples as u64,
            PlaybackMode::Pattern { .. } => self.pattern_state.playhead_samples as u64,
        };

        // Ensure bus buffers are properly sized
        for (_bus_id, buf) in self.bus_buffers.iter_mut() {
            if buf.len() != buf_len {
                buf.resize(buf_len, 0.0);
            }
            buf.fill(0.0);
        }

        // Ensure aux (sidechain) buffers are properly sized and cleared
        for (_aux_id, buf) in self.aux_buffers.iter_mut() {
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

        // Iterate through tracks, buses, and master in topological order
        for node in self.cached_routing_order.clone().iter() {
            match node {
                RoutingNode::Track(track_id) => {
                    let default_channel = Arc::new(MixerChannel::default());

                    let mut channel = self
                        .current_state
                        .graph
                        .mixer_state
                        .channels
                        .get(track_id)
                        .cloned()
                        .unwrap_or(default_channel);

                    let channel_mut = Arc::make_mut(&mut channel);

                    // Check mute/solo
                    if channel_mut.mute {
                        continue;
                    }
                    if is_any_solo && !channel_mut.solo {
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
                        .find(|g| g.track_id == *track_id && g.active)
                    {
                        let gen_id = gen_voice.id;
                        let events = &gen_voice.midi_events;
                        let param_events = &gen_voice.automation_events;

                        if let Some(gen_instance) = self
                            .plugin_state
                            .get_generator_mut(gen_id.to_u32() as usize)
                        {
                            // CONSUME AUTOMATION
                            for ev in param_events {
                                #[allow(irrefutable_let_patterns)]
                                if let GeneratorAutomationEvent::PluginParam { param_id, value } =
                                    ev
                                {
                                    gen_instance.plugin.apply_automation(*param_id, *value);
                                }
                            }
                            // PROCESS AUDIO
                            // Build context for the generator — MIDI events are passed via ProcessContext
                            let gen_ctx = ProcessContext {
                                bpm,
                                time_sig_numerator: 4,
                                time_sig_denominator: 4,
                                is_playing,
                                sample_position,
                                midi_events: events,
                                aux_buffer: None, // TODO: handle this aux buffer with actual aux buffer
                            };
                            gen_instance.plugin.process(&mut self.mix_buffer, &gen_ctx);
                            has_signal = true;
                        }
                    }

                    // Audio Voice
                    if Self::render_oneshots(
                        &mut self.active_oneshots,
                        self.sample_rate,
                        *track_id,
                        &mut self.mix_buffer,
                        channels,
                    ) {
                        has_signal = true;
                    }

                    // track tail handling
                    let track_effects_tail = self
                        .plugin_state
                        .get_track_effects(track_id.to_u32() as usize)
                        .map(|effects| {
                            effects
                                .iter()
                                .map(|e| e.plugin.tail_samples())
                                .max()
                                .unwrap_or(0)
                        })
                        .unwrap_or(0);

                    if has_signal {
                        self.track_tails.insert(*track_id, track_effects_tail);
                    } else {
                        let current_tail = self.track_tails.get(track_id).copied().unwrap_or(0);
                        if current_tail > 0 {
                            let new_tail = current_tail.saturating_sub(buf_len as u32);
                            self.track_tails.insert(*track_id, new_tail);
                            if new_tail == 0 {
                                if let Some(effects) = self
                                    .plugin_state
                                    .get_track_effects_mut(track_id.to_u32() as usize)
                                {
                                    effects.iter_mut().for_each(|e| e.plugin.reset());
                                }
                                continue;
                            }
                            // has_signal = true;
                        } else {
                            continue;
                        }
                    }

                    // Apply track mixer channel (volume/pan/phase) and effects
                    // Effects receive an empty MIDI slice — track routing is audio-only at this stage
                    let track_effect_ctx = ProcessContext {
                        bpm,
                        time_sig_numerator: 4,
                        time_sig_denominator: 4,
                        is_playing,
                        sample_position,
                        midi_events: &[],
                        aux_buffer: None, // TODO: handle this aux buffer with actual aux buffer
                    };
                    Self::apply_mixer_channel_with_effects(
                        channel_mut,
                        &mut self.plugin_state.track_effects,
                        &self.track_automation_events,
                        *track_id,
                        &mut self.mix_buffer,
                        channels,
                        &track_effect_ctx,
                    );

                    if let Some(delay_line) = self.track_delay_lines.get_mut(track_id) {
                        delay_line.process_block(&mut self.mix_buffer, channels);
                    }

                    // Route the track signal to destinations based on routing matrix
                    let track_routes: Vec<_> = routing
                        .iter()
                        .filter(|c| c.source == RoutingNode::Track(*track_id))
                        .collect();

                    if track_routes.is_empty() {
                        self.node_has_signal.insert(RoutingNode::Master, true);
                        apply_simd_mix(output, &self.mix_buffer);
                    } else {
                        // Route to each destination with appropriate send level
                        for conn in track_routes {
                            self.node_has_signal.insert(conn.destination, true);
                            match conn.destination {
                                RoutingNode::Master => {
                                    apply_simd_mix_gain(output, &self.mix_buffer, conn.send_level);
                                }
                                RoutingNode::Bus(bus_id) => {
                                    if let Some(bus_buf) = self.bus_buffers.get_mut(&bus_id) {
                                        apply_simd_mix_gain(
                                            bus_buf,
                                            &self.mix_buffer,
                                            conn.send_level,
                                        );
                                    }
                                }
                                RoutingNode::Track(_) => {
                                    // Invalid: can't route to a track
                                }
                            }
                        }
                    }
                }
                RoutingNode::Bus(bus_id) => {
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
                        .get_mut(bus_id)
                        .map(|b| Arc::make_mut(b));

                    let Some(bus_settings) = bus_channel else {
                        continue;
                    };

                    // Skip if muted
                    if bus_settings.channel.mute {
                        continue;
                    }

                    // ================= Bus Tail Handling ===================
                    let mut bus_has_signal = self
                        .node_has_signal
                        .get(&RoutingNode::Bus(*bus_id))
                        .copied()
                        .unwrap_or(false);
                    let bus_effects_tail = self
                        .plugin_state
                        .get_bus_effects(bus_id.to_u32() as usize)
                        .map(|effects| {
                            effects
                                .iter()
                                .map(|e| e.plugin.tail_samples())
                                .max()
                                .unwrap_or(0)
                        })
                        .unwrap_or(0);
                    if bus_has_signal {
                        self.bus_tails.insert(*bus_id, bus_effects_tail);
                    } else {
                        let current_tail = self.bus_tails.get(bus_id).copied().unwrap_or(0);
                        if current_tail > 0 {
                            let new_tail = current_tail.saturating_sub(buf_len as u32);
                            self.bus_tails.insert(*bus_id, new_tail);
                            if new_tail == 0 {
                                if let Some(effects) = self
                                    .plugin_state
                                    .get_bus_effects_mut(bus_id.to_u32() as usize)
                                {
                                    effects.iter_mut().for_each(|e| e.plugin.reset());
                                }
                                continue;
                            }
                            // Tail is ringing, keep bus alive
                            bus_has_signal = true;
                        } else {
                            // Since it has no input signal, no tail -> skip processing
                            continue;
                        }
                    }

                    // Copy to mix_buffer for processing
                    if self.mix_buffer.len() != buf_len {
                        self.mix_buffer.resize(buf_len, 0.0);
                    }
                    self.mix_buffer.copy_from_slice(&self.bus_temp_buffer);

                    let bus_volume = &mut bus_settings.channel.volume;
                    let bus_pan = &mut bus_settings.channel.pan;

                    // CONSUME BUS AUTOMATION
                    if let Some((_, auto_events)) = self
                        .bus_automation_events
                        .iter()
                        .find(|(id, _)| *id == *bus_id)
                    {
                        for event in auto_events {
                            match event {
                                BusAutomationEvent::Volume(v) => {
                                    bus_volume.apply_automation(*v);
                                }
                                BusAutomationEvent::Pan(v) => {
                                    bus_pan.apply_automation(*v);
                                }
                                BusAutomationEvent::PluginParam {
                                    effect_id,
                                    param_id,
                                    value,
                                } => {
                                    if let Some(effects) = self
                                        .plugin_state
                                        .get_bus_effects_mut(bus_id.to_u32() as usize)
                                    {
                                        if let Some(effect) =
                                            effects.iter_mut().find(|e| e.id == *effect_id)
                                        {
                                            effect.plugin.set_parameter(*param_id, *value);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Apply bus effects
                    if let Some(effects) = self
                        .plugin_state
                        .get_bus_effects_mut(bus_id.to_u32() as usize)
                    {
                        // Bus effects receive no MIDI — pure audio processing
                        let bus_effect_ctx = ProcessContext {
                            bpm,
                            time_sig_numerator: 4,
                            time_sig_denominator: 4,
                            is_playing,
                            sample_position,
                            midi_events: &[],
                            aux_buffer: None, // TODO: handle this aux buffer with actual aux buffer
                        };
                        for effect in effects.iter_mut() {
                            effect.plugin.process(&mut self.mix_buffer, &bus_effect_ctx);
                        }
                    }

                    // Apply PDC on Bus
                    if let Some(delay_line) = self.bus_delay_lines.get_mut(bus_id) {
                        delay_line.process_block(&mut self.mix_buffer, channels);
                    }

                    // Apply volume and pan (volume is stored in dB)
                    let volume = db_to_linear(bus_volume.get());
                    let pan = bus_pan.get();
                    let (left_gain, right_gain) = if channels == 2 {
                        let p = (pan + 1.0) * 0.5;
                        ((1.0 - p).sqrt() * volume, p.sqrt() * volume)
                    } else {
                        (volume, volume)
                    };

                    if channels == 2 {
                        let gain_v = f32x4::new([left_gain, right_gain, left_gain, right_gain]);
                        let mut iter = self.mix_buffer.chunks_exact_mut(4);

                        for chunk in iter.by_ref() {
                            let mut v = f32x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);
                            v *= gain_v;
                            chunk.copy_from_slice(&v.to_array());
                        }

                        // Remainder will be 0 or 2 floats (1 stereo frame)
                        for chunk in iter.into_remainder().chunks_exact_mut(2) {
                            chunk[0] *= left_gain;
                            chunk[1] *= right_gain;
                        }
                    } else {
                        let gain_v = f32x4::splat(left_gain);
                        let mut iter = self.mix_buffer.chunks_exact_mut(4);

                        for chunk in iter.by_ref() {
                            let mut v = f32x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);
                            v *= gain_v;
                            chunk.copy_from_slice(&v.to_array());
                        }

                        for s in iter.into_remainder() {
                            *s *= left_gain;
                        }
                    }

                    // Route bus output to destinations
                    let bus_routes: Vec<_> = routing
                        .iter()
                        .filter(|c| c.source == RoutingNode::Bus(*bus_id))
                        .collect();

                    for conn in bus_routes {
                        self.node_has_signal.insert(conn.destination, true);
                        match conn.destination {
                            RoutingNode::Master => {
                                apply_simd_mix_gain(output, &self.mix_buffer, conn.send_level);
                            }
                            RoutingNode::Bus(dest_bus_id) => {
                                if let Some(dest_buf) = self.bus_buffers.get_mut(&dest_bus_id) {
                                    apply_simd_mix_gain(
                                        dest_buf,
                                        &self.mix_buffer,
                                        conn.send_level,
                                    );
                                }
                            }
                            RoutingNode::Track(_) => {}
                        }
                    }
                }
                RoutingNode::Master => {
                    // TAIL HANDLING
                    let master_has_signal = self
                        .node_has_signal
                        .get(&RoutingNode::Master)
                        .copied()
                        .unwrap_or(false);
                    let master_effects_tail = self
                        .plugin_state
                        .master_effects
                        .iter()
                        .map(|e| e.plugin.tail_samples())
                        .max()
                        .unwrap_or(0);

                    if master_has_signal {
                        self.master_tail = master_effects_tail;
                    } else {
                        if self.master_tail > 0 {
                            self.master_tail = self.master_tail.saturating_sub(buf_len as u32);
                            // If the tail becomes 0, then master has finally gone silent!
                            if self.master_tail == 0 {
                                self.plugin_state
                                    .master_effects
                                    .iter_mut()
                                    .for_each(|e| e.plugin.reset());
                            }
                        }
                    }

                    if master_has_signal || self.master_tail > 0 {
                        let master_effect_ctx = ProcessContext {
                            bpm,
                            time_sig_numerator: 4,
                            time_sig_denominator: 4,
                            is_playing,
                            sample_position,
                            midi_events: &[],
                            aux_buffer: None, // TODO: handle this aux buffer with actual aux buffer
                        };
                        let mut master_bus =
                            self.current_state.graph.mixer_state.master_bus.clone();
                        let master_bus_mut = Arc::make_mut(&mut master_bus);
                        Self::apply_master_bus_with_effects(
                            master_bus_mut,
                            &mut self.plugin_state.master_effects,
                            &self.master_automation_events,
                            output,
                            channels,
                            &master_effect_ctx,
                        );
                    } else {
                        // output silent buffer
                        output.fill(0.0);
                    }
                }
            }
        }
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
        let fade_samples = ((sample_rate as f32) * 0.002) as u32;
        for voice in active_oneshots
            .iter_mut()
            .filter(|v| v.track_id == track_id)
        {
            did_render = true;
            let src_channels = voice.waveform.channels as usize;
            let step = (voice.waveform.sample_rate as f64) / (sample_rate as f64);

            let Some(buffer) = get_waveform_buffer(&voice.waveform.buffer) else {
                return false;
            };

            // Pre-calculate Loop Bounds to hoist out of the loop
            let max_len = (buffer.len() / src_channels) as f64;
            let trim_end = if voice.end_boundary > 0.0 && voice.end_boundary < max_len {
                voice.end_boundary
            } else {
                max_len
            };
            let loop_len = trim_end - voice.start_boundary;
            let is_looping = voice.waveform.is_looping && loop_len > 0.0;

            // Pre-calculate exact frame count to avoid branching in the loop
            let mut frames_to_process = buffer_frames.saturating_sub(voice.output_offset_samples);
            if !is_looping {
                let max_steps = (trim_end - 1.0 - voice.source_read_index) / step;
                if max_steps < 0.0 {
                    frames_to_process = 0; // Already past the end
                } else {
                    frames_to_process = frames_to_process.min((max_steps.floor() as usize) + 1);
                }
            }

            if frames_to_process == 0 {
                continue;
            }

            did_render = true;

            // Slice exactly the part of the buffer we are writing to
            let start_idx = voice.output_offset_samples * channels;
            let end_idx = start_idx + frames_to_process * channels;
            let target_slice = &mut output[start_idx..end_idx];

            let mut frames_written = 0;

            if channels == 2 {
                // ==== SIMD STEREO (2 frames per loop) ====
                let mut iter = target_slice.chunks_exact_mut(4);

                for chunk in iter.by_ref() {
                    // Frame 0
                    let elapsed0 = voice.clip_elapsed_samples + frames_written;
                    let rp0 = get_read_pos(
                        voice.source_read_index,
                        (frames_written as f64) * step,
                        is_looping,
                        trim_end,
                        voice.start_boundary,
                        loop_len,
                    );
                    let s0 = sample_waveform_dasp(&voice.waveform, rp0, src_channels);
                    let fade0 = calc_fade(elapsed0, fade_samples, voice.clip_loop_length);

                    // Frame 1
                    let elapsed1 = voice.clip_elapsed_samples + frames_written + 1;
                    let rp1 = get_read_pos(
                        voice.source_read_index,
                        ((frames_written + 1) as f64) * step,
                        is_looping,
                        trim_end,
                        voice.start_boundary,
                        loop_len,
                    );
                    let s1 = sample_waveform_dasp(&voice.waveform, rp1, src_channels);
                    let fade1 = calc_fade(elapsed1, fade_samples, voice.clip_loop_length);

                    // SIMD Vector packing
                    let samples = f32x4::new([s0[0], s0[1], s1[0], s1[1]]);
                    let fades = f32x4::new([fade0, fade0, fade1, fade1]);

                    // SIMD Fused Multiply-Add
                    let mut out_v = f32x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    out_v += samples * fades;
                    chunk.copy_from_slice(&out_v.to_array());

                    frames_written += 2;
                }

                // Remainder (0 or 2 floats = 1 stereo frame)
                for chunk in iter.into_remainder().chunks_exact_mut(2) {
                    let elapsed0 = voice.clip_elapsed_samples + frames_written;
                    let rp0 = get_read_pos(
                        voice.source_read_index,
                        (frames_written as f64) * step,
                        is_looping,
                        trim_end,
                        voice.start_boundary,
                        loop_len,
                    );
                    let s0 = sample_waveform_dasp(&voice.waveform, rp0, src_channels);
                    let fade0 = calc_fade(elapsed0, fade_samples, voice.clip_loop_length);

                    chunk[0] += s0[0] * fade0;
                    chunk[1] += s0[1] * fade0;
                    frames_written += 1;
                }
            } else {
                // ==== SIMD MONO (4 frames per loop) ====
                let mut iter = target_slice.chunks_exact_mut(4);

                for chunk in iter.by_ref() {
                    let mut s = [0.0; 4];
                    let mut f = [0.0; 4];

                    for i in 0..4 {
                        let elapsed = voice.clip_elapsed_samples + frames_written + i;
                        let rp = get_read_pos(
                            voice.source_read_index,
                            ((frames_written + i) as f64) * step,
                            is_looping,
                            trim_end,
                            voice.start_boundary,
                            loop_len,
                        );
                        s[i as usize] = sample_waveform_dasp(&voice.waveform, rp, src_channels)[0];
                        f[i as usize] = calc_fade(elapsed, fade_samples, voice.clip_loop_length);
                    }

                    let samples = f32x4::new(s);
                    let fades = f32x4::new(f);

                    let mut out_v = f32x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    out_v += samples * fades;
                    chunk.copy_from_slice(&out_v.to_array());

                    frames_written += 4;
                }

                // Remainder (1 to 3 frames)
                for chunk in iter.into_remainder().iter_mut() {
                    let elapsed = voice.clip_elapsed_samples + frames_written;
                    let rp = get_read_pos(
                        voice.source_read_index,
                        (frames_written as f64) * step,
                        is_looping,
                        trim_end,
                        voice.start_boundary,
                        loop_len,
                    );
                    let s0 = sample_waveform_dasp(&voice.waveform, rp, src_channels);
                    let fade0 = calc_fade(elapsed, fade_samples, voice.clip_loop_length);

                    *chunk += s0[0] * fade0;
                    frames_written += 1;
                }
            }
        }
        did_render
    }

    /// Apply mixer channel settings (volume, pan, phase) and effects from plugin_state
    fn apply_mixer_channel_with_effects(
        mixer_channel: &mut MixerChannel,
        track_effects: &mut Vec<Vec<AudioEffectInstance>>,
        track_automation_events: &[(TrackId, Vec<TrackAutomationEvent>)],
        track_id: TrackId,
        buffer: &mut [f32],
        channels: usize,
        process_ctx: &ProcessContext,
    ) {
        // Extract base values from the current UI state
        let track_volume = &mut mixer_channel.volume;
        let track_pan = &mut mixer_channel.pan;

        // CONSUME TRACK AUTOMATION
        if let Some((_, auto_events)) = track_automation_events
            .iter()
            .find(|(id, _)| *id == track_id)
        {
            for event in auto_events {
                match event {
                    TrackAutomationEvent::Volume(v) => {
                        track_volume.apply_automation(*v);
                    }
                    TrackAutomationEvent::Pan(v) => {
                        track_pan.apply_automation(*v);
                    }
                    TrackAutomationEvent::PluginParam {
                        effect_id,
                        param_id,
                        value,
                    } => {
                        if let Some(effects) = track_effects.get_mut(track_id.to_u32() as usize) {
                            if let Some(effect) = effects.iter_mut().find(|e| e.id == *effect_id) {
                                effect.plugin.apply_automation(*param_id, *value);
                            }
                        }
                    }
                }
            }
        }

        // ==== SIMD Phase Inversion ====
        if mixer_channel.inverted_phase {
            let neg_one = f32x4::splat(-1.0);
            let mut iter = buffer.chunks_exact_mut(4);

            for chunk in iter.by_ref() {
                let mut v = f32x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);
                v *= neg_one;
                chunk.copy_from_slice(&v.to_array());
            }

            for sample in iter.into_remainder() {
                *sample = -*sample;
            }
        }

        // Effects chain from plugin_state
        if let Some(effects) = track_effects.get_mut(track_id.to_u32() as usize) {
            for effect in effects.iter_mut() {
                effect.plugin.process(buffer, process_ctx);
            }
        }

        // Apply calculated Volume and Pan
        let pan = track_pan.get();
        let volume = db_to_linear(track_volume.get());

        let (left_gain, right_gain) = if channels == 2 {
            let p = (pan + 1.0) * 0.5;
            ((1.0 - p).sqrt() * volume, p.sqrt() * volume)
        } else {
            (volume, volume)
        };

        // Apply gain
        if channels == 2 {
            let gain_v = f32x4::new([left_gain, right_gain, left_gain, right_gain]);
            let mut iter = buffer.chunks_exact_mut(4);

            for chunk in iter.by_ref() {
                let mut v = f32x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);
                v *= gain_v;
                chunk.copy_from_slice(&v.to_array());
            }

            // Handle remainder (0 or 2 floats for stereo)
            for chunk in iter.into_remainder().chunks_exact_mut(2) {
                chunk[0] *= left_gain;
                chunk[1] *= right_gain;
            }
        } else {
            let gain_v = f32x4::splat(left_gain);
            let mut iter = buffer.chunks_exact_mut(4);

            for chunk in iter.by_ref() {
                let mut v = f32x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);
                v *= gain_v;
                chunk.copy_from_slice(&v.to_array());
            }

            for sample in iter.into_remainder() {
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
        master_bus: &mut MixerChannel,
        master_effects: &mut [AudioEffectInstance],
        master_automation_events: &[MasterAutomationEvent],
        buffer: &mut [f32],
        channels: usize,
        process_ctx: &ProcessContext,
    ) {
        let master_volume = &mut master_bus.volume;
        let master_pan = &mut master_bus.pan;

        // CONSUME MASTER AUTOMATION
        for event in master_automation_events {
            match event {
                MasterAutomationEvent::Volume(v) => {
                    master_volume.apply_automation(*v);
                }
                MasterAutomationEvent::Pan(v) => {
                    master_pan.apply_automation(*v);
                }
                MasterAutomationEvent::PluginParam {
                    effect_id,
                    param_id,
                    value,
                } => {
                    if let Some(effect) = master_effects.iter_mut().find(|e| e.id == *effect_id) {
                        effect.plugin.apply_automation(*param_id, *value);
                    }
                }
            }
        }

        // ==== SIMD Phase Inversion ====
        if master_bus.inverted_phase {
            let neg_one = f32x4::splat(-1.0);
            let mut iter = buffer.chunks_exact_mut(4);

            for chunk in iter.by_ref() {
                let mut v = f32x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);
                v *= neg_one;
                chunk.copy_from_slice(&v.to_array());
            }

            for sample in iter.into_remainder() {
                *sample = -*sample;
            }
        }

        // Master effects chain
        for effect in master_effects.iter_mut() {
            effect.plugin.process(buffer, process_ctx);
        }

        // Volume and Pan (volume is stored in dB)
        let pan = master_pan.get();
        let volume = db_to_linear(master_volume.get());
        let (left_gain, right_gain) = if channels == 2 {
            let p = (pan + 1.0) * 0.5;
            ((1.0 - p).sqrt() * volume, p.sqrt() * volume)
        } else {
            (volume, volume)
        };

        // ==== SIMD Apply Gain and Pan ====
        if channels == 2 {
            let gain_v = f32x4::new([left_gain, right_gain, left_gain, right_gain]);
            let mut iter = buffer.chunks_exact_mut(4);

            for chunk in iter.by_ref() {
                let mut v = f32x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);
                v *= gain_v;
                chunk.copy_from_slice(&v.to_array());
            }

            // Handle remainder (0 or 2 floats for stereo)
            for chunk in iter.into_remainder().chunks_exact_mut(2) {
                chunk[0] *= left_gain;
                chunk[1] *= right_gain;
            }
        } else {
            let gain_v = f32x4::splat(left_gain);
            let mut iter = buffer.chunks_exact_mut(4);

            for chunk in iter.by_ref() {
                let mut v = f32x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);
                v *= gain_v;
                chunk.copy_from_slice(&v.to_array());
            }

            for sample in iter.into_remainder() {
                *sample *= left_gain;
            }
        }
    }

    fn resolve_sequencer_events(&mut self, buffer_size: usize) {
        let start_time = self.song_state.playhead_samples;
        let end_time = start_time + (buffer_size as u32);

        // Use the tracks from the current audio graph state
        let tracks = self.current_state.graph.tracks.clone();

        for track in tracks.iter() {
            self.process_track(track.as_ref(), start_time, end_time);
        }
    }

    fn process_track(&mut self, track: &AudioTrack, start_time: u32, end_time: u32) {
        let track_id = track.id;

        let mut gen_voice_idx = None;
        if let Some(gen_instance) = &track.generator {
            gen_voice_idx = Self::ensure_generator_voice(
                &mut self.active_generators,
                &self.plugin_state,
                track_id,
                gen_instance,
            );
        }

        let samples_per_beat = ((60.0 / self.bpm) * (self.sample_rate as f32)) as f64;
        let samples_per_tick = samples_per_beat / 960.0;

        for clip_data in track.clips() {
            let (clip_start, clip_length, clip_offset) = match &clip_data.time {
                crate::core::project::clip::ClipTimeUnit::Samples {
                    start_time,
                    loop_length,
                    offset_start,
                } => (
                    *start_time as u32,
                    *loop_length as u32,
                    *offset_start as u32,
                ),
                crate::core::project::clip::ClipTimeUnit::Ticks {
                    start_time,
                    loop_length,
                    offset_start,
                } => {
                    let st = (*start_time as f64 * samples_per_tick) as u32;
                    let ll = (*loop_length as f64 * samples_per_tick) as u32;
                    let os = (*offset_start as f64 * samples_per_tick) as u32;
                    (st, ll, os)
                }
            };

            if clip_start > end_time {
                break;
            }
            let clip_end = clip_start + clip_length;
            if clip_end < start_time {
                continue;
            }

            let clip = Clip {
                name: clip_data.name.clone(),
                id: clip_data.id,
                source: clip_data.source.clone(),
                time: crate::core::project::clip::ClipTimeUnit::Samples {
                    start_time: clip_start as u64,
                    loop_length: clip_length as u64,
                    offset_start: clip_offset as u64,
                },
            };

            match &clip.source {
                DawSource::Audio(source_id) => {
                    let waveform_opt = self
                        .current_state
                        .graph
                        .asset_library
                        .source_map
                        .get(source_id)
                        .cloned();
                    if let Some(waveform) = waveform_opt {
                        self.prepare_audio_voice(track.id, &clip, &waveform, start_time, end_time);
                    }
                }
                DawSource::Midi(id) => {
                    let fresh_pattern = self.current_state.graph.patterns.get(id);

                    if let Some(pattern) = fresh_pattern {
                        if let Some(idx) = gen_voice_idx {
                            let gen_voice = &mut self.active_generators[idx];
                            Self::schedule_midi_events(
                                &mut gen_voice.midi_events,
                                self.sample_rate,
                                self.bpm,
                                &clip,
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

        // Just sort the events. Playing_keys is safely handled downstream now!
        if let Some(idx) = gen_voice_idx {
            let voice = &mut self.active_generators[idx];
            voice.midi_events.sort_by_key(|e| e.sample_offset);
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
        if plugin_state
            .get_generator(gen_instance.id.to_u32() as usize)
            .is_some()
        {
            // Create lightweight voice reference (actual plugin is in plugin_state)
            active_generators.push(GeneratorVoice::new(gen_instance.id, track_id, true));
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

            let Some(buffer) = get_waveform_buffer(&voice.waveform.buffer) else {
                break;
            };

            let buffer_len = buffer.len();
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
        let clip_timeline_start = clip.time.start_time_raw() as u32;
        let render_start = std::cmp::max(buffer_start, clip_timeline_start);
        let render_end = std::cmp::min(
            buffer_end,
            clip_timeline_start + clip.time.loop_length_raw() as u32,
        );

        if render_end <= render_start {
            return;
        }

        let output_offset = (render_start - buffer_start) as usize;
        let samples_elapsed = render_start - clip_timeline_start;
        let effective_pos = samples_elapsed + clip.time.offset_start_raw() as u32;

        let ratio = (waveform.sample_rate as f64) / (self.sample_rate as f64);
        let source_elapsed_frames = (effective_pos as f64) * ratio;

        let trim_start = waveform.trim_start as f64;
        let trim_end = if waveform.trim_end > 0 {
            waveform.trim_end as f64
        } else {
            let Some(buffer) = get_waveform_buffer(&waveform.buffer) else {
                return;
            };

            // Get bytes from memory-mapped buffer and convert to f32 frmames
            (buffer.len() / (waveform.channels as usize)) as f64
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
            clip_elapsed_samples: samples_elapsed,
            clip_loop_length: clip.time.loop_length_raw() as u32,
        });
    }

    fn schedule_midi_events(
        events: &mut SmallVec<[MidiEvent; 4]>,
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

        let clip_start = clip.time.start_time_raw() as u32;
        let clip_length = clip.time.loop_length_raw() as u32;
        let clip_offset = clip.time.offset_start_raw() as u32;
        let clip_end = clip_start + clip_length;

        let pattern_offset = 0;

        for note in &pattern.notes {
            let note_start =
                (((note.start_tick as f64) / 960.0) * (samples_per_beat as f64)) as u32;
            let note_dur = (((note.duration as f64) / 960.0) * (samples_per_beat as f64)) as u32;

            // Note position within the pattern (in samples from pattern start)
            let note_pos_in_pattern = pattern_offset + note_start;

            // Skip notes that start before the clip's trim offset
            if note_pos_in_pattern < clip_offset {
                continue;
            }

            // Calculate absolute timeline position: clip start + (note position - trim offset)
            let abs_start = clip_start + note_pos_in_pattern - clip_offset;
            let abs_end = abs_start + note_dur;

            // Skip notes that start at or after the clip end (outside trimmed region)
            // Because we no longer loop, if the clip is dragged out longer than the
            // pattern, abs_start will naturally just stop being evaluated when the notes run out!
            if abs_start >= clip_end {
                continue;
            }

            // Clamp note-off to clip boundary if it would extend past the clip end
            let effective_end = abs_end.min(clip_end);

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

            // Schedule NoteOff if it falls within the buffer
            if effective_end >= buffer_start && effective_end < buffer_end {
                events.push(MidiEvent {
                    sample_offset: (effective_end - buffer_start) as usize,
                    data: MidiMessage::NoteOff { key: note.key },
                });
            }
        }
        events.sort_by_key(|e| e.sample_offset);
    }

    // Helper to schedule notes without a Clip wrapper
    fn schedule_pattern_notes_raw(
        events: &mut SmallVec<[MidiEvent; 4]>,
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
    }

    fn evaluate_automation_lanes(&mut self) {
        let tempo = self.bpm;
        if tempo <= 0.0 {
            return;
        }

        // Convert playhead from samples to ticks (960 ticks per beat)
        let samples_per_beat = (60.0 / tempo) * (self.sample_rate as f32);
        let samples_per_tick = samples_per_beat / 960.0;
        let current_tick =
            ((self.song_state.playhead_samples as f64) / (samples_per_tick as f64)) as u32;

        for lane in self.current_state.graph.automation_lanes.values() {
            let value = lane.value_at_ticks(current_tick);

            match &lane.target {
                AutomationTarget::TrackGeneratorPluginParam { track_id, param_id } => {
                    if let Some(voice) = self
                        .active_generators
                        .iter_mut()
                        .find(|v| v.track_id == *track_id)
                    {
                        voice
                            .automation_events
                            .push(GeneratorAutomationEvent::PluginParam {
                                param_id: *param_id,
                                value,
                            });
                    }
                }
                AutomationTarget::TrackVolume(track_id) => {
                    let pos = self
                        .track_automation_events
                        .iter()
                        .position(|(id, _)| id == track_id)
                        .unwrap_or_else(|| {
                            let idx = self.track_automation_events.len();
                            self.track_automation_events.push((*track_id, Vec::new()));
                            idx
                        });
                    self.track_automation_events[pos]
                        .1
                        .push(TrackAutomationEvent::Volume(value));
                }
                AutomationTarget::TrackPan(track_id) => {
                    let pos = self
                        .track_automation_events
                        .iter()
                        .position(|(id, _)| id == track_id)
                        .unwrap_or_else(|| {
                            let idx = self.track_automation_events.len();
                            self.track_automation_events.push((*track_id, Vec::new()));
                            idx
                        });
                    self.track_automation_events[pos]
                        .1
                        .push(TrackAutomationEvent::Pan(value));
                }
                AutomationTarget::TrackPluginParam {
                    track_id,
                    effect_id,
                    param_id,
                } => {
                    let pos = self
                        .track_automation_events
                        .iter()
                        .position(|(id, _)| id == track_id)
                        .unwrap_or_else(|| {
                            let idx = self.track_automation_events.len();
                            self.track_automation_events.push((*track_id, Vec::new()));
                            idx
                        });
                    self.track_automation_events[pos]
                        .1
                        .push(TrackAutomationEvent::PluginParam {
                            effect_id: *effect_id,
                            param_id: *param_id,
                            value,
                        });
                }
                AutomationTarget::BusVolume(bus_id) => {
                    let pos = self
                        .bus_automation_events
                        .iter()
                        .position(|(id, _)| id == bus_id)
                        .unwrap_or_else(|| {
                            let idx = self.bus_automation_events.len();
                            self.bus_automation_events.push((*bus_id, Vec::new()));
                            idx
                        });
                    self.bus_automation_events[pos]
                        .1
                        .push(BusAutomationEvent::Volume(value));
                }
                AutomationTarget::BusPluginParam {
                    bus_id,
                    effect_id,
                    param_id,
                } => {
                    let pos = self
                        .bus_automation_events
                        .iter()
                        .position(|(id, _)| id == bus_id)
                        .unwrap_or_else(|| {
                            let idx = self.bus_automation_events.len();
                            self.bus_automation_events.push((*bus_id, Vec::new()));
                            idx
                        });
                    self.bus_automation_events[pos]
                        .1
                        .push(BusAutomationEvent::PluginParam {
                            effect_id: *effect_id,
                            param_id: *param_id,
                            value,
                        });
                }
                AutomationTarget::MasterVolume => {
                    self.master_automation_events
                        .push(MasterAutomationEvent::Volume(value));
                }
                AutomationTarget::TempoBpm => {
                    // Global target handled immediately since transport is owned by AudioEngine
                    self.bpm = value;
                }
                _ => {}
            }
        }
    }

    fn render_metronome(&mut self, output: &mut [f32], channels: usize, start_playhead: u32) {
        if !self.metronome_state.is_active {
            return;
        }

        let samples_per_beat = (60.0 / self.bpm) * (self.sample_rate as f32);
        if samples_per_beat <= 0.0 {
            return;
        }

        let frames = output.len() / channels;
        let mut out_iter = output.chunks_exact_mut(channels);

        for i in 0..frames {
            let current_sample = start_playhead + i as u32;

            // Detect exact beat boundaries perfectly
            let is_trigger = if current_sample == 0 {
                true
            } else {
                let prev_beat_idx = ((current_sample - 1) as f32 / samples_per_beat) as u32;
                let curr_beat_idx = (current_sample as f32 / samples_per_beat) as u32;
                curr_beat_idx > prev_beat_idx
            };

            if is_trigger {
                self.metronome_state.is_playing = true;
                self.metronome_state.play_index = 0; // Reset playhead to start of WAV

                let curr_beat_idx = (current_sample as f32 / samples_per_beat) as u32;
                self.metronome_state.is_downbeat = curr_beat_idx.is_multiple_of(4);
            }

            if self.metronome_state.is_playing {
                // Select the correct pre-loaded audio buffer
                let buffer = if self.metronome_state.is_downbeat {
                    &self.metronome_state.downbeat_buffer
                } else {
                    &self.metronome_state.offbeat_buffer
                };

                // Play the sample until the buffer runs out
                if self.metronome_state.play_index < buffer.len() {
                    let sample = buffer[self.metronome_state.play_index];

                    if let Some(chunk) = out_iter.next() {
                        for c in chunk.iter_mut() {
                            *c += sample; // Mix the mono sample equally into all output channels (L/R)
                        }
                    }
                    self.metronome_state.play_index += 1;
                } else {
                    // Reached the end of the WAV file, stop playing to save CPU
                    self.metronome_state.is_playing = false;
                    out_iter.next();
                }
            } else {
                out_iter.next();
            }
        }
    }

    fn recalculate_latencies(&mut self) {
        let mut internal_latency: HashMap<RoutingNode, u32> = HashMap::new();

        // Calculate internal latencies for tracks
        for track in self.current_state.graph.tracks.as_ref() {
            let mut lat = 0;
            if let Some(gen) = &track.generator {
                if let Some(instance) = self.plugin_state.get_generator(gen.id.to_u32() as usize) {
                    lat += instance.plugin.latency_samples();
                }
            }
            if let Some(effects) = self
                .plugin_state
                .get_track_effects(track.id.to_u32() as usize)
            {
                for e in effects {
                    lat += e.plugin.latency_samples();
                }
            }
            internal_latency.insert(RoutingNode::Track(track.id), lat);
        }

        // Calculate internal latencies for bus channels
        for bus_id in self.current_state.graph.mixer_state.buses.keys() {
            let mut lat = 0;
            if let Some(effects) = self.plugin_state.get_bus_effects(bus_id.to_u32() as usize) {
                for e in effects {
                    lat += e.plugin.latency_samples();
                }
            }
            internal_latency.insert(RoutingNode::Bus(*bus_id), lat);
        }

        // Calculate latencies for Master
        let mut master_lat = 0;
        for e in &self.plugin_state.master_effects {
            master_lat += e.plugin.latency_samples();
        }
        internal_latency.insert(RoutingNode::Master, master_lat);

        // Calculate Path Latencies (from Node to Output)
        let mut path_latency: HashMap<RoutingNode, u32> = HashMap::new();
        path_latency.insert(RoutingNode::Master, master_lat);

        // Process buses in REVERSE routing order (from master backwards)
        for node in self.cached_routing_order.iter().rev() {
            if let RoutingNode::Bus(_) = node {
                let my_internal = internal_latency.get(node).copied().unwrap_or(0);
                let mut max_dest_path = 0;
                for route in &self.current_state.graph.mixer_state.routing {
                    if route.source == *node {
                        max_dest_path = max_dest_path
                            .max(path_latency.get(&route.destination).copied().unwrap_or(0));
                    }
                }
                path_latency.insert(*node, my_internal + max_dest_path);
            }
        }

        // Process tracks
        for track in self.current_state.graph.tracks.as_ref() {
            let node = RoutingNode::Track(track.id);
            let my_internal = internal_latency.get(&node).copied().unwrap_or(0);
            let mut max_dest_path = 0;
            for route in &self.current_state.graph.mixer_state.routing {
                if route.source == node {
                    max_dest_path = max_dest_path
                        .max(path_latency.get(&route.destination).copied().unwrap_or(0));
                }
            }
            path_latency.insert(node, my_internal + max_dest_path);
        }

        // Find max system latency and allocate Delay Lines
        let max_system_latency = path_latency.values().copied().max().unwrap_or(0);
        let channels = self.num_channels as usize;

        for (node, path_lat) in &path_latency {
            let comp = max_system_latency - path_lat;
            self.compensation_delays.insert(*node, comp);

            match node {
                RoutingNode::Track(id) => {
                    let dl = self
                        .track_delay_lines
                        .entry(*id)
                        .or_insert_with(DelayLine::default);
                    dl.set_delay(comp as usize, channels);
                }
                RoutingNode::Bus(id) => {
                    let dl = self
                        .bus_delay_lines
                        .entry(*id)
                        .or_insert_with(DelayLine::default);
                    dl.set_delay(comp as usize, channels);
                }
                RoutingNode::Master => {} // Master has no compensation delay
            }
        }

        log::info!(
            "[PDC] Recalculated Latencies. Max System Latency: {} samples",
            max_system_latency
        );
    }

    fn get_plugin_box_gen(gen_id: GeneratorId) {

    }

    fn get_plugin_box_effect(target: EffectTarget, effect_id: EffectId) {

    }
}

/// Sample a waveform at a specific position using dasp interpolation.
/// Handles fallback from 1-channel to 2-channel stereo.
#[inline]
fn sample_waveform_dasp(waveform: &AudioWaveform, pos: f64, src_channels: usize) -> [f32; 2] {
    let idx = pos as usize;
    let alpha = (pos - (idx as f64)) as f32;

    let Some(buffer) = get_waveform_buffer(&waveform.buffer) else {
        return [0f32, 0f32];
    };

    if src_channels == 2 {
        let frames: &[[f32; 2]] = slice::from_sample_slice(buffer).unwrap_or(&[]);
        let len = frames.len();

        if idx >= len {
            return [0.0, 0.0];
        }

        let p0 = if idx > 0 {
            frames[idx - 1]
        } else {
            frames[idx]
        };
        let p1 = frames[idx];
        let p2 = if idx + 1 < len { frames[idx + 1] } else { p1 };
        let p3 = if idx + 2 < len { frames[idx + 2] } else { p2 };

        [
            hermite_interp(alpha, p0[0], p1[0], p2[0], p3[0]),
            hermite_interp(alpha, p0[1], p1[1], p2[1], p3[1]),
        ]
    } else {
        let frames: &[[f32; 1]] = slice::from_sample_slice(buffer).unwrap_or(&[]);
        let len = frames.len();

        if idx >= len {
            return [0.0, 0.0];
        }

        let p0 = if idx > 0 {
            frames[idx - 1]
        } else {
            frames[idx]
        };
        let p1 = frames[idx];
        let p2 = if idx + 1 < len { frames[idx + 1] } else { p1 };
        let p3 = if idx + 2 < len { frames[idx + 2] } else { p2 };

        let val = hermite_interp(alpha, p0[0], p1[0], p2[0], p3[0]);
        [val, val]
    }
}

#[inline(always)]
fn calc_fade(current_elapsed: u32, fade_samples: u32, loop_length: u32) -> f32 {
    if fade_samples == 0 {
        return 1.0;
    }
    if current_elapsed < fade_samples {
        (current_elapsed as f32) / (fade_samples as f32)
    } else if current_elapsed + fade_samples > loop_length {
        let remaining = loop_length.saturating_sub(current_elapsed);
        (remaining as f32) / (fade_samples as f32)
    } else {
        1.0
    }
}

#[inline(always)]
fn get_read_pos(
    base_idx: f64,
    offset: f64,
    is_looping: bool,
    trim_end: f64,
    start_bound: f64,
    loop_len: f64,
) -> f64 {
    let rp = base_idx + offset;
    if is_looping && rp >= trim_end {
        start_bound + ((rp - trim_end) % loop_len)
    } else {
        rp
    }
}

/// Helper function to decode 16-bit PCM WAV bytes into a flat f32 array
fn load_internal_wav(bytes: &[u8]) -> Vec<f32> {
    let cursor = std::io::Cursor::new(bytes);
    let Ok(reader) = hound::WavReader::new(cursor) else {
        return Vec::new();
    };

    // Convert 16-bit integer (-32768 to 32767) to f32 (-1.0 to 1.0)
    reader
        .into_samples::<i16>()
        .map(|s| s.unwrap_or(0) as f32 / 32768.0)
        .collect()
}
