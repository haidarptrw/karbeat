use crate::{ audio::event::PluginTarget, core::project::automation::AutomationTarget };

#[derive(Clone, Debug)]
pub enum ModulationEvent {
    /// Driven by the audio output of a specific plugin (e.g., Peak Controller)
    PeakController {
        source: PluginTarget,
        target: AutomationTarget,
        depth: f32,
        base_value: f32,
    },
    /// Driven by a timeline automation lane
    Automation {
        lane_id: u32,
        target: AutomationTarget,
    },
    /// Driven by a mathematical oscillator (LFO)
    LFO {
        rate_hz: f32,
        depth: f32,
        base_value: f32,
        target: AutomationTarget,
    },
}
