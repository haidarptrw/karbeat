use crate::plugin::wrapper::{EffectWrapper, RawEffectEngine};



/// Node of each filter
#[derive(Clone, Copy)]
pub struct KarbeatParametricEQFilterNode {
    /// Q value of this filter node
    pub q: f32,
    /// Order of the filter
    pub order: u16, // starts from order 0

    /// Indicates whether this node is effective or not
    pub active: bool,

    /// Amplitude of this node
    pub amplitude: f32,

    /// Frequency of this node
    pub freq: f32,
}

#[derive(Clone)]
pub struct KarbeatParametricEQEngine {
    pub nodes: Vec<KarbeatParametricEQFilterNode>
}

impl KarbeatParametricEQEngine {
    pub fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(10) // 10 sek
        }
    }
}

impl RawEffectEngine for KarbeatParametricEQEngine {
    fn process(&mut self, base: &mut crate::plugin::effect_base::EffectBase, buffer: &mut [f32]) {
        todo!()
    }

    fn reset(&mut self) {
        todo!()
    }

    fn set_custom_parameter(&mut self, id: u32, value: f32) {
        todo!()
    }

    fn get_custom_parameter(&self, id: u32) -> Option<f32> {
        todo!()
    }

    fn custom_default_parameters() -> std::collections::HashMap<u32, f32>
    where
        Self: Sized {
        todo!()
    }

    fn name() -> &'static str
    where
        Self: Sized {
        todo!()
    }
}

pub type KarbeatParametricEQ = EffectWrapper<KarbeatParametricEQEngine>;

pub fn create_parametric_eq(sample_rate: Option<f32>) -> EffectWrapper<KarbeatParametricEQEngine> {
    EffectWrapper::new(KarbeatParametricEQEngine::new(), sample_rate.unwrap_or(48000.0))
}