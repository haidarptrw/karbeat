// src/audio/engine.rs

use std::collections::HashMap;

use log::info;
use rtrb::{Consumer, Producer};
use triple_buffer::Output;

use crate::{
    audio::{event::PlaybackPosition, render_state::AudioRenderState},
    commands::AudioCommand,
    core::{
        project::{Clip, Pattern},
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
    beat_samples_accumulator: usize,

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
    Midi(MidiVoice),
    Audio(AudioVoice),
}

pub struct MidiVoice {
    pub note: u8,
    pub velocity: u8,
    pub phase: f32,
    pub is_finished: bool,
    // For sample accuracy: when does this note start within the current buffer
    pub start_offset_samples: usize,
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
            beat_samples_accumulator: 0,
            current_beat: 1,
            current_bar: 1,
            last_emitted_samples: 0,
        }
    }

    pub fn process(&mut self, output_buffer: &mut [f32]) {
        if self.state_consumer.update() {
            self.current_state = self.state_consumer.read().clone();
        }

        while let Ok(cmd) = self.command_consumer.pop() {
            match cmd {
                AudioCommand::PlayOneShot(waveform) => {
                    self.preview_voices.clear();
                    self.preview_voices.push(PreviewVoice::new(waveform, 1.0));
                }
                AudioCommand::StopAllPreviews => {
                    self.preview_voices.clear();
                }
                AudioCommand::ResetPlayhead => {
                    self.reset_playhead();
                }
                AudioCommand::SetPlayhead(samples) => {
                    println!("[AudioEngine] Set Playhead to {}", samples);
                    self.playhead_samples = samples as u64;
                    self.recalculate_beat_bar();
                    self.last_emitted_samples = self.playhead_samples;

                    // Immediately push the reset position back to UI so the slider snaps back
                    if !self.position_producer.is_full() {
                        let _ = self.position_producer.push(PlaybackPosition {
                            samples: self.playhead_samples,
                            beat: self.current_beat,
                            bar: self.current_bar,
                            tempo: self.current_state.tempo,
                            sample_rate: self.current_state.sample_rate,
                            is_playing: self.current_state.is_playing,
                        });
                    }
                }
            }
        }

        // CLEAR BUFFER
        output_buffer.fill(0.0);

        let channels = 2;

        // check if the playhead has already exceeded the max sample index
        if self.playhead_samples > self.current_state.max_sample_index {
            self.current_state.is_playing = false;
            self.reset_playhead();
        } else if self.current_state.is_playing {
            let frame_count = output_buffer.len() / channels;

            // sequencer
            self.resolve_sequencer_events(frame_count);

            self.render_voices_to_buffer(output_buffer, channels);

            self.playhead_samples += frame_count as u64;

            self.recalculate_beat_bar();

            let emission_interval = self.sample_rate / 60;

            if self.playhead_samples >= self.last_emitted_samples + emission_interval {
                if !self.position_producer.is_full() {
                    let _ = self.position_producer.push(PlaybackPosition {
                        samples: self.playhead_samples,
                        beat: self.current_beat,
                        bar: self.current_bar,
                        tempo: self.current_state.tempo,
                        sample_rate: self.current_state.sample_rate,
                        is_playing: self.current_state.is_playing,
                    });
                }
                self.last_emitted_samples = self.playhead_samples;
            }

            // Cleanup previous active voices
            for voices in self.active_voices.values_mut() {
                voices.retain(|v| match v {
                    Voice::Midi(s) => !s.is_finished,
                    Voice::Audio(_) => false, // Always clear audio voices after render
                });
            }
        } else {
            if !self.position_producer.is_full() {
                let _ = self.position_producer.push(PlaybackPosition {
                    samples: self.playhead_samples,
                    beat: self.current_beat,
                    bar: self.current_bar,
                    tempo: self.current_state.tempo,
                    sample_rate: self.current_state.sample_rate,
                    is_playing: false,
                });
            }
        }

        // Always RUN
        self.render_previews_to_buffer(output_buffer, channels);
    }

    /// Recalculates current Beat and Bar based on playhead_samples
    /// Uses 1-based indexing for musical time.
    fn recalculate_beat_bar(&mut self) {
        let samples_per_beat = (60.0 / self.current_state.tempo * self.sample_rate as f32) as usize;

        if samples_per_beat > 0 {
            // Formula: Beat = (TotalBeats) + 1
            self.current_beat = (self.playhead_samples as usize / samples_per_beat) + 1;

            // Formula: Bar = (TotalBeats / 4) + 1
            self.current_bar = (self.current_beat - 1) / 4 + 1;

            self.beat_samples_accumulator = self.playhead_samples as usize % samples_per_beat;
        } else {
            // Fallback
            self.current_beat = 1;
            self.current_bar = 1;
            self.beat_samples_accumulator = 0;
        }
    }

    fn reset_playhead(&mut self) {
        println!("[AudioEngine] Resetting Playhead to 0");
        self.playhead_samples = 0;
        self.current_beat = 1;
        self.current_bar = 1;
        self.beat_samples_accumulator = 0;
        self.last_emitted_samples = 0;
        self.current_state.is_playing = false;

        // Immediately push the reset position back to UI so the slider snaps back
        if !self.position_producer.is_full() {
            let _ = self.position_producer.push(PlaybackPosition {
                samples: 0,
                beat: 1,
                bar: 1,
                tempo: self.current_state.tempo,
                sample_rate: self.current_state.sample_rate,
                is_playing: false,
            });
        }
    }

    fn render_voices_to_buffer(&mut self, output: &mut [f32], channels: usize) {
        let buffer_frames = output.len() / channels;
        for (mixer_id_opt, voices) in &mut self.active_voices {
            // PLACEHOLDER: voice volume
            // TODO: Lookup MixerChannel volume here
            let vol = match mixer_id_opt {
                Some(id) => {
                    // TODO: Lookup MixerChannel volume using *id
                    // let channel = self.current_state.mixer.get(id);
                    // channel.volume
                    1.0
                }
                None => {
                    // Direct to Master (No mixer processing)
                    1.0
                }
            };

            for voice in voices.iter_mut() {
                match voice {
                    Voice::Midi(synth_voice) => {
                        if synth_voice.is_finished {
                            continue;
                        }

                        for frame_idx in 0..buffer_frames {
                            if frame_idx < synth_voice.start_offset_samples {
                                continue;
                            }

                            // PLACEHOLDER: Simple Sine Wave Logic
                            // TODO: use wave generator from the plugin
                            let freq =
                                440.0 * 2.0_f32.powf((synth_voice.note as f32 - 69.0) / 12.0);
                            let sample =
                                (synth_voice.phase * 2.0 * std::f32::consts::PI).sin() * 0.5;
                            synth_voice.phase += freq / self.sample_rate as f32;

                            // Stereo Mix
                            if channels > 0 {
                                output[frame_idx * channels] += sample * vol;
                            }
                            if channels > 1 {
                                output[frame_idx * channels + 1] += sample * vol;
                            }
                        }
                    }
                    Voice::Audio(audio_voice) => {
                        let src_channels = audio_voice.waveform.channels as usize;
                        let buffer_len = audio_voice.waveform.buffer.len();

                        // Calculate step size (Pitch/Speed ratio)
                        let step =
                            audio_voice.waveform.sample_rate as f64 / self.sample_rate as f64;

                        for frame_idx in audio_voice.output_offset_samples..buffer_frames {
                            let frames_written =
                                (frame_idx - audio_voice.output_offset_samples) as u64;

                            // 1. Calculate precise floating point position
                            let mut read_pos_f64 =
                                audio_voice.source_read_index + (frames_written as f64 * step);

                            // 2. Handle Looping / Trimming Limits
                            let trim_end = audio_voice.end_boundary;
                            let trim_start = audio_voice.start_boundary;

                            let max_len = (buffer_len / src_channels) as f64;

                            // Safety clamp for end of buffer
                            let end_bound = if trim_end > 0.0 && trim_end < max_len {
                                trim_end
                            } else {
                                max_len
                            };

                            if audio_voice.waveform.is_looping && (trim_end - trim_start) > 0.0 {
                                if read_pos_f64 >= end_bound {
                                    let loop_len = end_bound - trim_start;
                                    let remainder = read_pos_f64 - end_bound;
                                    read_pos_f64 = trim_start + (remainder % loop_len);
                                }
                            } else {
                                if read_pos_f64 >= end_bound - 1.0 {
                                    break;
                                }
                            }

                            // 3. Prepare Interpolation Data
                            let index_int = read_pos_f64 as usize; // Floor
                            let alpha = (read_pos_f64 - index_int as f64) as f32; // Fraction (0.0 to 0.99)

                            // 4. Calculate Next Index (for looking ahead)
                            let next_index_int = if index_int + 1 >= end_bound as usize {
                                if audio_voice.waveform.is_looping {
                                    trim_start as usize
                                } else {
                                    index_int // Clamp to end if not looping (prevents crash)
                                }
                            } else {
                                index_int + 1
                            };

                            // 5. Fetch Samples & Interpolate
                            let base_idx = index_int * src_channels;
                            let next_base_idx = next_index_int * src_channels;

                            if next_base_idx + (src_channels - 1) < buffer_len {
                                // Get Left Channel
                                let curr_l = audio_voice.waveform.buffer[base_idx];
                                let next_l = audio_voice.waveform.buffer[next_base_idx];
                                let val_l = lerp(curr_l, next_l, alpha);

                                // Get Right Channel
                                let (curr_r, next_r) = if src_channels > 1 {
                                    (
                                        audio_voice.waveform.buffer[base_idx + 1],
                                        audio_voice.waveform.buffer[next_base_idx + 1],
                                    )
                                } else {
                                    (curr_l, next_l) // Mono to Stereo
                                };
                                let val_r = lerp(curr_r, next_r, alpha);

                                // 6. Mix to Output
                                if channels > 0 {
                                    output[frame_idx * channels] += val_l * vol;
                                }
                                if channels > 1 {
                                    output[frame_idx * channels + 1] += val_r * vol;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn resolve_sequencer_events(&mut self, buffer_size: usize) {
        let start_time = self.playhead_samples;
        let end_time = start_time + buffer_size as u64;

        let AudioEngine {
            current_state,
            active_voices,
            sample_rate,
            ..
        } = self;

        for track in &current_state.tracks {
            for clip in track.clips() {
                if clip.start_time > end_time {
                    break;
                }
                let clip_end = clip.start_time + clip.loop_length;

                if clip_end < start_time {
                    continue;
                }
                match &clip.source {
                    crate::core::project::KarbeatSource::Audio(waveform) => {
                        Self::process_audio_clip(
                            active_voices,
                            track.target_mixer_channel_id,
                            clip,
                            waveform,
                            start_time,
                            end_time,
                            sample_rate.to_owned(),
                        );
                    }
                    crate::core::project::KarbeatSource::Midi(pattern) => {
                        Self::process_pattern_in_clip(
                            active_voices,
                            *sample_rate,
                            current_state.tempo,
                            track.target_mixer_channel_id,
                            clip,
                            pattern,
                            start_time,
                            end_time,
                        );
                    }
                    crate::core::project::KarbeatSource::Automation(_) => {
                        // TODO: Implementing Automation
                    }
                }
            }
        }
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
        // 1. TIMELINE DOMAIN
        let clip_timeline_start = clip.start_time;
        // Clip length is now authoritative for timeline boundaries
        let clip_timeline_end = clip.start_time + clip.loop_length;

        let render_start = std::cmp::max(buffer_start, clip_timeline_start);
        let render_end = std::cmp::min(buffer_end, clip_timeline_end);

        if render_end <= render_start {
            return;
        }

        let output_offset = (render_start - buffer_start) as usize;

        // 2. SOURCE DOMAIN
        let samples_elapsed_timeline = render_start - clip_timeline_start;
        let effective_play_pos_timeline = samples_elapsed_timeline + clip.offset_start;

        let ratio = waveform.sample_rate as f64 / sample_rate as f64;
        let source_elapsed_frames = effective_play_pos_timeline as f64 * ratio;

        // 3. BOUNDARY & LOOPING LOGIC
        // CHANGED: Retrieve trim properties from the WAVEFORM (Source)
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

        // 4. PUSH VOICE
        let voices = active_voices.entry(mixer_id).or_insert(Vec::new());

        voices.push(Voice::Audio(AudioVoice {
            waveform: waveform.clone(),
            output_offset_samples: output_offset,
            source_read_index: source_read_idx,
            start_boundary: trim_start_source,
            end_boundary: trim_end_source,
        }));
    }

    fn process_pattern_in_clip(
        active_voices: &mut HashMap<Option<u32>, Vec<Voice>>,
        sample_rate: u64,
        tempo: f32,
        mixer_id: Option<u32>,
        clip: &Clip,
        pattern: &Pattern,
        buffer_start: u64,
        buffer_end: u64,
    ) {
        let samples_per_beat = (60.0 / tempo * sample_rate as f32) as u64;
        let pattern_len_samples = pattern.length_bars as u64 * 4 * samples_per_beat;

        for (_midi_ch, notes) in &pattern.notes {
            for note in notes {
                let note_start_samples =
                    (note.start_tick as f64 / 960.0 * samples_per_beat as f64) as u64;

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
                    let absolute_note_time =
                        clip.start_time + pattern_offset + note_start_samples - clip.offset_start;

                    if absolute_note_time >= buffer_start && absolute_note_time < buffer_end {
                        let offset_in_buffer = (absolute_note_time - buffer_start) as usize;

                        let voices = active_voices.entry(mixer_id).or_insert(Vec::new());
                        // Wrap in Enum
                        voices.push(Voice::Midi(MidiVoice {
                            note: note.key,
                            velocity: note.velocity,
                            phase: 0.0,
                            is_finished: false,
                            start_offset_samples: offset_in_buffer,
                        }));
                    }
                }
            }
        }
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
}

/// Basic Linear Interpolation
#[inline(always)]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
