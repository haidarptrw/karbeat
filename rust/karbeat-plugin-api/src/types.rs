pub struct MidiEvent {
    pub sample_offset: usize,
    pub data: MidiMessage,
}

pub enum MidiMessage {
    NoteOn { key: u8, velocity: u8 },
    NoteOff { key: u8 },
    ControlChange { controller: u8, value: u8 },
    PitchBend { value: i16 },
}

/// Tells the DAW how to route audio/MIDI to this plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginCategory {
    /// Primarily generates audio from MIDI (Synths, Samplers)
    Instrument,
    /// Primarily processes incoming audio (Reverb, EQ, Compressors)
    Effect,
    /// Processes audio but also relies heavily on MIDI (Vocoders, Auto-Tune)
    MidiEffect,
}

pub struct ProcessContext<'a> {
    pub bpm: f32,
    pub time_sig_numerator: u8,
    pub time_sig_denominator: u8,
    pub is_playing: bool,
    pub sample_position: u64,
    pub midi_events: &'a [MidiEvent],
}
