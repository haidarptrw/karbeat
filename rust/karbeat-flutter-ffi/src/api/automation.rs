use std::collections::HashMap;

use karbeat_core::{
    api::automation_api,
    core::project::{
        AutomationCurveType, AutomationLane, AutomationPoint, AutomationTarget,
        EffectAutomationTarget, MixerChannelParamTarget, ModulationLink,
        ModulationLinkForOrderedLaneView, ModulationSource, TrackAutomationTarget,
    },
    shared::{BusId, EffectId, TrackId},
};

use crate::api::plugin::UiPluginTarget;

#[derive(Clone, Debug)]
pub struct AutomationLaneDto {
    pub id: u32,
    pub label: String,
    pub points: Vec<AutomationPointDto>,
    pub enabled: bool,
    pub min: f32,
    pub max: f32,
    pub default_value: f32,
}

#[derive(Clone, Debug)]
pub struct AutomationPointDto {
    pub time_ticks: u32,
    pub value: f32,
    pub curve_type: AutomationCurveTypeDto,
    pub tension: f32,
}

#[derive(Clone, Debug)]
pub enum AutomationTargetDto {
    Track {
        track_id: u32,
        track_target: TrackAutomationTargetDto,
    },
    Bus {
        bus_id: u32,
        mix_target: MixerChannelParamTargetDto,
    },
    Master(MixerChannelParamTargetDto),
    TempoBpm,
}

#[derive(Clone, Debug)]
pub enum TrackAutomationTargetDto {
    Generator { param_id: u32 },
    MixerChannel(MixerChannelParamTargetDto),
}

#[derive(Clone, Debug)]
pub enum MixerChannelParamTargetDto {
    Volume,
    Pan,
    Plugin {
        effect_id: u32,
        target: EffectAutomationTargetDto,
    },
}

#[derive(Clone, Debug)]
pub enum EffectAutomationTargetDto {
    Mix,
    PluginParam { param_id: u32 },
}

#[derive(Clone, Debug)]
pub enum AutomationCurveTypeDto {
    Linear,
    Exponential,
    Step,
}

pub struct ModulationLinkDto {
    pub id: u32,
    pub source_id: u32,              // Which LFO/Macro is driving this?
    pub target: AutomationTargetDto, // What parameter is being turned?
    pub depth: f32,                  // How much is it turning? (-1.0 to 1.0)
    pub base_value: f32,
    pub order_idx: usize,
}

pub enum ModulationSourceDto {
    PeakController { source: UiPluginTarget },
    Automation { lane_id: u32 },
    Lfo { rate_hz: f32 },
}

impl From<&ModulationSource> for ModulationSourceDto {
    fn from(value: &ModulationSource) -> Self {
        match value {
            ModulationSource::PeakController { source } => Self::PeakController {
                source: source.into(),
            },
            ModulationSource::Automation { lane_id } => Self::Automation {
                lane_id: lane_id.to_u32(),
            },
            ModulationSource::LFO { rate_hz } => Self::Lfo { rate_hz: *rate_hz },
        }
    }
}

impl From<ModulationSourceDto> for ModulationSource {
    fn from(value: ModulationSourceDto) -> Self {
        match value {
            ModulationSourceDto::PeakController { source } => Self::PeakController {
                source: source.into(),
            },
            ModulationSourceDto::Automation { lane_id } => Self::Automation {
                lane_id: lane_id.into(),
            },
            ModulationSourceDto::Lfo { rate_hz } => Self::LFO { rate_hz },
        }
    }
}

impl From<&ModulationLinkForOrderedLaneView> for ModulationLinkDto {
    fn from(value: &ModulationLinkForOrderedLaneView) -> Self {
        Self {
            id: value.prop.id.into(),
            source_id: value.prop.source_id.into(),
            target: AutomationTargetDto::from(&value.prop.target),
            depth: value.prop.depth,
            base_value: value.prop.base_value,
            order_idx: value.order_idx,
        }
    }
}

