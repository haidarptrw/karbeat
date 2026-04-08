use karbeat_macros::EnumParam;

#[derive(Clone, Copy, PartialEq, Debug, Default, EnumParam)]
#[repr(usize)]
pub enum SimpleFilterMode {
    #[default]
    LowPass = 0,
    HighPass = 1,
    BandPass = 2,
    Off = 3,
}

impl From<f32> for SimpleFilterMode {
    fn from(v: f32) -> Self {
        match v as u32 {
            0 => SimpleFilterMode::LowPass,
            1 => SimpleFilterMode::HighPass,
            2 => SimpleFilterMode::BandPass,
            _ => SimpleFilterMode::Off,
        }
    }
}