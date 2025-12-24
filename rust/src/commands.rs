use crate::core::{project::Pattern, track::audio_waveform::AudioWaveform};

pub enum AudioCommand {
    PlayOneShot(AudioWaveform),
    StopAllPreviews,
    ResetPlayhead,
    SetPlayhead(u32),
    PlayPreviewNote {note_key: u8, generator_id: u32, velocity: u8, is_note_on: bool},
    /// Set BPM to the field0 value
    SetBPM(f32),
}