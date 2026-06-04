use hashbrown::{HashMap, HashSet};
use karbeat_plugin_types::{Param, ParameterSpec};
use smallvec::SmallVec;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    commands::AudioCommand,
    context::{ctx, utils::send_audio_command},
    core::project::{plugin::AudioPlugin, ApplicationState, PluginInstance, TrackId},
    shared::{BusId, EffectId, SidechainRouteId},
};

// =============================================================================
// Routing Matrix Types
// =============================================================================

/// A node in the routing graph
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RoutingNode {
    Track(TrackId),
    Bus(BusId),
    Master,
    PluginSidechain(SidechainRouteId),
}

/// A routing connection in the matrix
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RoutingConnection {
    pub source: RoutingNode,
    pub destination: RoutingNode,
    /// Send level (0.0 = no signal, 1.0 = full signal)
    pub send_level: f32,
    /// If true, this is a "send" (post-fader tap) not the main output
    pub is_send: bool,
}

impl RoutingConnection {
    pub fn new(source: RoutingNode, destination: RoutingNode) -> Self {
        Self {
            source,
            destination,
            send_level: 1.0,
            is_send: false,
        }
    }

    pub fn new_send(source: RoutingNode, destination: RoutingNode, send_level: f32) -> Self {
        Self {
            source,
            destination,
            send_level,
            is_send: true,
        }
    }
}

/// A mixer bus with its own channel strip
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct BusMixerChannel {
    pub id: BusId,
    pub name: String,
    pub channel: MixerChannel,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct TrackMixerChannel {
    pub id: TrackId,
    pub name: String,
    pub channel: MixerChannel,
}

impl Default for BusMixerChannel {
    fn default() -> Self {
        Self {
            id: BusId::from(0),
            name: String::new(),
            channel: MixerChannel::default(),
        }
    }
}

impl Default for TrackMixerChannel {
    fn default() -> Self {
        Self {
            id: TrackId::from(0),
            name: String::new(),
            channel: MixerChannel::default(),
        }
    }
}

impl BusMixerChannel {
    pub fn new(id: BusId, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            channel: MixerChannel::default(),
        }
    }
}

/// Custom Error type for better error clarity
///
/// This represents an error that occur due to param setting operation
#[derive(Error, Debug, Clone)]
#[error("Mixer param error for track {track_id}: {message}")]
pub struct MixerSetParamError {
    pub message: String,
    pub track_id: TrackId,
}

#[derive(Error, Debug, Clone)]
#[error("Effect creation error: {message}")]
pub struct EffectCreationError {
    pub message: String,
}

impl MixerSetParamError {
    pub fn new(track_id: TrackId, message: &str) -> Self {
        Self {
            track_id,
            message: message.to_string(),
        }
    }
}

#[derive(Error, Debug)]
#[error("Mixer not found for track {track_id}: {message}")]
pub struct MixerNotFoundError {
    pub message: String,
    pub track_id: TrackId,
}

