use std::collections::HashMap;

use crate::{
    broadcast_state_change, commands::AudioCommand, core::project::mixer::{
        BusId, EffectInstance, MixerBus, MixerChannel, MixerChannelParams, MixerState,
        RoutingConnection, RoutingNode,
    }, ctx, utils::lock::{get_app_read, get_app_write}
};

/// ======================================
/// Type Definitions
/// ======================================

/// UI representation of a mixer channel.
pub struct UiMixerChannel {
    pub volume: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub inverted_phase: bool,
    /// List of effect IDs. UI does not need the heavy data object of effect instance
    pub effects: Vec<u32>,
}

impl From<&MixerChannel> for UiMixerChannel {
    fn from(value: &MixerChannel) -> Self {
        Self {
            volume: value.volume,
            pan: value.pan,
            mute: value.mute,
            solo: value.solo,
            inverted_phase: value.inverted_phase,
            effects: value
                .effects
                .iter()
                .map(|instance| instance.id.to_u32())
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
    /// 0=Track, 1=Bus, 2=Master
    pub source_type: u32,
    pub source_id: u32,
    /// 0=Track, 1=Bus, 2=Master
    pub dest_type: u32,
    pub dest_id: u32,
    pub send_level: f32,
    pub is_send: bool,
}

impl From<&RoutingConnection> for UiRoutingConnection {
    fn from(value: &RoutingConnection) -> Self {
        let (source_type, source_id) = match value.source {
            RoutingNode::Track(id) => (0, id.to_u32()),
            RoutingNode::Bus(id) => (1, id.to_u32()),
            RoutingNode::Master => (2, 0),
        };
        let (dest_type, dest_id) = match value.destination {
            RoutingNode::Track(id) => (0, id.to_u32()),
            RoutingNode::Bus(id) => (1, id.to_u32()),
            RoutingNode::Master => (2, 0),
        };
        Self {
            source_type,
            source_id,
            dest_type,
            dest_id,
            send_level: value.send_level,
            is_send: value.is_send,
        }
    }
}

/// UI representation of the mixer state.
pub struct UiMixerState {
    pub channels: HashMap<u32, UiMixerChannel>,
    pub master_bus: UiMixerChannel,
    pub buses: Vec<UiBus>,
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
            buses: value.buses.values().map(|b| b.as_ref().into()).collect(),
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

pub enum UiMixerChannelParams {
    Volume(f32),
    Pan(f32),
    Mute(bool),
    InvertedPhase(bool),
}

impl Into<UiMixerChannelParams> for &MixerChannelParams {
    fn into(self) -> UiMixerChannelParams {
        match self {
            MixerChannelParams::Volume(value) => UiMixerChannelParams::Volume(*value),
            MixerChannelParams::Pan(value) => UiMixerChannelParams::Pan(*value),
            MixerChannelParams::Mute(value) => UiMixerChannelParams::Mute(*value),
            MixerChannelParams::InvertedPhase(value) => UiMixerChannelParams::InvertedPhase(*value),
        }
    }
}

impl Into<MixerChannelParams> for &UiMixerChannelParams {
    fn into(self) -> MixerChannelParams {
        match self {
            UiMixerChannelParams::Volume(value) => MixerChannelParams::Volume(*value),
            UiMixerChannelParams::Pan(value) => MixerChannelParams::Pan(*value),
            UiMixerChannelParams::Mute(value) => MixerChannelParams::Mute(*value),
            UiMixerChannelParams::InvertedPhase(value) => MixerChannelParams::InvertedPhase(*value),
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

/// **GETTER: Fetch the master bus**
pub fn get_master_bus() -> UiMixerChannel {
    let app = get_app_read();
    let mixer_state = &app.mixer;
    mixer_state.master_bus.as_ref().into()
}

/// **GETTER: Fetch all buses**
pub fn get_buses() -> Vec<UiBus> {
    let app = get_app_read();
    app.mixer
        .buses
        .values()
        .map(|b| b.as_ref().into())
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
    let mut app = get_app_write();
    let mixer_state = &mut app.mixer;
    let params_legit: Vec<MixerChannelParams> = params.iter().map(|p| p.into()).collect();
    mixer_state
        .set_params_master_bus(&params_legit)
        .map_err(|e| e.message)?;
    Ok(())
}

pub fn set_mixer_channel_params(
    track_id: u32,
    params: Vec<UiMixerChannelParams>,
) -> Result<(), String> {
    let mut app = get_app_write();
    let mixer_state = &mut app.mixer;
    let params_legit: Vec<MixerChannelParams> = params.iter().map(|p| p.into()).collect();
    mixer_state
        .set_params_mixer_channel(&track_id.into(), &params_legit)
        .map_err(|e| e.message)?;
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
    }
    broadcast_state_change();
    Ok(())
}

/// Add an effect to a mixer channel by name (backwards compatible).
pub fn add_effect_to_mixer_channel(track_id: u32, effect_name: String) -> Result<(), String> {
    {
        let mut app = get_app_write();
        let mixer_state = &mut app.mixer;
        #[allow(deprecated)]
        mixer_state
            .add_effect_descriptor(&track_id.into(), &effect_name, "")
            .map_err(|e| format!("{}", e))?;
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
// ROUTING APIs
// ======================================

/// Set routing: source → destination with send level.
/// source_type: 0=Track, 1=Bus
/// dest_type: 1=Bus, 2=Master
pub fn set_routing(
    source_type: u32,
    source_id: u32,
    dest_type: u32,
    dest_id: u32,
    send_level: f32,
    is_send: bool,
) -> Result<(), String> {
    {
        let mut app = get_app_write();

        let source = match source_type {
            0 => RoutingNode::Track(source_id.into()),
            1 => RoutingNode::Bus(BusId::from(source_id)),
            _ => return Err("Invalid source type".to_string()),
        };

        let destination = match dest_type {
            1 => RoutingNode::Bus(BusId::from(dest_id)),
            2 => RoutingNode::Master,
            _ => return Err("Invalid destination type".to_string()),
        };

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
    source_type: u32,
    source_id: u32,
    dest_type: u32,
    dest_id: u32,
    is_send: bool,
) -> Result<(), String> {
    {
        let mut app = get_app_write();

        let source = match source_type {
            0 => RoutingNode::Track(source_id.into()),
            1 => RoutingNode::Bus(BusId::from(source_id)),
            _ => return Err("Invalid source type".to_string()),
        };

        let destination = match dest_type {
            1 => RoutingNode::Bus(BusId::from(dest_id)),
            2 => RoutingNode::Master,
            _ => return Err("Invalid destination type".to_string()),
        };

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
