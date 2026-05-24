use karbeat_dsp::compressor::SidechainCompressor;
use karbeat_macros::karbeat_plugin;
use karbeat_plugin_api::prelude::*;

/// An audio FX to do sidechain compressing,
/// Meaning that the compression is influenced
/// by another input signal
#[derive(Clone)]
#[karbeat_plugin]
pub struct DigidawSidechainCompressor {
    #[nested(prefix = "compressor/")]
    pub compressor: SidechainCompressor,
}

impl Default for DigidawSidechainCompressor {
    fn default() -> Self {
        Self {
            compressor: SidechainCompressor::default(),
        }
    }
}

#[karbeat_macros::auto_param]
impl AudioPlugin for DigidawSidechainCompressor {
    fn name(&self) -> &str {
        "DigiDAW Sidechain Compressor"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::Effect
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize, _max_buffer_size: usize) {
        self.compressor.prepare(sample_rate as f64);
    }

    fn reset(&mut self) {
        self.compressor.prepare(48000.0);
    }

    fn process(&mut self, buffer: &mut [f32], context: &ProcessContext) {
        let aux = context.aux_buffer.get();

        for i in 0..buffer.len() {
            let main_sample = buffer[i];

            // Extract the sidechain signal, defaulting to 0.0 if not routed
            let sc_sample = aux.map_or(0.0, |a| a.get(i).copied().unwrap_or(0.0));

            buffer[i] = self.compressor.process_sample(main_sample, sc_sample);
        }
    }
}
