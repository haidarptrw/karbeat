#[derive(Clone, Copy, Debug)]
pub struct PlaybackPosition {
    pub samples: u64,
    pub beat: usize,
    pub bar: usize,
    pub tempo: f32, // Useful for Flutter to interpolate movement
    pub sample_rate: u32,
    pub is_playing: bool,
}