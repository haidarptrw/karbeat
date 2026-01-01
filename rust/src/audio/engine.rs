// src/audio/engine.rs

use std::sync::{Arc, Mutex};

use rtrb::{Consumer, Producer};
use triple_buffer::Output;

use crate::{
    audio::{event::PlaybackPosition, render_state::AudioRenderState},
    commands::AudioCommand,
    core::project::plugin::{MidiEvent, MidiMessage},
    core::project::{
        AudioSourceId, AudioWaveform, Clip, GeneratorId, GeneratorInstance, GeneratorInstanceType,
        KarbeatPlugin, KarbeatSource, KarbeatTrack, Pattern, TrackId, TransportState,
    },
};

pub struct AudioEngine {
    // Comms
    state_consumer: Output<AudioRenderState>,
    position_producer: Producer<PlaybackPosition>,
    current_state: AudioRenderState,

    // Timeline
    sample_rate: u64,
    playhead_samples: u64,
    current_beat: usize,
    current_bar: usize,

    // Active Voices
    active_generators: Vec<GeneratorVoice>,
    active_oneshots: Vec<AudioVoice>,
    preview_voices: Vec<PreviewVoice>,

    // Real-time Command Queue
    command_consumer: Consumer<AudioCommand>,

    // Update emit scheduler
    last_emitted_samples: u64,

    mix_buffer: Vec<f32>,
}

pub struct GeneratorVoice {
    pub id: GeneratorId,
    pub track_id: TrackId,
    // The shared plugin instance (Thread-safe)
    pub generator: Arc<Mutex<KarbeatPlugin>>,
    // Events queued for the CURRENT buffer block only
    pub events: Vec<MidiEvent>,
    // Track if this generator is persistent or temporary
    pub active: bool,
}

