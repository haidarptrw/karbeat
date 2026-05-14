use karbeat_macros::karbeat_plugin;

/// An audio FX to do sidechain compressing,
/// Meaning that the compression is influenced
/// by another input signal
#[karbeat_plugin]
pub struct SidechainCompressor {
    #[param(
        id = "gate_threshold",
        name = "Gate Threshold",
        group = "Gate",
        min = -60.0,
        max = 12.0,
        step = 0.1,
        default = 0.0
    )]
    pub gate_threshold: f32,

    // Expander parameters
    #[param(
        id = "expander_threshold",
        name = "Expander Threshold",
        group = "Expander",
        min = -60.0,
        max = 12.0,
        step = 0.1,
        default = 0.0
    )]
    pub expander_threshold: f32,
    #[param(
        id = "expander_ratio",
        name = "Expander Ratio",
        group = "Expander",
        min = 0.0,
        max = 16.0,
        step = 0.1,
        default = 1.0
    )]
    pub expander_ratio: f32,

    // Compressor parameters
    #[param(
        id = "compressor_threshold",
        name = "Compressor Threshold",
        group = "Compressor",
        min = -60.0,
        max = 12.0,
        step = 0.1,
        default = 0.0
    )]
    pub compressor_threshold: f32,

    #[param(
        id = "compressor_ratio",
        name = "Compressor Ratio",
        group = "Compressor",
        min = 0.0,
        max = 16.0,
        step = 0.1,
        default = 1.0
    )]
    pub compressor_ratio: f32,
}
