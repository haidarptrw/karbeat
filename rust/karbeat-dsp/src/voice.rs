use serde::{Deserialize, Serialize};

use crate::envelope::AdsrProcessor;

#[derive(Clone, Serialize, Deserialize)]
pub struct SynthVoice {
    pub note: u8,
    pub velocity: u8,
    pub is_active: bool,
    pub phase: Vec<f64>,
    pub adsr: AdsrProcessor, // Every voice tracks its own ADSR state
}

impl SynthVoice {
    pub fn new(note: u8, velocity: u8, sample_rate: f32, num_voices: usize) -> Self {
        let mut adsr = AdsrProcessor::new(sample_rate);
        adsr.note_on(); // Start the attack phase immediately

        Self {
            note,
            velocity,
            is_active: true,
            phase: vec![0.0; num_voices],
            adsr,
        }
    }

    pub fn release(&mut self) {
        self.adsr.note_off(); // Trigger the graceful fade-out
    }
}
