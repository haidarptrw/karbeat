use std::collections::HashMap;
use std::time::Duration;

use flutter_rust_bridge::frb;
use karbeat_core::shared::id::*;
pub use karbeat_core::{
    core::project::TrackId,
    plugin_types::{ParameterSpec, ParameterValueType},
};

use crate::frb_generated::StreamSink;
use karbeat_core::api::mixer_api;
use karbeat_core::commands::MixerChannelTarget;
use karbeat_core::core::project::mixer::{
    BusMixerChannel, EffectInstance, MixerChannel, MixerChannelParams, MixerState,
    RoutingConnection, RoutingNode,
};

// ======================================
// Type Definitions
// ======================================

// ======================================
// MixerChannelTarget DTO
// ======================================

/// UI-facing mixer channel target — identifies which channel to address.
#[frb]
pub enum UiMixerChannelTarget {
    Track(u32),
    Bus(u32),
    Master,
}

impl From<&UiMixerChannelTarget> for MixerChannelTarget {
    fn from(val: &UiMixerChannelTarget) -> Self {
        match val {
            UiMixerChannelTarget::Track(id) => MixerChannelTarget::Track(TrackId::from(*id)),
            UiMixerChannelTarget::Bus(id) => MixerChannelTarget::Bus(BusId::from(*id)),
            UiMixerChannelTarget::Master => MixerChannelTarget::Master,
        }
    }
}

/// Full DSP state snapshot of a mixer channel, polled via
/// poll_mixer_channel_feedback() after calling query_mixer_channel().
#[derive(Clone)]
pub struct UiMixerChannelSnapshot {
    pub track_id: u32, // u32::MAX for buses, u32::MAX - 1 for master
    pub bus_id: Option<u32>,
    pub is_master: bool,
    pub volume: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub inverted_phase: bool,
}

/// UI representation of a mixer channel.
pub struct UiMixerChannel {
    pub volume: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub inverted_phase: bool,
    /// List of effect summaries (ID and name).
    pub effects: Vec<UiEffectSummary>,
}

pub struct UiEffectSummary {
    pub id: u32,
    pub registry_id: u32,
    pub name: String,
}

impl From<&MixerChannel> for UiMixerChannel {
    fn from(value: &MixerChannel) -> Self {
        Self {
            // Volume is in dB (both UI and backend use dB)
            volume: value.volume.get(),
            pan: value.pan.get(),
            mute: value.mute,
            solo: value.solo,
            inverted_phase: value.inverted_phase,
            effects: value
                .effects
                .iter()
                .map(|instance| UiEffectSummary {
                    id: instance.id.to_u32(),
                    registry_id: instance.instance.registry_id,
                    name: instance.instance.name.clone(),
                })
                .collect(),
        }
    }
}

/// UI representation of a mixer bus.
pub struct UiBus {
    pub id: u32,
    pub name: String,
    pub channel: UiMixerChannel,
}

impl From<&BusMixerChannel> for UiBus {
    fn from(value: &BusMixerChannel) -> Self {
        Self {
            id: value.id.to_u32(),
            name: value.name.clone(),
            channel: (&value.channel).into(),
        }
    }
}

/// UI representation of a routing connection.
pub struct UiRoutingConnection {
    pub source: UiRoutingNode,
    pub destination: UiRoutingNode,
    pub send_level: f32,
    pub is_send: bool,
}

impl From<&RoutingConnection> for UiRoutingConnection {
    fn from(value: &RoutingConnection) -> Self {
        Self {
            source: (&value.source).into(),
            destination: (&value.destination).into(),
            send_level: value.send_level,
            is_send: value.is_send,
        }
    }
}

/// UI DTO describing a routing node (Track, Bus, Master).
#[frb]
pub enum UiRoutingNode {
    Track(u32),
    Bus(u32),
    Master,
    PluginSidechain, // this actually useless because the UI does not need this
}

impl From<&RoutingNode> for UiRoutingNode {
    fn from(value: &RoutingNode) -> Self {
        match value {
            RoutingNode::Track(id) => UiRoutingNode::Track(id.to_u32()),
            RoutingNode::Bus(id) => UiRoutingNode::Bus(id.to_u32()),
            RoutingNode::Master => UiRoutingNode::Master,
            _ => Self::PluginSidechain,
        }
    }
}

impl From<&UiRoutingNode> for RoutingNode {
    fn from(value: &UiRoutingNode) -> Self {
        match value {
            UiRoutingNode::Track(id) => RoutingNode::Track((*id).into()),
            UiRoutingNode::Bus(id) => RoutingNode::Bus(BusId::from(*id)),
            UiRoutingNode::Master => RoutingNode::Master,
            _ => RoutingNode::Master,
        }
    }
}

