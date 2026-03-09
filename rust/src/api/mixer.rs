use std::collections::HashMap;

use flutter_rust_bridge::frb;

use crate::{
    broadcast_state_change,
    commands::AudioCommand,
    core::project::mixer::{
        BusId, EffectInstance, MixerBus, MixerChannel, MixerChannelParams, MixerState,
        RoutingConnection, RoutingNode,
    },
    ctx,
    frb_generated::StreamSink,
    utils::lock::{get_app_read, get_app_write},
};

// ======================================
// Type Definitions
// ======================================

/// Lightweight event pushed to Flutter when a mixer param changes
/// from the backend (automation, undo, or any non-UI source).
pub struct MixerParamEvent {
    /// Track ID. `u32::MAX` means master bus.
    pub track_id: u32,
    pub volume: Option<f32>,
    pub pan: Option<f32>,
    pub mute: Option<bool>,
    pub solo: Option<bool>,
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
    pub name: String,
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
            effects: value
                .effects
                .iter()
                .map(|instance| UiEffectSummary {
                    id: instance.id.to_u32(),
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

impl Into<RoutingNode> for &UiRoutingNode {
    fn into(self) -> RoutingNode {
        match self {
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
            channels: value
                .channels
                .iter()
                .map(|(id, channel)| (id.to_u32(), channel.as_ref().into()))
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
    pub parameters: HashMap<u32, f32>,
}

impl From<&EffectInstance> for UiEffectInstance {
    fn from(value: &EffectInstance) -> Self {
        Self {
            id: value.id.to_u32(),
            name: value.instance.name.clone(),
            parameters: value.instance.parameters.clone(),
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

impl Into<UiMixerChannelParams> for &MixerChannelParams {
    fn into(self) -> UiMixerChannelParams {
        match self {
            MixerChannelParams::Volume(value) => UiMixerChannelParams::Volume(*value),
            MixerChannelParams::Pan(value) => UiMixerChannelParams::Pan(*value),
            MixerChannelParams::Mute(value) => UiMixerChannelParams::Mute(*value),
            MixerChannelParams::InvertedPhase(value) => UiMixerChannelParams::InvertedPhase(*value),
            MixerChannelParams::Solo(value) => UiMixerChannelParams::Solo(*value),
        }
    }
}

impl Into<MixerChannelParams> for &UiMixerChannelParams {
    fn into(self) -> MixerChannelParams {
        match self {
            // Volume is in dB (both UI and backend use dB)
            UiMixerChannelParams::Volume(value) => MixerChannelParams::Volume(*value),
            UiMixerChannelParams::Pan(value) => MixerChannelParams::Pan(*value),
            UiMixerChannelParams::Mute(value) => MixerChannelParams::Mute(*value),
            UiMixerChannelParams::InvertedPhase(value) => MixerChannelParams::InvertedPhase(*value),
            UiMixerChannelParams::Solo(value) => MixerChannelParams::Solo(*value),
        }
    }
}

/// ======================================
/// STREAM
/// ======================================

/// Create the Rust → Flutter event stream for mixer param changes.
pub fn create_mixer_event_stream(sink: StreamSink<MixerParamEvent>) -> Result<(), String> {
    let mut guard = ctx()
        .mixer_event_sink
        .lock()
        .map_err(|e| format!("lock error: {}", e))?;
    *guard = Some(sink);
    log::info!("Mixer event stream connected");
    Ok(())
}

/// Helper: push an event to the mixer sink (if connected).
fn push_mixer_event(event: MixerParamEvent) {
    if let Ok(guard) = ctx().mixer_event_sink.lock() {
        if let Some(sink) = guard.as_ref() {
            let _ = sink.add(event);
        }
    }
}

/// ======================================
/// GETTERS
/// ======================================

/// **GETTER: Fetch the mixer state**
pub fn get_mixer_state() -> UiMixerState {
    let app = get_app_read();
    let mixer_state = &app.mixer;
    mixer_state.into()
}

/// **GETTER: Fetch a specific mixer channel**
pub fn get_mixer_channel(track_id: u32) -> Result<UiMixerChannel, String> {
    let app = get_app_read();
    let mixer_state = &app.mixer;
    let channel = mixer_state.channels.get(&track_id.into());
    channel
        .ok_or("Channel not found".to_owned())
        .map(|c| c.as_ref().into())
}

pub fn get_mixer_channel_populated(
    track_id: u32,
) -> Result<(UiMixerChannel, Vec<UiEffectInstance>), String> {
    let app = get_app_read();
    let mixer_state = &app.mixer;
    let channel = mixer_state.channels.get(&track_id.into());
    let channel = channel.ok_or("Channel not found".to_owned())?;
    let ui_channel: UiMixerChannel = channel.as_ref().into();
    let effects = channel.effects.iter().map(|e| e.into()).collect();
    Ok((ui_channel, effects))
}

/// **GETTER: Fetch the master bus**
pub fn get_master_bus() -> UiMixerChannel {
    let app = get_app_read();
    let mixer_state = &app.mixer;
    mixer_state.master_bus.as_ref().into()
}

pub fn get_master_bus_populated() -> Vec<UiEffectInstance> {
    let app = get_app_read();
    let mixer_state = &app.mixer;
    mixer_state
        .master_bus
        .effects
        .iter()
        .map(|e| e.into())
        .collect()
}

/// **GETTER: Fetch all buses**
pub fn get_buses() -> HashMap<u32, UiBus> {
    let app = get_app_read();
    app.mixer
        .buses
        .iter()
        .map(|(i, b)| (i.to_u32(), b.as_ref().into()))
        .collect()
}

/// **GETTER: Fetch the routing matrix**
pub fn get_routing_matrix() -> Vec<UiRoutingConnection> {
    let app = get_app_read();
    app.mixer.routing.iter().map(|c| c.into()).collect()
}

// ======================================
// MIXER ACTIONS AND APIs
// ======================================

pub fn set_master_bus_params(params: Vec<UiMixerChannelParams>) -> Result<(), String> {
    {
        let mut app = get_app_write();
        let mixer_state = &mut app.mixer;
        let params_legit: Vec<MixerChannelParams> = params.iter().map(|p| p.into()).collect();
        mixer_state
            .set_params_master_bus(&params_legit)
            .map_err(|e| e.message)?;
    } // drop write lock before broadcast

    broadcast_state_change();

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
            UiMixerChannelParams::Volume(v) => event.volume = Some(*v),
            UiMixerChannelParams::Pan(v) => event.pan = Some(*v),
            UiMixerChannelParams::Mute(v) => event.mute = Some(*v),
            UiMixerChannelParams::Solo(v) => event.solo = Some(*v),
            _ => {}
        }
    }
    push_mixer_event(event);

    Ok(())
}

pub fn set_mixer_channel_params(
    track_id: u32,
    params: Vec<UiMixerChannelParams>,
) -> Result<(), String> {
    {
        let mut app = get_app_write();
        let mixer_state = &mut app.mixer;
        let params_legit: Vec<MixerChannelParams> = params.iter().map(|p| p.into()).collect();
        mixer_state
            .set_params_mixer_channel(&track_id.into(), &params_legit)
            .map_err(|e| e.message)?;
    } // drop write lock before broadcast

    broadcast_state_change();

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
            UiMixerChannelParams::Volume(v) => event.volume = Some(*v),
            UiMixerChannelParams::Pan(v) => event.pan = Some(*v),
            UiMixerChannelParams::Mute(v) => event.mute = Some(*v),
            UiMixerChannelParams::Solo(v) => event.solo = Some(*v),
            _ => {}
        }
    }
    push_mixer_event(event);

    Ok(())
}

/// Add an effect to a mixer channel by its registry ID (preferred method).
pub fn add_effect_to_mixer_channel_by_id(track_id: u32, registry_id: u32) -> Result<(), String> {
    {
        let mut app = get_app_write();
        let mixer_state = &mut app.mixer;
        mixer_state
            .add_effect_descriptor_by_id(&track_id.into(), registry_id)
            .map_err(|e| format!("{}", e))?;
        log::info!(
            "Added effect with registry ID {} to track {}",
            registry_id,
            track_id
        );
    }
    broadcast_state_change();
    Ok(())
}

pub fn remove_effect_from_mixer_channel(track_id: u32, effect_instance_id: u32) -> Result<(), String> {
    {
        let mut app = get_app_write();
        let mixer_state = &mut app.mixer;
        mixer_state
            .remove_effect_by_id(&track_id.into(), effect_instance_id.into())
            .map_err(|e| format!("{}", e))?;
        log::info!(
            "Removed effect instance ID {} from track {}",
            effect_instance_id,
            track_id
        );
    }
    broadcast_state_change();
    Ok(())
}

pub fn add_effect_to_master_bus(registry_id: u32) -> Result<(), String> {
    {
        let mut app = get_app_write();
        app.mixer
            .add_effect_to_master_bus(registry_id)
            .map_err(|e| format!("{}", e))?;
        log::info!(
            "Added effect with registry ID {} to master bus",
            registry_id,
        );
    }
    broadcast_state_change();
    Ok(())
}

pub fn remove_effect_from_master_bus(effect_instance_id: u32) -> Result<(), String> {
    {
        let mut app = get_app_write();
        app.mixer
            .remove_effect_from_master_bus(effect_instance_id.into())
            .map_err(|e| format!("{}", e))?;
        log::info!(
            "Removed effect instance ID {} from master bus",
            effect_instance_id,
        );
    }
    broadcast_state_change();
    Ok(())
}

// ======================================
// BUS MANAGEMENT APIs
// ======================================

/// Create a new mixer bus and return its ID.
pub fn create_bus(name: String) -> Result<u32, String> {
    let bus_id = {
        let mut app = get_app_write();
        let bus_id = app.mixer.create_bus(name.clone());

        // Send command to audio thread
        if let Some(sender) = ctx().command_sender.lock().unwrap().as_mut() {
            let _ = sender.push(AudioCommand::AddBus {
                bus_id,
                name: name.clone(),
            });
        }

        bus_id
    };
    broadcast_state_change();
    Ok(bus_id.to_u32())
}

/// Delete a mixer bus.
pub fn delete_bus(bus_id: u32) -> Result<(), String> {
    {
        let mut app = get_app_write();
        app.mixer.remove_bus(bus_id.into())?;

        // Send command to audio thread
        if let Some(sender) = ctx().command_sender.lock().unwrap().as_mut() {
            let _ = sender.push(AudioCommand::RemoveBus {
                bus_id: bus_id.into(),
            });
        }
    }
    broadcast_state_change();
    Ok(())
}

/// Set bus channel parameters (volume, pan, mute).
pub fn set_bus_params(bus_id: u32, params: Vec<UiMixerChannelParams>) -> Result<(), String> {
    {
        let mut app = get_app_write();
        let params_legit: Vec<MixerChannelParams> = params.iter().map(|p| p.into()).collect();
        app.mixer.set_params_bus(&bus_id.into(), &params_legit)?;
    }
    broadcast_state_change();
    Ok(())
}

// ======================================
// BUS EFFECT MANAGEMENT APIs
// ======================================

/// Add an effect to a bus by its registry ID.
pub fn add_effect_to_bus(bus_id: u32, registry_id: u32) -> Result<(), String> {
    {
        let mut app = get_app_write();
        app.mixer
            .add_effect_to_bus(bus_id.into(), registry_id)
            .map_err(|e| format!("{}", e))?;
        log::info!(
            "Added effect with registry ID {} to bus {}",
            registry_id,
            bus_id
        );
    }
    broadcast_state_change();
    Ok(())
}

pub fn rename_bus(bus_id: u32, new_name: String) -> Result<(), String> {
    {
        let mut app = get_app_write();
        app.mixer.rename_bus(bus_id.into(), &new_name)?;
    }
    broadcast_state_change();
    Ok(())
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
    is_send: bool,
) -> Result<(), String> {
    {
        let mut app = get_app_write();
        let source: RoutingNode = (&source).into();
        let destination: RoutingNode = (&destination).into();

        let conn = RoutingConnection {
            source,
            destination,
            send_level,
            is_send,
        };

        app.mixer.add_routing(conn)?;

        // Sync routing to audio thread
        if let Some(sender) = ctx().command_sender.lock().unwrap().as_mut() {
            let _ = sender.push(AudioCommand::UpdateRouting {
                routing: app.mixer.routing.clone(),
            });
        }
    }
    broadcast_state_change();
    Ok(())
}

/// Remove a routing connection.
pub fn remove_routing(
    source: UiRoutingNode,
    destination: UiRoutingNode,
    is_send: bool,
) -> Result<(), String> {
    {
        let mut app = get_app_write();
        let source: RoutingNode = (&source).into();
        let destination: RoutingNode = (&destination).into();

        app.mixer.remove_routing(source, destination, is_send)?;

        // Sync routing to audio thread
        if let Some(sender) = ctx().command_sender.lock().unwrap().as_mut() {
            let _ = sender.push(AudioCommand::UpdateRouting {
                routing: app.mixer.routing.clone(),
            });
        }
    }
    broadcast_state_change();
    Ok(())
}