impl From<ModulationLinkDto> for ModulationLinkForOrderedLaneView {
    fn from(value: ModulationLinkDto) -> Self {
        Self {
            order_idx: value.order_idx,
            prop: ModulationLink {
                id: value.id.into(),
                source_id: value.source_id.into(),
                target: value.target.into(),
                depth: value.depth,
                base_value: value.base_value,
            },
        }
    }
}

impl From<AutomationCurveType> for AutomationCurveTypeDto {
    fn from(curve: AutomationCurveType) -> Self {
        match curve {
            AutomationCurveType::Linear => Self::Linear,
            AutomationCurveType::Exponential => Self::Exponential,
            AutomationCurveType::Step => Self::Step,
        }
    }
}

impl From<AutomationCurveTypeDto> for AutomationCurveType {
    fn from(value: AutomationCurveTypeDto) -> Self {
        match value {
            AutomationCurveTypeDto::Linear => Self::Linear,
            AutomationCurveTypeDto::Exponential => Self::Exponential,
            AutomationCurveTypeDto::Step => Self::Step,
        }
    }
}

impl From<AutomationPointDto> for AutomationPoint {
    fn from(point: AutomationPointDto) -> Self {
        Self {
            time_ticks: point.time_ticks,
            value: point.value,
            curve_type: point.curve_type.into(),
            tension: point.tension.into(),
        }
    }
}

impl From<AutomationPoint> for AutomationPointDto {
    fn from(p: AutomationPoint) -> Self {
        Self {
            time_ticks: p.time_ticks,
            value: p.value,
            curve_type: p.curve_type.into(),
            tension: p.tension.get(),
        }
    }
}

impl From<AutomationLaneDto> for AutomationLane {
    fn from(l: AutomationLaneDto) -> Self {
        Self {
            id: l.id.into(),
            label: l.label,
            points: l.points.into_iter().map(|p| p.into()).collect(),
            enabled: l.enabled,
            min: l.min,
            max: l.max,
            default_value: l.default_value,
        }
    }
}

impl From<&AutomationLane> for AutomationLaneDto {
    fn from(l: &AutomationLane) -> Self {
        Self {
            id: l.id.into(),
            label: l.label.clone(),
            points: l.points.iter().map(|p| p.to_owned().into()).collect(),
            enabled: l.enabled,
            min: l.min,
            max: l.max,
            default_value: l.default_value,
        }
    }
}

impl From<&EffectAutomationTarget> for EffectAutomationTargetDto {
    fn from(target: &EffectAutomationTarget) -> Self {
        match target {
            EffectAutomationTarget::Mix => Self::Mix,
            EffectAutomationTarget::PluginParam { param_id } => Self::PluginParam {
                param_id: *param_id,
            },
        }
    }
}

impl From<&MixerChannelParamTarget> for MixerChannelParamTargetDto {
    fn from(target: &MixerChannelParamTarget) -> Self {
        match target {
            MixerChannelParamTarget::Volume => Self::Volume,
            MixerChannelParamTarget::Pan => Self::Pan,
            MixerChannelParamTarget::Plugin { effect_id, target } => Self::Plugin {
                effect_id: effect_id.to_u32(),
                target: EffectAutomationTargetDto::from(target),
            },
        }
    }
}

impl From<&TrackAutomationTarget> for TrackAutomationTargetDto {
    fn from(target: &TrackAutomationTarget) -> Self {
        match target {
            TrackAutomationTarget::Generator { param_id } => Self::Generator {
                param_id: *param_id,
            },
            TrackAutomationTarget::MixerChannel(target) => {
                Self::MixerChannel(MixerChannelParamTargetDto::from(target))
            }
        }
    }
}

