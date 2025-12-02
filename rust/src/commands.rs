use crate::core::track::audio_waveform::AudioWaveform;

pub enum AudioCommand {
    PlayOneShot(AudioWaveform),
    StopAllPreviews,
}