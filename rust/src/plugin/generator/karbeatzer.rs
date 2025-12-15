// src/plugin/generator/karbeatzer.rs

use crate::core::plugin::{KarbeatGenerator, MidiEvent, MidiMessage};
use std::f32::consts::PI;

/// **Karbeatzer**, an enhanced subtractive synthesizer
pub struct Karbeatzer {
    sample_rate: f32,
    oscillators: [Oscillator; 3],
    
    // Global Filter
    filter: Filter,
    
    // Global Envelope Settings (ADSR)
    amp_env_settings: AdsrSettings,

    // FX
    drive: f32, // 0.0 to 1.0 (Saturation)
    
    active_voices: Vec<Voice>,
    gain: f32,

    // Internal scratch buffers for block processing
    // Size is typically small (e.g. up to MAX_BLOCK_SIZE)
    voice_buffer: Vec<f32>, 
}

#[derive(Clone, Copy)]
struct Oscillator {
    waveform: Waveform,
    detune: f32, // In semitones
    mix: f32,    // 0.0 to 1.0
    pulse_width: f32, // 0.0 to 1.0 (For Pulse/Square)
}

#[derive(Clone, Copy, PartialEq)]
enum Waveform {
    Sine = 0,
    Saw = 1,
    Square = 2,
    Triangle = 3,
    Noise = 4,
}

impl From<f32> for Waveform {
    fn from(v: f32) -> Self {
        match v as u32 {
            0 => Waveform::Sine,
            1 => Waveform::Saw,
            2 => Waveform::Square,
            3 => Waveform::Triangle,
            _ => Waveform::Noise,
        }
    }
}

// --- FILTER ---
#[derive(Clone, Copy)]
struct Filter {
    cutoff: f32,    // Hz
    resonance: f32, // 0.0 to 1.0 (Q)
    mode: FilterMode,
    // Internal state (Stereo)
    s1_l: f32, s2_l: f32,
    s1_r: f32, s2_r: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum FilterMode {
    LowPass = 0,
    HighPass = 1,
    BandPass = 2,
    Off = 3,
}

// --- ENVELOPE ---
#[derive(Clone, Copy)]
struct AdsrSettings {
    attack: f32,  // Seconds
    decay: f32,   // Seconds
    sustain: f32, // 0.0 to 1.0
    release: f32, // Seconds
}

enum EnvelopeStage {
    Attack,
    Decay,
    Sustain,
    Release,
    Idle,
}

struct Voice {
    note: u8,
    velocity: u8,
    phase: [f32; 3], // Phase for each oscillator
    
    // Envelope State
    env_stage: EnvelopeStage,
    env_level: f32,
    env_timer: f32, // Seconds elapsed in current stage
    release_start_level: f32, // Level when note-off happened
    
    is_active: bool,
}

impl Karbeatzer {
    pub fn new(sample_rate: Option<f32>) -> Self {
        Self {
            sample_rate: sample_rate.unwrap_or(48000.0),
            oscillators: [
                Oscillator { waveform: Waveform::Saw, detune: 0.0, mix: 1.0, pulse_width: 0.5 },
                Oscillator { waveform: Waveform::Square, detune: 0.1, mix: 0.5, pulse_width: 0.5 },
                Oscillator { waveform: Waveform::Sine, detune: -12.0, mix: 0.3, pulse_width: 0.5 },
            ],
            filter: Filter {
                cutoff: 2000.0,
                resonance: 0.2,
                mode: FilterMode::LowPass,
                s1_l: 0.0, s2_l: 0.0,
                s1_r: 0.0, s2_r: 0.0,
            },
            amp_env_settings: AdsrSettings {
                attack: 0.01,
                decay: 0.2,
                sustain: 0.7,
                release: 0.5,
            },
            drive: 0.0,
            active_voices: Vec::with_capacity(16),
            gain: 0.5,
            voice_buffer: Vec::with_capacity(512), // Pre-allocate
        }
    }