impl From<TrackAutomationTargetDto> for TrackAutomationTarget {
    fn from(dto: TrackAutomationTargetDto) -> Self {
        match dto {
            TrackAutomationTargetDto::Generator { param_id } => Self::Generator { param_id },
            TrackAutomationTargetDto::MixerChannel(target) => {
                Self::MixerChannel(MixerChannelParamTarget::from(target))
            }
        }
    }
}

impl From<&AutomationTarget> for AutomationTargetDto {
    fn from(target: &AutomationTarget) -> Self {
        match target {
            AutomationTarget::Track {
                track_id,
                track_target,
            } => Self::Track {
                track_id: track_id.to_u32(),
                track_target: TrackAutomationTargetDto::from(track_target),
            },
            AutomationTarget::Bus { bus_id, mix_target } => Self::Bus {
                bus_id: bus_id.to_u32(),
                mix_target: MixerChannelParamTargetDto::from(mix_target),
            },
            AutomationTarget::Master(mix_target) => {
                Self::Master(MixerChannelParamTargetDto::from(mix_target))
            }
            AutomationTarget::TempoBpm => Self::TempoBpm,
        }
    }
}

// ============================================================================
// CONVERSIONS: UI DTO -> Core Domain (Dart to Rust)
// ============================================================================

impl From<EffectAutomationTargetDto> for EffectAutomationTarget {
    fn from(dto: EffectAutomationTargetDto) -> Self {
        match dto {
            EffectAutomationTargetDto::Mix => Self::Mix,
            EffectAutomationTargetDto::PluginParam { param_id } => Self::PluginParam { param_id },
        }
    }
}

impl From<MixerChannelParamTargetDto> for MixerChannelParamTarget {
    fn from(dto: MixerChannelParamTargetDto) -> Self {
        match dto {
            MixerChannelParamTargetDto::Volume => Self::Volume,
            MixerChannelParamTargetDto::Pan => Self::Pan,
            MixerChannelParamTargetDto::Plugin { effect_id, target } => Self::Plugin {
                effect_id: EffectId::from(effect_id),
                target: EffectAutomationTarget::from(target),
            },
        }
    }
}

impl From<AutomationTargetDto> for AutomationTarget {
    fn from(dto: AutomationTargetDto) -> Self {
        match dto {
            AutomationTargetDto::Track {
                track_id,
                track_target,
            } => Self::Track {
                track_id: TrackId::from(track_id),
                track_target: TrackAutomationTarget::from(track_target),
            },
            AutomationTargetDto::Bus { bus_id, mix_target } => Self::Bus {
                bus_id: BusId::from(bus_id),
                mix_target: MixerChannelParamTarget::from(mix_target),
            },
            AutomationTargetDto::Master(mix_target) => {
                Self::Master(MixerChannelParamTarget::from(mix_target))
            }
            AutomationTargetDto::TempoBpm => Self::TempoBpm,
        }
    }
}

/// Fetch the list of (modulation_id, automation_id, automation_lane) where
/// the target is the given track id
pub fn get_automation_lanes_for_track(track_id: u32) -> Vec<(u32, u32, AutomationLaneDto)> {
    automation_api::get_automation_lanes_for_track(track_id.into())
        .into_iter()
        .map(|(mod_id, automation_id, arc_lane)| {
            let lane = arc_lane.as_ref().into();
            (mod_id.into(), automation_id.into(), lane)
        })
        .collect()
}

/// Fetch the list of (modulation_id, automation_id, automation_lane) where
/// the target is the given bus id
pub fn get_automation_lanes_for_bus(bus_id: u32) -> Vec<(u32, u32, AutomationLaneDto)> {
    automation_api::get_automation_lanes_for_bus(bus_id.into())
        .into_iter()
        .map(|(mod_id, automation_id, arc_lane)| {
            let lane = arc_lane.as_ref().into();
            (mod_id.into(), automation_id.into(), lane)
        })
        .collect()
}