pub struct AudioVoice {
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
        sample_rate: u64,
        initial_state: AudioRenderState,
    ) -> Self {
        let mix_buffer = Vec::with_capacity(2048);
        Self {
            state_consumer,
            command_consumer,
            position_producer,
            current_state: initial_state,
            sample_rate,
            playhead_samples: 0,
            active_generators: Vec::with_capacity(32),
            active_oneshots: Vec::with_capacity(32),
            preview_voices: Vec::with_capacity(4),
            current_beat: 1,
            current_bar: 1,
            last_emitted_samples: 0,
            mix_buffer,
        }
    }

    pub fn process(&mut self, output_buffer: &mut [f32]) {
        // 1. Sync State
        if self.state_consumer.update() {
            let new_state = self.state_consumer.read().clone();

            // Check if we switched from Playing -> Stopped via heavy update
            if self.current_state.transport.is_playing && !new_state.transport.is_playing {
                self.stop_all_active_generators();
            }

            self.current_state = new_state.clone();
        }

        // 2. Process Commands (Play, Stop, Seek)
        while let Ok(cmd) = self.command_consumer.pop() {
            self.process_command(cmd);
        }

        // 3. Clear Buffer
        output_buffer.fill(0.0);
        let channels = 2;
        let frame_count = output_buffer.len() / channels;

        // 4. Transport Logic
        if self.current_state.transport.is_playing {
            // Check end of song
            if self.playhead_samples > self.current_state.graph.max_sample_index {
                self.stop_playback();
            } else {
                // Schedule Events (MIDI / Audio Clips)
                self.resolve_sequencer_events(frame_count);

                // Render Active Voices
                self.render_voices_to_buffer(output_buffer, channels);

                // Advance Playhead
                self.playhead_samples += frame_count as u64;
                self.recalculate_beat_bar();
                self.emit_playback_position();

                // Cleanup
                self.cleanup_finished_voices();
            }
        } else {
            self.render_voices_to_buffer(output_buffer, channels);

            // Clean up audio voices (oneshots), keep generator voices active if they have pending audio tail
            self.cleanup_finished_voices();

            self.emit_static_position();
        }

        // 5. Always Render Previews (Metronome, Browser Preview)
        self.render_previews_to_buffer(output_buffer, channels);
    }

    fn stop_playback(&mut self) {
        self.reset_playhead();
    }

    fn stop_all_active_generators(&mut self) {
        for voice in self.active_generators.iter_mut() {
            if let Ok(mut guard) = voice.generator.lock() {
                if let KarbeatPlugin::Generator(gen) = &mut *guard {
                    // Reset the plugin to kill all internal voices/envelopes
                    gen.reset();
                }
            }
            // Clear any pending MIDI events that might have been queued
            voice.events.clear();
        }
        self.preview_voices.clear();
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
                self.playhead_samples = samples as u64;
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
        }
    }

    /// Recalculates current Beat and Bar based on playhead_samples
    /// Uses 1-based indexing for musical time.
    fn recalculate_beat_bar(&mut self) {
        let tempo = self.current_state.transport.bpm;
        if tempo <= 0.0 {
            return;
        }

        let samples_per_beat = (60.0 / tempo * self.sample_rate as f32) as usize;
        if samples_per_beat == 0 {
            return;
        }

        self.current_beat = (self.playhead_samples as usize / samples_per_beat) + 1;
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
        if self.playhead_samples >= self.last_emitted_samples + emission_interval {
            if !self.position_producer.is_full() {
                let _ = self
                    .position_producer
                    .push(self.build_position_struct(Some(true)));
            }
            self.last_emitted_samples = self.playhead_samples;
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
        PlaybackPosition {
            samples: self.playhead_samples,
            beat: self.current_beat,
            bar: self.current_bar,
            tempo: self.current_state.transport.bpm,
            sample_rate: self.current_state.graph.sample_rate,
            is_playing,
        }
    }

    #[allow(dead_code)]
    fn emit_position_toggle_play(&mut self, is_playing: bool) {
        if !self.position_producer.is_full() {
            let _ = self
                .position_producer
                .push(self.build_position_struct(Some(is_playing)));
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
        // Find the track that has this generator
        let target_info = self.current_state.graph.tracks.iter().find_map(|t| {
            if let Some(gen) = &t.generator {
                if gen.id == generator_id {
                    return Some((t.id, gen.clone()));
                }
            }
            None
        });

        if let Some((track_id, gen_instance)) = target_info {
            // Ensure the voice is active (even if Transport is stopped)
            // This creates the voice if it doesn't exist.
            if let Some(voice_idx) = self.ensure_generator_voice(track_id, &gen_instance) {
                // Inject MIDI event
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

                // Keep voice alive for processing even if track is empty
                gen_voice.active = true;
            } else {
                log::warn!(
                    "PlayPreviewNote: Generator ID {:?} not found in active graph",
                    generator_id
                );
            }
        }
    }

    fn render_voices_to_buffer(&mut self, output: &mut [f32], channels: usize) {
        // Render all generators
        self.render_generator(output);

        // Render all oneshot audio voices
        self.render_oneshots(output, channels);
    }

    fn render_generator(&mut self, output: &mut [f32]) {
        let buf_len = output.len();

        for voice in self.active_generators.iter_mut() {
            if !voice.active {
                continue;
            }

            // Optimization: Try-lock to prevent audio thread stalls
            // If the UI is holding the lock for too long, we skip one frame of this synth
            // rather than freezing the entire audio engine.
            if let Ok(mut guard) = voice.generator.try_lock() {
                if let KarbeatPlugin::Generator(generator) = &mut *guard {
                    // Ensure scratch buffer is sized correctly and silent
                    self.mix_buffer.resize(buf_len, 0.0);
                    self.mix_buffer.fill(0.0);

                    // Render into scratch buffer
                    generator.process(&mut self.mix_buffer, &voice.events);

                    // Mix scratch buffer into main output
                    // TODO: SIMD optimization here eventually
                    for i in 0..buf_len {
                        output[i] += self.mix_buffer[i]; // * volume
                    }
                }
            } else {
                // Log warning or count dropped frames if needed
            }
        }
    }

    fn render_oneshots(&mut self, output: &mut [f32], channels: usize) {
        let buffer_frames = output.len() / channels;

        for voice in self.active_oneshots.iter_mut() {
            let src_channels = voice.waveform.channels as usize;
            let step = voice.waveform.sample_rate as f64 / self.sample_rate as f64;

            // Pre-calculate Loop Bounds to hoist out of the loop
            let max_len = (voice.waveform.buffer.len() / src_channels) as f64;
            let trim_end = if voice.end_boundary > 0.0 && voice.end_boundary < max_len {
                voice.end_boundary
            } else {
                max_len
            };
            let loop_len = trim_end - voice.start_boundary;
            let is_looping = voice.waveform.is_looping && loop_len > 0.0;

            for frame_idx in voice.output_offset_samples..buffer_frames {
                let frames_written = (frame_idx - voice.output_offset_samples) as u64;
                let mut read_pos = voice.source_read_index + (frames_written as f64 * step);

                if is_looping {
                    if read_pos >= trim_end {
                        let remainder = read_pos - trim_end;
                        read_pos = voice.start_boundary + (remainder % loop_len);
                    }
                } else if read_pos >= trim_end - 1.0 {
                    // Mark for cleanup? In this simple engine, we just stop adding
                    break;
                }

                // Inline sampling logic for speed
                let (l, r) = sample_waveform_inline(&voice.waveform, read_pos, src_channels);

                if channels > 0 {
                    output[frame_idx * channels] += l;
                }
                if channels > 1 {
                    output[frame_idx * channels + 1] += r;
                }
            }
        }
    }

    fn resolve_sequencer_events(&mut self, buffer_size: usize) {
        let start_time = self.playhead_samples;
        let end_time = start_time + buffer_size as u64;

        // Use the tracks from the current audio graph state
        let tracks = self.current_state.graph.tracks.clone();

        for track in tracks.iter() {
            self.process_track(track.as_ref(), start_time, end_time);
        }
    }

    fn process_track(&mut self, track: &KarbeatTrack, start_time: u64, end_time: u64) {
        let track_id = track.id;

        // 1. Ensure Generator Voice exists
        let mut gen_voice_idx = None;
        if let Some(gen_instance) = &track.generator {
            gen_voice_idx = self.ensure_generator_voice(track_id, gen_instance);
        }

        // 2. Process Clips
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
                    // Clone the waveform to avoid borrow conflict with self
                    let waveform_opt = self
                        .current_state
                        .graph
                        .asset_library
                        .source_map
                        .get(source_id)
                        .cloned();
                    if let Some(waveform) = waveform_opt {
                        self.prepare_audio_voice(clip, &waveform, start_time, end_time);
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
                                pattern, // Use the fresh pattern here
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

    fn ensure_generator_voice(
        &mut self,
        track_id: TrackId,
        gen_instance: &GeneratorInstance,
    ) -> Option<usize> {
        // Find existing generator voice by ID
        if let Some(idx) = self
            .active_generators
            .iter()
            .position(|g| g.id == gen_instance.id)
        {
            return Some(idx);
        }

        // Create new generator voice
        if let GeneratorInstanceType::Plugin(p) = &gen_instance.instance_type {
            if let Some(plugin_arc) = &p.instance {
                if let Ok(mut guard) = plugin_arc.lock() {
                    if let KarbeatPlugin::Generator(gen) = &mut *guard {
                        // Use buffer size from current state, fallback to 512 if not set
                        let buf_size = if self.current_state.graph.buffer_size > 0 {
                            self.current_state.graph.buffer_size
                        } else {
                            512
                        };

                        gen.prepare(self.sample_rate as f32, buf_size);
                    }
                }

                self.active_generators.push(GeneratorVoice {
                    id: gen_instance.id,
                    track_id,
                    generator: plugin_arc.clone(),
                    events: Vec::new(),
                    active: true,
                });
                return Some(self.active_generators.len() - 1);
            }
        }
        None
    }

    // --- HELPER: AUDIO CLIP PROCESSING ---
    #[allow(dead_code)]
    fn process_audio_clip(
        &mut self,
        clip: &Clip,
        waveform: &AudioWaveform,
        buffer_start: u64,
        buffer_end: u64,
    ) {
        let clip_timeline_start = clip.start_time;
        let clip_timeline_end = clip.start_time + clip.loop_length;

        let render_start = std::cmp::max(buffer_start, clip_timeline_start);
        let render_end = std::cmp::min(buffer_end, clip_timeline_end);

        if render_end <= render_start {
            return;
        }

        let output_offset = (render_start - buffer_start) as usize;

        let samples_elapsed_timeline = render_start - clip_timeline_start;
        let effective_play_pos_timeline = samples_elapsed_timeline + clip.offset_start;

        let ratio = waveform.sample_rate as f64 / self.sample_rate as f64;
        let source_elapsed_frames = effective_play_pos_timeline as f64 * ratio;

        let trim_start_source = waveform.trim_start as f64;

        let trim_end_source = if waveform.trim_end > 0 {
            waveform.trim_end as f64
        } else {
            // Fallback to full buffer length if 0
            (waveform.buffer.len() / waveform.channels as usize) as f64
        };

        let source_read_idx;
        let loop_region_len = trim_end_source - trim_start_source;

        if waveform.is_looping && loop_region_len > 0.0 {
            let offset_in_loop = source_elapsed_frames % loop_region_len;
            source_read_idx = trim_start_source + offset_in_loop;
        } else {
            source_read_idx = trim_start_source + source_elapsed_frames;
            if source_read_idx >= trim_end_source {
                return;
            }
        }

        self.active_oneshots.push(AudioVoice {
            waveform: waveform.clone(),
            output_offset_samples: output_offset,
            source_read_index: source_read_idx,
            start_boundary: trim_start_source,
            end_boundary: trim_end_source,
        });
    }

    fn render_previews_to_buffer(&mut self, output: &mut [f32], channels: usize) {
        let buffer_frames = output.len() / channels;

        for voice in &mut self.preview_voices {
            if voice.is_finished {
                continue;
            }

            let src_channels = voice.waveform.channels as usize;
            let buffer_len = voice.waveform.buffer.len();
            let step = voice.waveform.sample_rate as f64 / self.sample_rate as f64;

            for i in 0..buffer_frames {
                let current_pos_f64 = voice.current_frame + voice.waveform.trim_start as f64;

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

                // Interpolation Logic
                let index_int = current_pos_f64 as usize;
                let alpha = (current_pos_f64 - index_int as f64) as f32;

                let next_index_int = if index_int + 1 >= end_bound as usize {
                    index_int
                } else {
                    index_int + 1
                };

                let base_idx = index_int * src_channels;
                let next_base_idx = next_index_int * src_channels;

                if next_base_idx + (src_channels - 1) < buffer_len {
                    let curr_l = voice.waveform.buffer[base_idx];
                    let next_l = voice.waveform.buffer[next_base_idx];
                    let val_l = lerp(curr_l, next_l, alpha);

                    let (curr_r, next_r) = if src_channels > 1 {
                        (
                            voice.waveform.buffer[base_idx + 1],
                            voice.waveform.buffer[next_base_idx + 1],
                        )
                    } else {
                        (curr_l, next_l)
                    };
                    let val_r = lerp(curr_r, next_r, alpha);

                    if channels > 0 {
                        output[i * channels] += val_l * voice.volume;
                    }
                    if channels > 1 {
                        output[i * channels + 1] += val_r * voice.volume;
                    }
                }

                voice.current_frame += step;
            }
        }

        self.preview_voices.retain(|v| !v.is_finished);
    }

    fn prepare_audio_voice(
        &mut self,
        clip: &Clip,
        waveform: &AudioWaveform,
        buffer_start: u64,
        buffer_end: u64,
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

        let ratio = waveform.sample_rate as f64 / self.sample_rate as f64;
        let source_elapsed_frames = effective_pos as f64 * ratio;

        let trim_start = waveform.trim_start as f64;
        let trim_end = if waveform.trim_end > 0 {
            waveform.trim_end as f64
        } else {
            (waveform.buffer.len() / waveform.channels as usize) as f64
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
            waveform: waveform.clone(),
            output_offset_samples: output_offset,
            source_read_index: source_read_idx,
            start_boundary: trim_start,
            end_boundary: trim_end,
        });
    }

    fn schedule_midi_events(
        events: &mut Vec<MidiEvent>,
        sample_rate: u64,
        tempo: f32,
        clip: &Clip,
        pattern: &Pattern,
        buffer_start: u64,
        buffer_end: u64,
    ) {
        let samples_per_beat = (60.0 / tempo * sample_rate as f32) as u64;
        if samples_per_beat == 0 {
            return;
        }

        let pattern_len_samples =
            (pattern.length_ticks as f64 / 960.0 * samples_per_beat as f64) as u64;
        if pattern_len_samples == 0 {
            return;
        }

        // Calculate overlap
        // let relative_start = buffer_start.saturating_sub(clip.start_time);
        // let relative_end = buffer_end - clip.start_time;

        // let loop_read_start = relative_start + clip.offset_start;
        // let loop_read_end = relative_end + clip.offset_start;

        // let start_iter = loop_read_start / pattern_len_samples;
        // let end_iter = loop_read_end / pattern_len_samples;

        let start_iter = 0;
        let end_iter = 0;

        for i in start_iter..=end_iter {
            let pattern_offset = i * pattern_len_samples;

            for note in &pattern.notes {
                let note_start = (note.start_tick as f64 / 960.0 * samples_per_beat as f64) as u64;
                let note_dur = (note.duration as f64 / 960.0 * samples_per_beat as f64) as u64;

                let abs_start = clip.start_time + pattern_offset + note_start - clip.offset_start;
                let abs_end = abs_start + note_dur;

                if abs_start < clip.offset_start {
                    continue;
                }

                if abs_start >= buffer_start && abs_start < buffer_end {
                    events.push(MidiEvent {
                        sample_offset: (abs_start - buffer_start) as usize,
                        data: MidiMessage::NoteOn {
                            key: note.key,
                            velocity: note.velocity,
                        },
                    });
                }
                if abs_end >= buffer_start && abs_end < buffer_end {
                    events.push(MidiEvent {
                        sample_offset: (abs_end - buffer_start) as usize,
                        data: MidiMessage::NoteOff { key: note.key },
                    });
                }
            }
        }
        events.sort_by_key(|e| e.sample_offset);
    }
}

#[inline(always)]
fn sample_waveform_inline(waveform: &AudioWaveform, pos: f64, channels: usize) -> (f32, f32) {
    let idx = pos as usize;
    let alpha = (pos - idx as f64) as f32;
    let base = idx * channels;

    // Unchecked access is faster, but requires ensuring bounds previously.
    // Using safe access for now.
    if base + channels >= waveform.buffer.len() {
        return (0.0, 0.0);
    }

    let next_base = if base + channels < waveform.buffer.len() {
        base + channels
    } else {
        base
    };

    let curr_l = waveform.buffer[base];
    let next_l = waveform.buffer[next_base];
    let val_l = curr_l + (next_l - curr_l) * alpha; // lerp

    let val_r = if channels > 1 {
        let curr_r = waveform.buffer[base + 1];
        let next_r = waveform.buffer[next_base + 1];
        curr_r + (next_r - curr_r) * alpha
    } else {
        val_l
    };

    (val_l, val_r)
}

/// Basic Linear Interpolation
#[inline(always)]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