    /// Renders a block of audio for a single voice.
    /// `buffer` length determines the number of samples to generate.
    fn generate_voice_block(oscillators: &[Oscillator], sample_rate: f32, voice: &mut Voice, buffer: &mut [f32], amp_env: &AdsrSettings) {
        let block_size = buffer.len();
        let base_freq = 440.0 * 2.0_f32.powf((voice.note as f32 - 69.0) / 12.0);
        let dt = 1.0 / sample_rate;

        // Pre-calculate phase increments for efficiency
        let mut phase_incs = [0.0; 3];
        for (i, osc) in oscillators.iter().enumerate() {
            let freq = base_freq * 2.0_f32.powf(osc.detune / 12.0);
            phase_incs[i] = freq / sample_rate;
        }

        // Fill buffer
        for frame in 0..block_size {
            // 1. Envelope Logic (Per sample for smoothness, or could be per block for optimization)
            voice.env_timer += dt;
            
            // Inline envelope state machine for performance
            match voice.env_stage {
                EnvelopeStage::Attack => {
                    let rate = if amp_env.attack < 0.001 { 1000.0 } else { 1.0 / amp_env.attack };
                    voice.env_level = (voice.env_timer * rate).min(1.0);
                    if voice.env_level >= 1.0 {
                        voice.env_level = 1.0;
                        voice.env_stage = EnvelopeStage::Decay;
                        voice.env_timer = 0.0;
                    }
                }
                EnvelopeStage::Decay => {
                    let rate = if amp_env.decay < 0.001 { 1000.0 } else { 1.0 / amp_env.decay };
                    let progress = (voice.env_timer * rate).min(1.0);
                    voice.env_level = 1.0 - (progress * (1.0 - amp_env.sustain));
                    if progress >= 1.0 {
                        voice.env_level = amp_env.sustain;
                        voice.env_stage = EnvelopeStage::Sustain;
                    }
                }
                EnvelopeStage::Sustain => {
                    voice.env_level = amp_env.sustain;
                }
                EnvelopeStage::Release => {
                    let rate = if amp_env.release < 0.001 { 1000.0 } else { 1.0 / amp_env.release };
                    let progress = (voice.env_timer * rate).min(1.0);
                    voice.env_level = voice.release_start_level * (1.0 - progress);
                    if progress >= 1.0 {
                        voice.env_level = 0.0;
                        voice.is_active = false;
                        voice.env_stage = EnvelopeStage::Idle;
                    }
                }
                EnvelopeStage::Idle => {
                    voice.env_level = 0.0;
                    voice.is_active = false;
                }
            }

            if !voice.is_active {
                buffer[frame] = 0.0;
                continue;
            }

            let velocity_gain = voice.velocity as f32 / 127.0;
            let current_gain = velocity_gain * voice.env_level;

            let mut sample_accum = 0.0;

            // 2. Oscillator Summation
            for (i, osc) in oscillators.iter().enumerate() {
                let phase = voice.phase[i];
                
                let osc_out = match osc.waveform {
                    Waveform::Sine => (phase * 2.0 * PI).sin(),
                    Waveform::Saw => 2.0 * phase - 1.0,
                    Waveform::Square => if phase < osc.pulse_width { 1.0 } else { -1.0 },
                    Waveform::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
                    Waveform::Noise => fastrand::f32() * 2.0 - 1.0,
                };

                sample_accum += osc_out * osc.mix;

                // Advance phase
                voice.phase[i] += phase_incs[i];
                if voice.phase[i] >= 1.0 {
                    voice.phase[i] -= 1.0;
                }
            }

            buffer[frame] = sample_accum * current_gain;
        }
    }