/// UI representation of the mixer state.
pub struct UiMixerState {
    pub channels: HashMap<u32, UiMixerChannel>,
    pub master_bus: UiMixerChannel,
    pub buses: HashMap<u32, UiBus>,
    pub routing: Vec<UiRoutingConnection>,
}

impl From<&MixerState> for UiMixerState {
    fn from(value: &MixerState) -> Self {
        Self {
            channels: value
                .channels
                .iter()
                .map(|(id, channel)| (id.to_u32(), UiMixerChannel::from(&channel.channel)))
                .collect(),
            master_bus: value.master_bus.as_ref().into(),
            buses: value
                .buses
                .iter()
                .map(|(id, bus)| (id.to_u32(), bus.as_ref().into()))
                .collect(),
            routing: value.routing.iter().map(|c| c.into()).collect(),
        }
    }
}

pub struct UiEffectInstance {
    pub id: u32,
    pub name: String,
}

impl From<&EffectInstance> for UiEffectInstance {
    fn from(value: &EffectInstance) -> Self {
        Self {
            id: value.id.to_u32(),
            name: value.instance.name.clone(),
        }
    }
}

impl UiMixerState {
    #[frb(sync)]
    pub fn new() -> Self {
        Self::from(&MixerState::default())
    }

    #[frb(sync)]
    pub fn new_with_param(
        channels: HashMap<u32, UiMixerChannel>,
        master_bus: UiMixerChannel,
        buses: HashMap<u32, UiBus>,
        routing: Vec<UiRoutingConnection>,
    ) -> Self {
        Self {
            channels,
            master_bus,
            buses,
            routing,
        }
    }
}

pub enum UiMixerChannelParams {
    Volume(f32),
    Pan(f32),
    Mute(bool),
    InvertedPhase(bool),
    Solo(bool),
}

impl From<&MixerChannelParams> for UiMixerChannelParams {
    fn from(params: &MixerChannelParams) -> Self {
        match params {
            MixerChannelParams::Volume(value) => Self::Volume(*value),
            MixerChannelParams::Pan(value) => Self::Pan(*value),
            MixerChannelParams::Mute(value) => Self::Mute(*value),
            MixerChannelParams::InvertedPhase(value) => Self::InvertedPhase(*value),
            MixerChannelParams::Solo(value) => Self::Solo(*value),
        }
    }
}

