use std::collections::HashMap;

use flutter_rust_bridge::frb;
use karbeat_core::core::project::TrackId;
use karbeat_core::shared::id::*;

use crate::frb_generated::StreamSink;
use karbeat_core::{
    context::{ ctx, MixerParamEvent },
    core::project::mixer::{
        EffectInstance,
        MixerBus,
        MixerChannel,
        MixerChannelParams,
        MixerState,
        RoutingConnection,
        RoutingNode,
    },
};
use karbeat_core::api::mixer_api as mixer_api;

// ======================================
// Type Definitions
// ======================================

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

pub struct UiMixerParamEvent {
    pub track_id: u32,
    pub volume: Option<f32>,
    pub pan: Option<f32>,
    pub mute: Option<bool>,
    pub solo: Option<bool>,
}

impl From<MixerParamEvent> for UiMixerParamEvent {
    fn from(value: MixerParamEvent) -> Self {
        Self {
            track_id: value.track_id,
            volume: value.volume,
            pan: value.pan,
            mute: value.mute,
            solo: value.solo,
        }
    }
}

impl From<&MixerChannel> for UiMixerChannel {
    fn from(value: &MixerChannel) -> Self {
        Self {
            // Volume is in dB (both UI and backend use dB)
            volume: value.volume,
            pan: value.pan,
            mute: value.mute,
            solo: value.solo,
            inverted_phase: value.inverted_phase,
            effects: value.effects
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

impl From<&MixerBus> for UiBus {
    fn from(value: &MixerBus) -> Self {
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
}

impl From<&RoutingNode> for UiRoutingNode {
    fn from(value: &RoutingNode) -> Self {
        match value {
            RoutingNode::Track(id) => UiRoutingNode::Track(id.to_u32()),
            RoutingNode::Bus(id) => UiRoutingNode::Bus(id.to_u32()),
            RoutingNode::Master => UiRoutingNode::Master,
        }
    }
}

impl From<&UiRoutingNode> for RoutingNode {
    fn from(value: &UiRoutingNode) -> Self {
        match value {
            UiRoutingNode::Track(id) => RoutingNode::Track((*id).into()),
            UiRoutingNode::Bus(id) => RoutingNode::Bus(BusId::from(*id)),
            UiRoutingNode::Master => RoutingNode::Master,
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
            channels: value.channels
                .iter()
                .map(|(id, channel)| (id.to_u32(), channel.as_ref().into()))
                .collect(),
            master_bus: value.master_bus.as_ref().into(),
            buses: value.buses
                .iter()
                .map(|(id, bus)| (id.to_u32(), bus.as_ref().into()))
                .collect(),
            routing: value.routing
                .iter()
                .map(|c| c.into())
                .collect(),
        }
    }
}

pub struct UiEffectInstance {
    pub id: u32,
    pub name: String,
    pub parameters: HashMap<u32, f32>,
}

impl From<&EffectInstance> for UiEffectInstance {
    fn from(value: &EffectInstance) -> Self {
        Self {
            id: value.id.to_u32(),
            name: value.instance.name.clone(),
            parameters: value.instance.parameters.clone().into_iter().collect(),
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
        routing: Vec<UiRoutingConnection>
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

// ======================================
// STREAM
// ======================================

/// Create the Rust → Flutter event stream for mixer param changes.
pub fn create_mixer_event_stream(sink: StreamSink<UiMixerParamEvent>) -> Result<(), String> {
    let mut guard = ctx().mixer_event_sink.lock();
    *guard = Some(
        Box::new(move |event| {
            let _ = sink.add(event.into());
        })
    );
    log::info!("Mixer event stream connected");
    Ok(())
}

/// Helper: push an event to the mixer sink (if connected).
fn push_mixer_event(event: MixerParamEvent) {
    if let Some(sink) = ctx().mixer_event_sink.lock().as_ref() {
        sink(event);
    }
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
    mixer_api
        ::get_mixer_channel(TrackId::from(track_id), |mixer_channel|
            UiMixerChannel::from(mixer_channel)
        )
        .map_err(|e| e.to_string())
}

pub fn get_mixer_channel_populated(
    track_id: u32
) -> Result<(UiMixerChannel, Vec<UiEffectInstance>), String> {
    mixer_api
        ::get_mixer_channel_populated(
            TrackId::from(track_id),
            |channel| UiMixerChannel::from(channel),
            |effect| UiEffectInstance::from(effect)
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
    mixer_api::get_buses(|id, bus| { (id.to_u32(), UiBus::from(bus)) })
}

/// **GETTER: Fetch the routing matrix**
pub fn get_routing_matrix() -> Vec<UiRoutingConnection> {
    mixer_api::get_routing_matrix(|conn| { UiRoutingConnection::from(conn) })
}

// ======================================
// MIXER ACTIONS AND APIs
// ======================================

pub fn set_master_bus_params(params: Vec<UiMixerChannelParams>) -> Result<(), String> {
    let params_legit: Vec<MixerChannelParams> = params
        .iter()
        .map(|p| p.into())
        .collect();
    mixer_api::set_master_bus_params(&params_legit).map_err(|e| e.to_string())?;

    // Push event to Flutter stream
    let mut event = MixerParamEvent {
        track_id: u32::MAX,
        volume: None,
        pan: None,
        mute: None,
        solo: None,
    };
    for p in &params {
        match p {
            UiMixerChannelParams::Volume(v) => {
                event.volume = Some(*v);
            }
            UiMixerChannelParams::Pan(v) => {
                event.pan = Some(*v);
            }
            UiMixerChannelParams::Mute(v) => {
                event.mute = Some(*v);
            }
            UiMixerChannelParams::Solo(v) => {
                event.solo = Some(*v);
            }
            _ => {}
        }
    }
    push_mixer_event(event);

    Ok(())
}

pub fn set_mixer_channel_params(
    track_id: u32,
    params: Vec<UiMixerChannelParams>
) -> Result<(), String> {
    let params_legit: Vec<MixerChannelParams> = params
        .iter()
        .map(|p| p.into())
        .collect();
    mixer_api
        ::set_mixer_channel_params(TrackId::from(track_id), &params_legit)
        .map_err(|e| e.to_string())?;

    // Push event to Flutter stream
    let mut event = MixerParamEvent {
        track_id,
        volume: None,
        pan: None,
        mute: None,
        solo: None,
    };
    for p in &params {
        match p {
            UiMixerChannelParams::Volume(v) => {
                event.volume = Some(*v);
            }
            UiMixerChannelParams::Pan(v) => {
                event.pan = Some(*v);
            }
            UiMixerChannelParams::Mute(v) => {
                event.mute = Some(*v);
            }
            UiMixerChannelParams::Solo(v) => {
                event.solo = Some(*v);
            }
            _ => {}
        }
    }
    push_mixer_event(event);

    Ok(())
}

/// Add an effect to a mixer channel by its registry ID (preferred method).
pub fn add_effect_to_mixer_channel_by_id(track_id: u32, registry_id: u32) -> Result<(), String> {
    mixer_api
        ::add_effect_to_mixer_channel_by_id(TrackId::from(track_id), registry_id)
        .map_err(|e| e.to_string())?;
    log::info!("Added effect with registry ID {} to track {}", registry_id, track_id);
    Ok(())
}

pub fn remove_effect_from_mixer_channel(
    track_id: u32,
    effect_instance_id: u32
) -> Result<(), String> {
    mixer_api
        ::remove_effect_from_mixer_channel(
            TrackId::from(track_id),
            EffectId::from(effect_instance_id)
        )
        .map_err(|e| e.to_string())?;
    log::info!("Removed effect instance ID {} from track {}", effect_instance_id, track_id);
    Ok(())
}

pub fn add_effect_to_master_bus(registry_id: u32) -> Result<(), String> {
    mixer_api::add_effect_to_master_bus(registry_id).map_err(|e| e.to_string())?;
    log::info!("Added effect with registry ID {} to master bus", registry_id);
    Ok(())
}

pub fn remove_effect_from_master_bus(effect_instance_id: u32) -> Result<(), String> {
    mixer_api
        ::remove_effect_from_master_bus(EffectId::from(effect_instance_id))
        .map_err(|e| e.to_string())?;
    log::info!("Removed effect instance ID {} from master bus", effect_instance_id);
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

/// Set bus channel parameters (volume, pan, mute).
pub fn set_bus_params(bus_id: u32, params: Vec<UiMixerChannelParams>) -> Result<(), String> {
    let params_legit: Vec<MixerChannelParams> = params
        .iter()
        .map(|p| p.into())
        .collect();
    mixer_api::set_bus_params(BusId::from(bus_id), &params_legit).map_err(|e| e.to_string())
}

// ======================================
// BUS EFFECT MANAGEMENT APIs
// ======================================

/// Add an effect to a bus by its registry ID.
pub fn add_effect_to_bus(bus_id: u32, registry_id: u32) -> Result<(), String> {
    mixer_api::add_effect_to_bus(BusId::from(bus_id), registry_id).map_err(|e| e.to_string())?;
    log::info!("Added effect with registry ID {} to bus {}", registry_id, bus_id);
    Ok(())
}

pub fn rename_bus(bus_id: u32, new_name: String) -> Result<(), String> {
    mixer_api::rename_bus(BusId::from(bus_id), &new_name).map_err(|e| e.to_string())
}

// TODO: Implement remove_effect_from_bus when the Audio Engine already
// handled the RemoveEffectFromBus command

// ======================================
// ROUTING APIs
// ======================================

/// Set routing: source → destination with send level.
pub fn set_routing(
    source: UiRoutingNode,
    destination: UiRoutingNode,
    send_level: f32,
    is_send: bool
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
    is_send: bool
) -> Result<(), String> {
    mixer_api
        ::remove_routing((&source).into(), (&destination).into(), is_send)
        .map_err(|e| e.to_string())
}
