// src/audio/engine.rs

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use rtrb::{Consumer, Producer};
use triple_buffer::Output;

use crate::{
    audio::{event::PlaybackPosition, render_state::AudioRenderState},
    commands::AudioCommand,
    core::{
        plugin::{KarbeatPlugin, MidiEvent, MidiMessage},
        project::{Clip, KarbeatSource, Pattern, TransportState},
        track::audio_waveform::AudioWaveform,
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

    // Polyphony: Map <TrackID, List of Active Voices>
    active_voices: HashMap<Option<u32>, Vec<Voice>>,

    // Real-time Command Queue
    command_consumer: Consumer<AudioCommand>,

    // For one shot
    preview_voices: Vec<PreviewVoice>,

    // Update emit scheduler
    last_emitted_samples: u64,
}

pub enum Voice {
    Generator(GeneratorVoice),
    Audio(AudioVoice),
}

pub struct GeneratorVoice {
    pub id: u32,
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
        Self {
            state_consumer,
            command_consumer,
            position_producer,
            current_state: initial_state,
            sample_rate,
            playhead_samples: 0,
            active_voices: HashMap::new(),
            preview_voices: Vec::new(),
            current_beat: 1,
            current_bar: 1,
            last_emitted_samples: 0,
        }
    }

    pub fn process(&mut self, output_buffer: &mut [f32]) {
        // 1. Sync State
        if self.state_consumer.update() {
            self.current_state = self.state_consumer.read().clone();
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
                // A. Schedule Events (MIDI / Audio Clips)
                self.resolve_sequencer_events(frame_count);

                // B. Render Active Voices
                self.render_voices_to_buffer(output_buffer, channels);

                // C. Advance Playhead
                self.playhead_samples += frame_count as u64;
                self.recalculate_beat_bar();
                self.emit_playback_position();

                // D. Cleanup
                self.cleanup_finished_voices();
            }
        } else {
            // Not playing? Emit static position for UI sync
            self.emit_static_position();
        }

        // 5. Always Render Previews (Metronome, Browser Preview)
        self.render_previews_to_buffer(output_buffer, channels);
    }

    fn stop_playback(&mut self) {
        // We can't modify `current_state` directly if it's supposed to be read-only from `state_consumer`.
        // Ideally, we send a "Stop" event back to the main thread via a producer.
        // For now, we locally stop processing.
        // self.current_state.transport.is_playing = false; // This is a local override only
        self.reset_playhead();
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
                self.emit_static_position(); // Snap UI immediately
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
                    .push(self.build_position_struct(true));
            }
            self.last_emitted_samples = self.playhead_samples;
        }
    }

    fn emit_static_position(&mut self) {
        if !self.position_producer.is_full() {
            let _ = self
                .position_producer
                .push(self.build_position_struct(false));
        }
    }

    fn build_position_struct(&self, is_playing: bool) -> PlaybackPosition {
        PlaybackPosition {
            samples: self.playhead_samples,
            beat: self.current_beat,
            bar: self.current_bar,
            tempo: self.current_state.transport.bpm,
            sample_rate: self.current_state.graph.sample_rate,
            is_playing,
        }
    }

    fn cleanup_finished_voices(&mut self) {
        for voices in self.active_voices.values_mut() {
            voices.retain(|v| match v {
                Voice::Generator(g) => g.active, // Generators stay alive (persistent)
                Voice::Audio(_) => false, // Audio voices are One-Shot per buffer (re-added every frame)
            });

            // Clear MIDI events for generators for the next frame
            for v in voices.iter_mut() {
                if let Voice::Generator(g) = v {
                    g.events.clear();
                }
            }
        }
    }

    fn render_voices_to_buffer(&mut self, output: &mut [f32], channels: usize) {
        let mut gen_buffer = vec![0.0; output.len()];

        for (mixer_id_opt, voices) in &self.active_voices {
            // TODO: Implement Mixer Channel Volume / Processing here
            let vol = 1.0;

            for voice in voices.iter() {
                match voice {
                    Voice::Generator(gen_voice) => {
                        self.render_generator(gen_voice, output, &mut gen_buffer, vol);
                    }
                    Voice::Audio(audio_voice) => {
                        self.render_audio_voice(audio_voice, output, channels, vol);
                    }
                }
            }
        }
    }

    fn render_generator(
        &self,
        voice: &GeneratorVoice,
        output: &mut [f32],
        gen_buffer: &mut [f32],
        vol: f32,
    ) {
        if let Ok(mut guard) = voice.generator.lock() {
            if let KarbeatPlugin::Generator(generator) = &mut *guard {
                gen_buffer.fill(0.0);
                generator.process(gen_buffer, &voice.events);

                // Mix into main buffer
                for (i, sample) in gen_buffer.iter().enumerate() {
                    output[i] += sample * vol;
                }
            }
        }
    }

    fn render_audio_voice(
        &self,
        voice: &AudioVoice,
        output: &mut [f32],
        channels: usize,
        vol: f32,
    ) {
        let buffer_frames = output.len() / channels;
        let src_channels = voice.waveform.channels as usize;

        // Pitch/Speed Ratio
        let step = voice.waveform.sample_rate as f64 / self.sample_rate as f64;

        // Loop Bounds
        let max_len = (voice.waveform.buffer.len() / src_channels) as f64;
        let trim_start = voice.start_boundary;
        let trim_end = if voice.end_boundary > 0.0 && voice.end_boundary < max_len {
            voice.end_boundary
        } else {
            max_len
        };

        let loop_len = trim_end - trim_start;

        for frame_idx in voice.output_offset_samples..buffer_frames {
            let frames_written = (frame_idx - voice.output_offset_samples) as u64;

            // Calculate Read Position
            let mut read_pos = voice.source_read_index + (frames_written as f64 * step);

            // Handle Looping
            if voice.waveform.is_looping && loop_len > 0.0 {
                if read_pos >= trim_end {
                    let remainder = read_pos - trim_end;
                    read_pos = trim_start + (remainder % loop_len);
                }
            } else if read_pos >= trim_end - 1.0 {
                break;
            }

            // Interpolate and Mix
            let (l, r) = self.sample_waveform(&voice.waveform, read_pos, src_channels);

            if channels > 0 {
                output[frame_idx * channels] += l * vol;
            }
            if channels > 1 {
                output[frame_idx * channels + 1] += r * vol;
            }
        }
    }

    // Extracted Interpolation Logic
    #[inline(always)]
    fn sample_waveform(&self, waveform: &AudioWaveform, pos: f64, channels: usize) -> (f32, f32) {
        let idx = pos as usize;
        let alpha = (pos - idx as f64) as f32;

        let base = idx * channels;
        // Simple clamp to prevent panic if next index is OOB (looping logic should handle this, but safety first)
        let next_base = if base + channels < waveform.buffer.len() {
            base + channels
        } else {
            base
        };

        let curr_l = waveform.buffer[base];
        let next_l = waveform.buffer[next_base];
        let val_l = lerp(curr_l, next_l, alpha);

        let val_r = if channels > 1 {
            let curr_r = waveform.buffer[base + 1];
            let next_r = waveform.buffer[next_base + 1];
            lerp(curr_r, next_r, alpha)
        } else {
            val_l // Mono -> Stereo
        };

        (val_l, val_r)
    }

    fn resolve_sequencer_events(&mut self, buffer_size: usize) {
        let start_time = self.playhead_samples;
        let end_time = start_time + buffer_size as u64;

        let tracks = self.current_state.graph.tracks.clone();

        for track in tracks {
            self.process_track(track.as_ref(), start_time, end_time);
        }
    }

    fn process_track(
        &mut self,
        track: &crate::core::project::KarbeatTrack,
        start_time: u64,
        end_time: u64,
    ) {
        // 1. Ensure Generator Voice exists
        let mut gen_voice_idx = None;
        if let Some(gen_instance) = &track.generator {
            gen_voice_idx =
                self.ensure_generator_voice(track.target_mixer_channel_id, gen_instance);
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
                KarbeatSource::Audio(waveform) => {
                    Self::prepare_audio_voice(
                        &mut self.active_voices,
                        track.target_mixer_channel_id,
                        clip,
                        waveform,
                        start_time,
                        end_time,
                        self.sample_rate,
                    );
                }
                KarbeatSource::Midi(pattern) => {
                    if let Some(idx) = gen_voice_idx {
                        if let Some(voices) =
                            self.active_voices.get_mut(&track.target_mixer_channel_id)
                        {
                            if let Voice::Generator(ref mut gen_voice) = voices[idx] {
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
                }
                _ => {}
            }
        }
    }

    fn ensure_generator_voice(
        &mut self,
        mixer_id: Option<u32>,
        gen_instance: &crate::core::project::GeneratorInstance,
    ) -> Option<usize> {
        let voices = self.active_voices.entry(mixer_id).or_insert(Vec::new());

        // Find existing
        if let Some(idx) = voices
            .iter()
            .position(|v| matches!(v, Voice::Generator(g) if g.id == gen_instance.id))
        {
            return Some(idx);
        }

        // Create new
        if let crate::core::project::GeneratorInstanceType::Plugin(p) = &gen_instance.instance_type
        {
            if let Some(plugin_arc) = &p.instance {
                voices.push(Voice::Generator(GeneratorVoice {
                    id: gen_instance.id,
                    generator: plugin_arc.clone(),
                    events: Vec::new(),
                    active: true,
                }));
                return Some(voices.len() - 1);
            }
        }
        None
    }

    // --- HELPER: AUDIO CLIP PROCESSING ---
    fn process_audio_clip(
        active_voices: &mut HashMap<Option<u32>, Vec<Voice>>,
        mixer_id: Option<u32>,
        clip: &Clip,
        waveform: &AudioWaveform,
        buffer_start: u64,
        buffer_end: u64,
        sample_rate: u64,
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

        let ratio = waveform.sample_rate as f64 / sample_rate as f64;
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

        let voices = active_voices.entry(mixer_id).or_insert(Vec::new());

        voices.push(Voice::Audio(AudioVoice {
            waveform: waveform.clone(),
            output_offset_samples: output_offset,
            source_read_index: source_read_idx,
            start_boundary: trim_start_source,
            end_boundary: trim_end_source,
        }));
    }

    fn process_pattern_events(
        events: &mut Vec<MidiEvent>,
        sample_rate: u64,
        tempo: f32,
        clip: &Clip,
        pattern: &Pattern,
        buffer_start: u64,
        buffer_end: u64,
    ) {
        let samples_per_beat = (60.0 / tempo * sample_rate as f32) as u64;
        let pattern_len_samples =
            (pattern.length_ticks as f64 / 960.0 * samples_per_beat as f64) as u64;

        if pattern_len_samples == 0 {
            return;
        }

        for note in &pattern.notes {
            let note_start_samples =
                (note.start_tick as f64 / 960.0 * samples_per_beat as f64) as u64;
            let note_duration_samples =
                (note.duration as f64 / 960.0 * samples_per_beat as f64) as u64;

            let relative_buffer_start = if buffer_start > clip.start_time {
                buffer_start - clip.start_time
            } else {
                0
            };
            let relative_buffer_end = buffer_end - clip.start_time;

            let loop_read_start = relative_buffer_start + clip.offset_start;
            let loop_read_end = relative_buffer_end + clip.offset_start;

            let start_iter = loop_read_start / pattern_len_samples;
            let end_iter = loop_read_end / pattern_len_samples;

            for i in start_iter..=end_iter {
                let pattern_offset = i * pattern_len_samples;
                let abs_note_start =
                    clip.start_time + pattern_offset + note_start_samples - clip.offset_start;
                let abs_note_end = abs_note_start + note_duration_samples;

                // 1. Note ON
                if abs_note_start >= buffer_start && abs_note_start < buffer_end {
                    events.push(MidiEvent {
                        sample_offset: (abs_note_start - buffer_start) as usize,
                        data: MidiMessage::NoteOn {
                            key: note.key,
                            velocity: note.velocity,
                        },
                    });
                }

                // 2. Note OFF
                if abs_note_end >= buffer_start && abs_note_end < buffer_end {
                    events.push(MidiEvent {
                        sample_offset: (abs_note_end - buffer_start) as usize,
                        data: MidiMessage::NoteOff { key: note.key },
                    });
                }
            }
        }

        events.sort_by_key(|e| e.sample_offset);
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
        active_voices: &mut HashMap<Option<u32>, Vec<Voice>>,
        mixer_id: Option<u32>,
        clip: &Clip,
        waveform: &AudioWaveform,
        buffer_start: u64,
        buffer_end: u64,
        sample_rate: u64,
    ) {
        // ... (Keep existing logic) ...
        let clip_timeline_start = clip.start_time;
        let render_start = std::cmp::max(buffer_start, clip_timeline_start);
        let render_end = std::cmp::min(buffer_end, clip_timeline_start + clip.loop_length);

        if render_end <= render_start {
            return;
        }

        let output_offset = (render_start - buffer_start) as usize;
        let samples_elapsed = render_start - clip_timeline_start;
        let effective_pos = samples_elapsed + clip.offset_start;

        let ratio = waveform.sample_rate as f64 / sample_rate as f64;
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

        active_voices
            .entry(mixer_id)
            .or_default()
            .push(Voice::Audio(AudioVoice {
                waveform: waveform.clone(),
                output_offset_samples: output_offset,
                source_read_index: source_read_idx,
                start_boundary: trim_start,
                end_boundary: trim_end,
            }));
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
        // ... (Keep existing logic) ...
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
        let relative_start = buffer_start.saturating_sub(clip.start_time);
        let relative_end = buffer_end - clip.start_time;

        let loop_read_start = relative_start + clip.offset_start;
        let loop_read_end = relative_end + clip.offset_start;

        let start_iter = loop_read_start / pattern_len_samples;
        let end_iter = loop_read_end / pattern_len_samples;

        for i in start_iter..=end_iter {
            let pattern_offset = i * pattern_len_samples;

            for note in &pattern.notes {
                let note_start = (note.start_tick as f64 / 960.0 * samples_per_beat as f64) as u64;
                let note_dur = (note.duration as f64 / 960.0 * samples_per_beat as f64) as u64;

                let abs_start = clip.start_time + pattern_offset + note_start - clip.offset_start;
                let abs_end = abs_start + note_dur;

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

/// Basic Linear Interpolation
#[inline(always)]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