pub fn add_automation_lane(
    target: AutomationTargetDto,
    label: &str,
    min: f32,
    max: f32,
    default_value: f32,
) -> Result<AutomationLaneDto, String> {
    match automation_api::add_automation_lane(target.into(), label, min, max, default_value) {
        Ok(arc_lane) => {
            let lane_dto = AutomationLaneDto::from(arc_lane.as_ref());
            Ok(lane_dto)
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Fetch all automation lanes across all targets
pub fn get_automations_lanes_all() -> HashMap<u32, AutomationLaneDto> {
    automation_api::get_automations_lanes_all(|lane| (lane.id.into(), AutomationLaneDto::from(lane)))
}

pub fn add_automation_lane_for_track(
    track_id: u32,
    target: AutomationTargetDto,
    label: &str,
    min: f32,
    max: f32,
    default_value: f32,
) -> Result<AutomationLaneDto, String> {
    match automation_api::add_automation_lane_for_track(
        track_id.into(),
        target.into(),
        label,
        min,
        max,
        default_value,
    ) {
        Ok(arc_lane) => {
            let lane_dto = AutomationLaneDto::from(arc_lane.as_ref());
            Ok(lane_dto)
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn add_automation_lane_for_bus(
    bus_id: u32,
    target: AutomationTargetDto,
    label: &str,
    min: f32,
    max: f32,
    default_value: f32,
) -> Result<AutomationLaneDto, String> {
    match automation_api::add_automation_lane_for_bus(
        bus_id.into(),
        target.into(),
        label,
        min,
        max,
        default_value,
    ) {
        Ok(arc_lane) => {
            let lane_dto = AutomationLaneDto::from(arc_lane.as_ref());
            Ok(lane_dto)
        }
        Err(e) => Err(e.to_string()),
    }
}

// pub fn remove_automation_lane(automation_id: u32) -> Result<(), String> {
//     automation_api::remove_automation_lane(automation_id.into()).map_err(|e| e.to_string())
// }

pub fn add_new_automation_point(
    automation_id: u32,
    time_ticks: u32,
    value: f32,
) -> Result<AutomationPointDto, String> {
    automation_api::add_new_automation_point(automation_id.into(), time_ticks, value)
        .map(|point| point.into())
        .map_err(|e| e.to_string())
}

pub fn remove_automation_point(automation_id: u32, index: usize) -> Result<(), String> {
    automation_api::remove_automation_point(automation_id.into(), index).map_err(|e| e.to_string())
}

pub fn update_automation_point(
    automation_id: u32,
    index: usize,
    time_ticks: u32,
    value: f32,
    tension: f32,
) -> Result<usize, String> {
    automation_api::update_automation_point(automation_id.into(), index, time_ticks, value, tension)
        .map_err(|e| e.to_string())
}

// ▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱
// Modulation API
// ▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱

/// Get all modulations in the project
pub fn get_all_linked_modulation_params() -> HashMap<u32, ModulationLinkDto> {
    automation_api::get_all_linked_modulation_params(|id, mod_link| (id.to_u32(), mod_link.into()))
}

/// Add generic modulation source
pub fn add_modulation_source(source: ModulationSourceDto) -> u32 {
    automation_api::add_modulation_source(source.into()).to_u32()
}

/// Remove the modulation source. This function also cascade delete all link
/// with this source
pub fn remove_modulation_source(mod_id: u32) {
    automation_api::remove_modulation_source(mod_id.into());
}

/// Remove modulation link based on queried modulation link id
pub fn remove_modulation_link(mod_link_id: u32) {
    automation_api::remove_modulation_link(mod_link_id.into());
}

/// Link the target param to a modulation source
pub fn link_this_param_to_controller(
    source_id: u32,
    target: AutomationTargetDto,
    depth: f32,
    base_value: f32,
) -> Result<u32, String> {
    automation_api::link_this_param_to_controller(
        source_id.into(),
        target.into(),
        depth,
        base_value,
    )
    .map_err(|e| e.to_string())
    .and_then(|v| Ok(v.to_u32()))
}
