use karbeat_macros::karbeat_plugin;


#[karbeat_plugin]
pub struct PeakController {
    #[param(id="gate_threshold", name="Gate Threshold", group="Gate", min=0.001, max=0.999, step=0.1, default=0.2)]
    pub gate_threshold: f32,

    // Expander parameters
    #[param(id="expander_threshold", name="Expander Threshold", )]
    pub expander_threshold: f32,
    pub expander_ratio: f32,
}