    /// Processes a block of stereo interleaved audio through the filter
    fn process_filter_block(filter: &mut Filter, buffer: &mut [f32], sample_rate: f32) {
        if filter.mode == FilterMode::Off {
            return;
        }

        // Calculate coefficients once per block (Control Rate)
        let f = 2.0 * (PI * filter.cutoff / sample_rate).sin();
        let q = filter.resonance.clamp(0.0, 0.99);
        let damping = 2.0 * (1.0 - q);

        let frames = buffer.len() / 2;

        for i in 0..frames {
            let l_idx = i * 2;
            let r_idx = i * 2 + 1;

            let in_l = buffer[l_idx];
            let in_r = buffer[r_idx];

            // Left
            let lp_l = filter.s2_l + f * filter.s1_l;
            let hp_l = in_l - lp_l - damping * filter.s1_l;
            let bp_l = f * hp_l + filter.s1_l;
            filter.s1_l = bp_l;
            filter.s2_l = lp_l;

            // Right
            let lp_r = filter.s2_r + f * filter.s1_r;
            let hp_r = in_r - lp_r - damping * filter.s1_r;
            let bp_r = f * hp_r + filter.s1_r;
            filter.s1_r = bp_r;
            filter.s2_r = lp_r;

            match filter.mode {
                FilterMode::LowPass => {
                    buffer[l_idx] = lp_l;
                    buffer[r_idx] = lp_r;
                },
                FilterMode::HighPass => {
                    buffer[l_idx] = hp_l;
                    buffer[r_idx] = hp_r;
                },
                FilterMode::BandPass => {
                    buffer[l_idx] = bp_l;
                    buffer[r_idx] = bp_r;
                },
                _ => {}
            }
        }
    }
}

impl KarbeatGenerator for Karbeatzer {
    fn name(&self) -> &str {
        "Karbeatzer"
    }

    fn prepare(&mut self, sample_rate: f32, max_buffer_size: usize) {
        self.sample_rate = sample_rate;
        // Resize internal buffer to match max block size coming from host
        if self.voice_buffer.len() < max_buffer_size {
            self.voice_buffer.resize(max_buffer_size, 0.0);
        }
    }

    fn reset(&mut self) {
        self.active_voices.clear();
        self.filter.s1_l = 0.0; self.filter.s2_l = 0.0;
        self.filter.s1_r = 0.0; self.filter.s2_r = 0.0;
    }

    fn process(&mut self, output_buffer: &mut [f32], midi_events: &[MidiEvent]) {
        // 1. Clear Output Buffer
        output_buffer.fill(0.0);
        
        let total_frames = output_buffer.len() / 2; // Stereo frames
        let mut current_frame = 0;
        let mut event_idx = 0;

        // Block Processing Loop: Split the buffer based on MIDI event timestamps
        while current_frame < total_frames {
            // Determine the next event timestamp or end of buffer
            let next_event_frame = if event_idx < midi_events.len() {
                midi_events[event_idx].sample_offset as usize
            } else {
                total_frames
            };

            // Ensure we don't go backwards or past end
            let end_frame = next_event_frame.min(total_frames);
            let block_len = end_frame - current_frame;

            // A. Render Audio for this sub-block [current_frame .. end_frame]
            if block_len > 0 {
                // We use slices of the output buffer directly to accumulate voice data
                let out_slice = &mut output_buffer[current_frame * 2 .. end_frame * 2];
                
                // For each active voice, generate a block and mix it in
                for voice in self.active_voices.iter_mut() {
                    if !voice.is_active { continue; }

                    // Use internal scratch buffer to generate mono voice block
                    // We only need 'block_len' samples
                    let scratch = &mut self.voice_buffer[0..block_len];
                    
                    Karbeatzer::generate_voice_block(&self.oscillators, self.sample_rate, voice, scratch, &self.amp_env_settings);

                    // Mix mono voice into stereo output slice
                    for (i, &sample) in scratch.iter().enumerate() {
                        out_slice[i*2] += sample;     // L
                        out_slice[i*2 + 1] += sample; // R
                    }
                }

                // Apply Global Filter to this sub-block
                Karbeatzer::process_filter_block(&mut self.filter, out_slice, self.sample_rate);

                // Apply Drive to this sub-block
                if self.drive > 0.0 {
                    let drive_amt = 1.0 + self.drive * 4.0;
                    for sample in out_slice.iter_mut() {
                        *sample = (*sample * drive_amt).tanh();
                    }
                }
                
                // Apply Global Gain
                for sample in out_slice.iter_mut() {
                    *sample *= self.gain;
                }
            }

            // B. Process MIDI Event(s) at 'end_frame'
            while event_idx < midi_events.len() && midi_events[event_idx].sample_offset as usize == end_frame {
                match midi_events[event_idx].data {
                    MidiMessage::NoteOn { key, velocity } => {
                        if velocity > 0 {
                            self.active_voices.push(Voice {
                                note: key,
                                velocity,
                                phase: [0.0; 3],
                                env_stage: EnvelopeStage::Attack,
                                env_level: 0.0,
                                env_timer: 0.0,
                                release_start_level: 0.0,
                                is_active: true,
                            });
                        } else {
                            // Note Off (Vel 0)
                            for v in self.active_voices.iter_mut() {
                                if v.note == key && v.is_active {
                                    v.env_stage = EnvelopeStage::Release;
                                    v.env_timer = 0.0;
                                    v.release_start_level = v.env_level;
                                }
                            }
                        }
                    },
                    MidiMessage::NoteOff { key } => {
                         for v in self.active_voices.iter_mut() {
                            if v.note == key && v.is_active {
                                v.env_stage = EnvelopeStage::Release;
                                v.env_timer = 0.0;
                                v.release_start_level = v.env_level;
                            }
                        }
                    },
                    _ => {}
                }
                event_idx += 1;
            }

            current_frame = end_frame;
        }

        // Cleanup Inactive Voices (Once per process call)
        self.active_voices.retain(|v| v.is_active);
    }