impl From<&UiMixerChannelParams> for MixerChannelParams {
    fn from(params: &UiMixerChannelParams) -> Self {
        match params {
            // Volume is in dB (both UI and backend use dB)
            UiMixerChannelParams::Volume(value) => Self::Volume(*value),
            UiMixerChannelParams::Pan(value) => Self::Pan(*value),
            UiMixerChannelParams::Mute(value) => Self::Mute(*value),
            UiMixerChannelParams::InvertedPhase(value) => Self::InvertedPhase(*value),
            UiMixerChannelParams::Solo(value) => Self::Solo(*value),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ParameterValueTypeDTO {
    Float,
    Int,
    Bool,
    Choice,
}

// 2. Mirror the Struct
#[derive(Clone, Debug)]
pub struct ParameterSpecDTO {
    pub id: u32,
    pub name: String,
    pub group: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub default_value: f32,
    pub step: f32,
    pub value_type: ParameterValueTypeDTO,
    pub choices: Vec<String>,
}

impl From<ParameterValueType> for ParameterValueTypeDTO {
    fn from(val: ParameterValueType) -> Self {
        match val {
            ParameterValueType::Float => ParameterValueTypeDTO::Float,
            ParameterValueType::Int => ParameterValueTypeDTO::Int,
            ParameterValueType::Bool => ParameterValueTypeDTO::Bool,
            ParameterValueType::Choice => ParameterValueTypeDTO::Choice,
        }
    }
}

impl From<&ParameterSpec> for ParameterSpecDTO {
    fn from(spec: &ParameterSpec) -> Self {
        Self {
            id: spec.id,
            name: spec.name.clone(),
            group: spec.group.clone(),
            value: spec.value as f32,
            min: spec.min as f32,
            max: spec.max as f32,
            default_value: spec.default_value as f32,
            step: spec.step as f32,
            value_type: spec.value_type.into(), // Automatically uses the enum mapping above
            choices: spec.choices.clone(),
        }
    }
}

// ======================================
// STREAMS
// ======================================

/// Opens a stream that continuously polls the audio-thread feedback ring buffer
/// for `MixerChannelSnapshot` events and forwards them to Flutter.
///
/// Call `query_mixer_channel(target)` to request a snapshot; the audio thread
/// will push the response into the feedback ring buffer, and it will arrive
/// here within one polling interval (~16 ms).
///
/// This mirrors the pattern used by `create_plugin_message_stream` for plugin
/// parameters — there is no special direct-callback path; everything goes
/// through the shared `PENDING_FEEDBACK` buffer.
///
/// The polling thread terminates automatically when Flutter closes the stream.
pub fn create_mixer_snapshot_stream(
    sink: StreamSink<UiMixerChannelSnapshot>,
) -> Result<(), String> {
    std::thread::spawn(move || {
        loop {
            let snapshots = mixer_api::poll_mixer_channel_feedback(|snap| {
                let (track_id, bus_id, is_master) = match &snap.target {
                    MixerChannelTarget::Track(id) => (id.to_u32(), None, false),
                    MixerChannelTarget::Bus(id) => (u32::MAX, Some(id.to_u32()), false),
                    MixerChannelTarget::Master => (u32::MAX - 1, None, true),
                };
                UiMixerChannelSnapshot {
                    track_id,
                    bus_id,
                    is_master,
                    volume: snap.volume,
                    pan: snap.pan,
                    mute: snap.mute,
                    solo: snap.solo,
                    inverted_phase: snap.inverted_phase,
                }
            });

            for snapshot in snapshots {
                if sink.add(snapshot).is_err() {
                    // Flutter closed the stream — exit cleanly
                    return;
                }
            }

            // ~60 fps poll rate, same as plugin parameter streams
            std::thread::sleep(Duration::from_millis(16));
        }
    });

    Ok(())
}

// ======================================
// SETTERS — Command-Based (no AppState write)
// ======================================

/// Set a single DSP parameter on a mixer channel.
/// Routes through the audio thread ring buffer; AppState is only updated on save.
pub fn set_mixer_channel_param(target: UiMixerChannelTarget, param: UiMixerChannelParams) {
    let core_target = MixerChannelTarget::from(&target);
    let core_param = MixerChannelParams::from(&param);
    mixer_api::set_mixer_channel_param(core_target, core_param);
}

/// Request a full snapshot of a mixer channel's current DSP state.
/// Results arrive asynchronously via the `create_mixer_snapshot_stream` polling stream.
pub fn query_mixer_channel(target: UiMixerChannelTarget) {
    mixer_api::query_mixer_channel(MixerChannelTarget::from(&target));
}

// ======================================
// GETTERS
// ======================================

/// **GETTER: Fetch the mixer state**
pub fn get_mixer_state() -> UiMixerState {
    mixer_api::get_mixer_state(|mixer_state| UiMixerState::from(mixer_state))
}

/// **GETTER: Fetch a specific mixer channel**
pub fn get_mixer_channel(track_id: u32) -> Result<UiMixerChannel, String> {
    mixer_api::get_mixer_channel(TrackId::from(track_id), |mixer_channel| {
        UiMixerChannel::from(mixer_channel)
    })
    .map_err(|e| e.to_string())
}

pub fn get_mixer_channel_populated(
    track_id: u32,
) -> Result<(UiMixerChannel, Vec<UiEffectInstance>), String> {
    mixer_api::get_mixer_channel_populated(
        TrackId::from(track_id),
        |channel| UiMixerChannel::from(channel),
        |effect| UiEffectInstance::from(effect),
    )
    .map_err(|e| e.to_string())
}

/// **GETTER: Fetch the master bus**
pub fn get_master_bus() -> UiMixerChannel {
    mixer_api::get_master_bus().as_ref().into()
}

pub fn get_master_bus_populated() -> Vec<UiEffectInstance> {
    mixer_api::get_master_bus_populated(|e| UiEffectInstance::from(e))
}

/// **GETTER: Fetch all buses**
pub fn get_buses() -> HashMap<u32, UiBus> {
    mixer_api::get_buses(|id, bus| (id.to_u32(), UiBus::from(bus)))
}

/// **GETTER: Fetch the routing matrix**
pub fn get_routing_matrix() -> Vec<UiRoutingConnection> {
    mixer_api::get_routing_matrix(|conn| UiRoutingConnection::from(conn))
}

/// Get track channel's parameter specs
pub fn get_track_mixer_channel_specs(track_id: u32) -> Option<Vec<ParameterSpecDTO>> {
    mixer_api::get_track_mixer_channel_specs(&TrackId(track_id), |param_spec| {
        ParameterSpecDTO::from(param_spec)
    })
}

/// Get bus channel's parameter specs
pub fn get_bus_mixer_channel_specs(bus_id: u32) -> Option<Vec<ParameterSpecDTO>> {
    mixer_api::get_bus_mixer_channel_specs(&BusId(bus_id), |param_spec| {
        ParameterSpecDTO::from(param_spec)
    })
}

/// get master channel's parameter specs
pub fn get_master_channel_specs() -> Vec<ParameterSpecDTO> {
    mixer_api::get_master_channel_specs(|param_spec| ParameterSpecDTO::from(param_spec))
}

// ======================================
// Effect Chain (structural — AppState backed)
// ======================================

/// Add an effect to a mixer channel by its registry ID (preferred method).
pub fn add_effect_to_mixer_channel_by_id(track_id: u32, registry_id: u32) -> Result<(), String> {
    mixer_api::add_effect_to_mixer_channel_by_id(TrackId::from(track_id), registry_id)
        .map_err(|e| e.to_string())?;
    log::info!(
        "Added effect with registry ID {} to track {}",
        registry_id,
        track_id
    );
    Ok(())
}

pub fn remove_effect_from_mixer_channel(
    track_id: u32,
    effect_instance_id: u32,
) -> Result<(), String> {
    mixer_api::remove_effect_from_mixer_channel(
        TrackId::from(track_id),
        EffectId::from(effect_instance_id),
    )
    .map_err(|e| e.to_string())?;
    log::info!(
        "Removed effect instance ID {} from track {}",
        effect_instance_id,
        track_id
    );
    Ok(())
}

pub fn add_effect_to_master_bus(registry_id: u32) -> Result<(), String> {
    mixer_api::add_effect_to_master_bus(registry_id).map_err(|e| e.to_string())?;
    log::info!(
        "Added effect with registry ID {} to master bus",
        registry_id
    );
    Ok(())
}

pub fn remove_effect_from_master_bus(effect_instance_id: u32) -> Result<(), String> {
    mixer_api::remove_effect_from_master_bus(EffectId::from(effect_instance_id))
        .map_err(|e| e.to_string())?;
    log::info!(
        "Removed effect instance ID {} from master bus",
        effect_instance_id
    );
    Ok(())
}

// ======================================
// BUS MANAGEMENT APIs
// ======================================

/// Create a new mixer bus and return its ID.
pub fn create_bus(name: String) -> Result<u32, String> {
    // TODO: Refactor this to Core's API
    let bus_id = mixer_api::create_bus(name);
    Ok(bus_id.into())
}

/// Delete a mixer bus.
pub fn delete_bus(bus_id: u32) -> Result<(), String> {
    mixer_api::delete_bus(BusId::from(bus_id)).map_err(|e| e.to_string())
}

// ======================================
// BUS EFFECT MANAGEMENT APIs
// ======================================

/// Add an effect to a bus by its registry ID.
pub fn add_effect_to_bus(bus_id: u32, registry_id: u32) -> Result<(), String> {
    mixer_api::add_effect_to_bus(BusId::from(bus_id), registry_id).map_err(|e| e.to_string())?;
    log::info!(
        "Added effect with registry ID {} to bus {}",
        registry_id,
        bus_id
    );
    Ok(())
}

pub fn rename_bus(bus_id: u32, new_name: String) -> Result<(), String> {
    mixer_api::rename_bus(BusId::from(bus_id), &new_name).map_err(|e| e.to_string())
}

// ======================================
// ROUTING APIs
// ======================================

pub fn get_channel_destinations(is_bus: bool, channel_id: u32) -> Vec<UiRoutingConnection> {
    let source_node = if is_bus {
        RoutingNode::Bus(BusId::from(channel_id))
    } else {
        RoutingNode::Track(TrackId::from(channel_id))
    };

    mixer_api::get_destinations_of_mixer_channel(&source_node, |conn| UiRoutingConnection {
        destination: UiRoutingNode::from(&conn.destination),
        send_level: conn.send_level,
        source: UiRoutingNode::from(&source_node),
        is_send: conn.is_send,
    })
}

/// Set routing: source → destination with send level.
pub fn set_routing(
    source: UiRoutingNode,
    destination: UiRoutingNode,
    send_level: f32,
    is_send: bool,
) -> Result<(), String> {
    let conn = RoutingConnection {
        source: (&source).into(),
        destination: (&destination).into(),
        send_level,
        is_send,
    };

    mixer_api::set_routing(conn).map_err(|e| e.to_string())
}

/// Remove a routing connection.
pub fn remove_routing(
    source: UiRoutingNode,
    destination: UiRoutingNode,
    is_send: bool,
) -> Result<(), String> {
    mixer_api::remove_routing((&source).into(), (&destination).into(), is_send)
        .map_err(|e| e.to_string())
}