impl MixerNotFoundError {
    pub fn new(track_id: TrackId, message: &str) -> Self {
        Self {
            track_id,
            message: message.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MixerChannelParams {
    Volume(f32),
    Pan(f32),
    Mute(bool),
    InvertedPhase(bool),
    Solo(bool),
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(default)]
pub struct EffectInstance {
    pub id: EffectId,
    pub instance: Arc<PluginInstance>,
}

impl EffectInstance {
    pub fn new(id: EffectId, instance: PluginInstance) -> Self {
        Self {
            id,
            instance: Arc::new(instance),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(default)]
pub struct MixerState {
    /// Per-track mixer channels (volume, pan, effects)
    pub channels: HashMap<TrackId, Arc<TrackMixerChannel>>,
    /// Master bus channel
    pub master_bus: Arc<MixerChannel>,
    /// Named buses for grouping/submixing
    pub buses: HashMap<BusId, Arc<BusMixerChannel>>,
    /// All routing connections in the matrix
    pub routing: Vec<RoutingConnection>,
    /// Counter for generating bus IDs
    pub bus_counter: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct MixerChannel {
    pub volume: Param<f32>, //dB
    pub pan: Param<f32>,    // -1.0 to 1.0
    pub mute: bool,
    pub solo: bool,
    pub inverted_phase: bool,

    pub effect_counter: u32,

    // The effects chain (EQ, Compressor) comes AFTER the generator
    pub effects: SmallVec<[EffectInstance; 9]>,
}

impl Default for MixerChannel {
    fn default() -> Self {
        Self {
            volume: Param::new_f32(1, "Volume", "MixerChannel", 0.0, -60.0, 6.0, 0.1), // 0 dB = unity gain
            pan: Param::new_f32(2, "Pan", "MixerChannel", 0.0, -1.0, 1.0, 0.01),
            mute: false,
            solo: false,
            effect_counter: 0,
            inverted_phase: false,
            effects: SmallVec::new(),
        }
    }
}

impl MixerChannel {
    pub fn add_effect(
        &mut self,
        effect_registry_id: u32,
    ) -> anyhow::Result<(Box<dyn AudioPlugin + Send + Sync>, String, EffectId)> {
        let effect_id = EffectId::next(&mut self.effect_counter);

        let (effect_plugin, effect_name, _default_params) = {
            let registry = ctx().plugin_registry.read();
            if let Some((effect_box, name)) = registry.create_effect_by_id(effect_registry_id) {
                let default_params = effect_box.default_parameters();
                (effect_box, name, default_params)
            } else {
                let message = format!(
                    "Effect with ID {} not found in registry",
                    effect_registry_id
                );
                log::error!("{}", message);
                // Decrement counters if failed to prevent gaps/orphans
                self.effect_counter -= 1;

                return Err(anyhow::anyhow!(message));
            }
        };

        let plugin_instance = PluginInstance::new_with_id(effect_registry_id, &effect_name);

        let effect_instance = EffectInstance::new(effect_id, plugin_instance);
        self.effects.push(effect_instance);

        Ok((effect_plugin, effect_name, effect_id))
    }

    pub fn remove_effect(&mut self, effect_id: EffectId) -> anyhow::Result<()> {
        self.effects.retain(|effect| effect.id != effect_id);

        Ok(())
    }

    /// Get parameter specs of channel's parameter
    pub fn get_channel_specs(&self) -> Vec<ParameterSpec> {
        vec![
            self.volume.to_spec(),
            self.pan.to_spec(),
            // Note: maybe I will change the bool parameter to Param<bool> too
        ]
    }
}

impl MixerState {
    /// Add an effect descriptor to a mixer channel by its registry ID.
    pub fn add_effect_descriptor_by_id(
        &mut self,
        track_id: &TrackId,
        registry_id: u32,
    ) -> anyhow::Result<()> {
        let mixer_channel_arc = self
            .channels
            .get_mut(track_id)
            .ok_or_else(|| MixerNotFoundError::new(*track_id, "Cannot find the mixer channel"))
            .map_err(|e| anyhow::anyhow!(e))?;

        // Clone and modify the channel
        let channel = Arc::make_mut(mixer_channel_arc);

        let (effect_plugin, effect_name, effect_id) = channel.channel.add_effect(registry_id)?;

        // Push to the audio thread
        send_audio_command(AudioCommand::AddEffect {
            target: crate::commands::EffectTarget::Track(*track_id),
            effect_id,
            effect: effect_plugin,
        });

        log::info!(
            "Effect {} (registry_id={}) added to track {:?}",
            effect_name,
            registry_id,
            track_id
        );

        Ok(())
    }

    pub fn remove_effect_by_id(
        &mut self,
        track_id: &TrackId,
        effect_id: EffectId,
    ) -> anyhow::Result<()> {
        let mixer_channel_arc = self
            .channels
            .get_mut(track_id)
            .ok_or_else(|| MixerNotFoundError::new(*track_id, "Cannot find the mixer channel"))
            .map_err(|e| anyhow::anyhow!(e))?;

        // Clone and modify the channel
        let channel = Arc::make_mut(mixer_channel_arc);
        channel.channel.remove_effect(effect_id)?;

        send_audio_command(AudioCommand::RemoveEffect {
            target: crate::commands::EffectTarget::Track(*track_id),
            effect_id,
        });

        Ok(())
    }

    /// Get all effect instances from a mixer channel
    pub fn get_effects(
        &self,
        track_id: &TrackId,
    ) -> Result<Vec<EffectInstance>, MixerNotFoundError> {
        let mut mixer_channel_arc = self
            .channels
            .get(track_id)
            .ok_or_else(|| MixerNotFoundError::new(*track_id, "Cannot find the mixer channel"))?
            .to_owned();

        // Clone and modify the channel
        let channel = Arc::make_mut(&mut mixer_channel_arc);
        Ok(channel.channel.effects.to_vec())
    }

    pub fn add_effect_to_master_bus(&mut self, registry_id: u32) -> anyhow::Result<()> {
        let channel = Arc::make_mut(&mut self.master_bus);
        let (effect_plugin, effect_name, effect_id) = channel.add_effect(registry_id)?;

        send_audio_command(AudioCommand::AddEffect {
            target: crate::commands::EffectTarget::Master,
            effect_id,
            effect: effect_plugin,
        });

        log::info!(
            "Effect {} (registry_id={}) added to master bus",
            effect_name,
            registry_id
        );
        Ok(())
    }

    pub fn remove_effect_from_master_bus(&mut self, effect_id: EffectId) -> anyhow::Result<()> {
        let channel = Arc::make_mut(&mut self.master_bus);
        channel.remove_effect(effect_id)?;

        // Send master effect removal command to audio thread
        send_audio_command(AudioCommand::RemoveEffect {
            target: crate::commands::EffectTarget::Master,
            effect_id,
        });

        Ok(())
    }

    // =========================================================================
    // Bus Management
    // =========================================================================

    /// Create a new mixer bus and return its ID
    pub fn create_bus(&mut self, name: String) -> BusId {
        let bus_id = BusId::next(&mut self.bus_counter);
        let bus = BusMixerChannel::new(bus_id, &name);
        self.buses.insert(bus_id, Arc::new(bus));

        // By default, new buses route to master
        self.routing.push(RoutingConnection::new(
            RoutingNode::Bus(bus_id),
            RoutingNode::Master,
        ));

        // send signal to audio thread that the BUSSSS is created
        send_audio_command(AudioCommand::AddBus { bus_id, name });

        bus_id
    }

    /// Remove a bus and all routing connections to/from it
    pub fn remove_bus(&mut self, bus_id: BusId) -> anyhow::Result<()> {
        if !self.buses.contains_key(&bus_id) {
            return Err(anyhow::anyhow!("Bus {:?} not found", bus_id));
        }

        // Remove the bus
        self.buses.remove(&bus_id);

        // Remove all routing connections involving this bus
        self.routing.retain(|conn| {
            conn.source != RoutingNode::Bus(bus_id) && conn.destination != RoutingNode::Bus(bus_id)
        });

        // send signal to audio thread that the BUSSSS is deleted
        send_audio_command(AudioCommand::RemoveBus { bus_id });

        Ok(())
    }

    /// Get a mutable reference to a bus
    pub fn get_bus_mut(&mut self, bus_id: &BusId) -> Option<&mut Arc<BusMixerChannel>> {
        self.buses.get_mut(bus_id)
    }

    pub fn rename_bus(&mut self, bus_id: BusId, new_name: &str) -> anyhow::Result<()> {
        let bus_arc = self
            .buses
            .get_mut(&bus_id)
            .ok_or_else(|| anyhow::anyhow!("Bus {:?} not found", bus_id))?;

        let bus = Arc::make_mut(bus_arc);
        let old_name = bus.name.clone();
        bus.name = new_name.to_string();

        log::info!("Bus {:?} renamed from {} to {}", bus_id, old_name, new_name);
        Ok(())
    }

    pub fn add_effect_to_bus(
        &mut self,
        bus_id: BusId,
        registry_id: u32,
    ) -> anyhow::Result<(String, EffectId)> {
        let bus_arc = self
            .buses
            .get_mut(&bus_id)
            .ok_or_else(|| anyhow::anyhow!("Bus {:?} not found", bus_id))?;

        let bus = Arc::make_mut(bus_arc);
        let (effect_plugin, effect_name, effect_id) = bus.channel.add_effect(registry_id)?;

        send_audio_command(AudioCommand::AddEffect {
            target: crate::commands::EffectTarget::Bus(bus_id),
            effect_id,
            effect: effect_plugin,
        });

        log::info!(
            "Effect {} (registry_id={}) added to bus {:?}",
            effect_name,
            registry_id,
            bus_id
        );

        Ok((effect_name, effect_id))
    }

    pub fn remove_effect_from_bus(
        &mut self,
        bus_id: BusId,
        effect_id: EffectId,
    ) -> anyhow::Result<()> {
        let bus_arc = self
            .buses
            .get_mut(&bus_id)
            .ok_or_else(|| anyhow::anyhow!("Bus {:?} not found", bus_id))?;

        let bus = Arc::make_mut(bus_arc);
        bus.channel.remove_effect(effect_id)?;

        // TODO: implement sending command to audio thread to delete the effect
        Ok(())
    }

    // =========================================================================
    // Routing Management
    // =========================================================================

    /// Add a routing connection. Returns error if it would create a cycle.
    pub fn add_routing(&mut self, connection: RoutingConnection) -> anyhow::Result<()> {
        // Validate: source cannot be Master
        if connection.source == RoutingNode::Master {
            return Err(anyhow::anyhow!("Master cannot be a routing source"));
        }

        // Validate: destination cannot be a Track
        if matches!(connection.destination, RoutingNode::Track(_)) {
            return Err(anyhow::anyhow!("Tracks cannot be routing destinations"));
        }

        // Check for duplicate
        let exists = self.routing.iter().any(|c| {
            c.source == connection.source
                && c.destination == connection.destination
                && c.is_send == connection.is_send
        });
        if exists {
            return Err(anyhow::anyhow!("Routing connection already exists"));
        }

        // Temporarily add and check for cycles
        self.routing.push(connection.clone());
        if self.has_routing_cycle() {
            self.routing.pop();
            return Err(anyhow::anyhow!("Routing would create a cycle"));
        }

        // Sync routing to audio thread
        send_audio_command(AudioCommand::UpdateRouting {
            routing: self.routing.clone(),
        });

        Ok(())
    }

    /// Remove a routing connection
    pub fn remove_routing(
        &mut self,
        source: RoutingNode,
        destination: RoutingNode,
        is_send: bool,
    ) -> anyhow::Result<()> {
        let original_len = self.routing.len();
        self.routing.retain(|c| {
            !(c.source == source && c.destination == destination && c.is_send == is_send)
        });

        if self.routing.len() == original_len {
            return Err(anyhow::anyhow!("Routing connection not found"));
        }

        // Sync routing to audio thread
        send_audio_command(AudioCommand::UpdateRouting {
            routing: self.routing.clone(),
        });

        Ok(())
    }

    /// Check if the routing graph has a cycle using DFS
    pub fn has_routing_cycle(&self) -> bool {
        // Build adjacency list for buses only (tracks and master can't create cycles)
        let mut adj: HashMap<BusId, Vec<BusId>> = HashMap::new();
        for bus_id in self.buses.keys() {
            adj.insert(*bus_id, Vec::new());
        }

        for conn in &self.routing {
            if let (RoutingNode::Bus(src), RoutingNode::Bus(dst)) = (conn.source, conn.destination)
            {
                if let Some(neighbors) = adj.get_mut(&src) {
                    neighbors.push(dst);
                }
            }
        }

        // DFS to detect cycles
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for bus_id in self.buses.keys() {
            if !visited.contains(bus_id) && find_cycle(*bus_id, &adj, &mut visited, &mut rec_stack)
            {
                return true;
            }
        }

        false
    }

    /// Get topologically sorted routing order for audio thread processing
    /// Returns nodes in order: sources first, then intermediate buses, then master
    pub fn get_routing_order(&self) -> Vec<RoutingNode> {
        // All tracks come first (they are sources)
        let mut order: Vec<RoutingNode> = self
            .channels
            .keys()
            .map(|id| RoutingNode::Track(*id))
            .collect();

        // Topological sort of buses using Kahn's algorithm
        let mut in_degree: HashMap<BusId, usize> = HashMap::new();
        let mut adj: HashMap<BusId, Vec<BusId>> = HashMap::new();

        for bus_id in self.buses.keys() {
            in_degree.insert(*bus_id, 0);
            adj.insert(*bus_id, Vec::new());
        }

        // Count incoming edges from other buses
        for conn in &self.routing {
            if let (RoutingNode::Bus(src), RoutingNode::Bus(dst)) = (conn.source, conn.destination)
            {
                if let Some(neighbors) = adj.get_mut(&src) {
                    neighbors.push(dst);
                }
                if let Some(deg) = in_degree.get_mut(&dst) {
                    *deg += 1;
                }
            }
        }

        // Start with buses that have no incoming bus edges
        let mut queue: Vec<BusId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        while let Some(bus_id) = queue.pop() {
            order.push(RoutingNode::Bus(bus_id));

            if let Some(neighbors) = adj.get(&bus_id) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(&neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor);
                        }
                    }
                }
            }
        }

        // Master comes last
        order.push(RoutingNode::Master);
        order
    }

    /// Auto-route a track to master (used when creating new tracks)
    pub fn add_track_default_routing(&mut self, track_id: TrackId) {
        // Check if track already has any routing
        let has_routing = self
            .routing
            .iter()
            .any(|c| c.source == RoutingNode::Track(track_id));

        if !has_routing {
            self.routing.push(RoutingConnection::new(
                RoutingNode::Track(track_id),
                RoutingNode::Master,
            ));
        }
    }

    /// Remove all routing for a track (used when deleting tracks)
    pub fn remove_track_routing(&mut self, track_id: TrackId) {
        self.routing
            .retain(|c| c.source != RoutingNode::Track(track_id));
    }
}

impl ApplicationState {
    /// Get the mixer of a track ID
    pub fn get_mixer_from_track(&self, track_id: &TrackId) -> Option<Arc<TrackMixerChannel>> {
        // check if the track exists
        if self.tracks.get(track_id).is_none() {
            return None;
        }

        self.mixer.channels.get(track_id).cloned()
    }

    /// Get the entire mixer state
    pub fn get_mixer_state(&self) -> &MixerState {
        return &self.mixer;
    }
}
/// Helper to find cycle using DFS
fn find_cycle(
    node: BusId,
    adj: &hashbrown::HashMap<BusId, Vec<BusId>>,
    visited: &mut hashbrown::HashSet<BusId>,
    rec_stack: &mut hashbrown::HashSet<BusId>,
) -> bool {
    visited.insert(node);
    rec_stack.insert(node);

    if let Some(neighbors) = adj.get(&node) {
        for &neighbor in neighbors {
            if !visited.contains(&neighbor) {
                if find_cycle(neighbor, adj, visited, rec_stack) {
                    return true;
                }
            } else if rec_stack.contains(&neighbor) {
                return true;
            }
        }
    }

    rec_stack.remove(&node);
    false
}