    fn set_parameter(&mut self, id: u32, value: f32) {
        match id {
            // Global Gain
            0 => self.gain = value,
            
            // Filter
            1 => self.filter.cutoff = value.clamp(20.0, 20000.0),
            2 => self.filter.resonance = value.clamp(0.0, 0.95),
            3 => self.filter.mode = match value as u32 {
                0 => FilterMode::LowPass,
                1 => FilterMode::HighPass,
                2 => FilterMode::BandPass,
                _ => FilterMode::Off,
            },

            // ADSR
            4 => self.amp_env_settings.attack = value.clamp(0.001, 5.0),
            5 => self.amp_env_settings.decay = value.clamp(0.001, 5.0),
            6 => self.amp_env_settings.sustain = value.clamp(0.0, 1.0),
            7 => self.amp_env_settings.release = value.clamp(0.001, 10.0),

            // FX
            8 => self.drive = value.clamp(0.0, 1.0),

            // Osc 1
            10 => self.oscillators[0].waveform = Waveform::from(value),
            11 => self.oscillators[0].detune = value,
            12 => self.oscillators[0].mix = value,
            13 => self.oscillators[0].pulse_width = value.clamp(0.01, 0.99),

            // Osc 2
            20 => self.oscillators[1].waveform = Waveform::from(value),
            21 => self.oscillators[1].detune = value,
            22 => self.oscillators[1].mix = value,
            23 => self.oscillators[1].pulse_width = value.clamp(0.01, 0.99),

            // Osc 3
            30 => self.oscillators[2].waveform = Waveform::from(value),
            31 => self.oscillators[2].detune = value,
            32 => self.oscillators[2].mix = value,
            33 => self.oscillators[2].pulse_width = value.clamp(0.01, 0.99),

            _ => {}
        }
    }

    fn get_parameter(&self, id: u32) -> f32 {
        match id {
            0 => self.gain,
            1 => self.filter.cutoff,
            2 => self.filter.resonance,
            3 => self.filter.mode as u32 as f32,
            
            4 => self.amp_env_settings.attack,
            5 => self.amp_env_settings.decay,
            6 => self.amp_env_settings.sustain,
            7 => self.amp_env_settings.release,

            8 => self.drive,

            10 => self.oscillators[0].waveform as u32 as f32,
            11 => self.oscillators[0].detune,
            12 => self.oscillators[0].mix,
            13 => self.oscillators[0].pulse_width,

            20 => self.oscillators[1].waveform as u32 as f32,
            21 => self.oscillators[1].detune,
            22 => self.oscillators[1].mix,
            23 => self.oscillators[1].pulse_width,

            30 => self.oscillators[2].waveform as u32 as f32,
            31 => self.oscillators[2].detune,
            32 => self.oscillators[2].mix,
            33 => self.oscillators[2].pulse_width,

            _ => 0.0
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}