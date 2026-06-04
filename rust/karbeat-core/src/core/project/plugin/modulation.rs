use std::sync::Arc;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::{
    audio::event::PluginTarget,
    core::project::{
        automation::AutomationTarget, ApplicationState, AutomationCurveType, AutomationLane,
        AutomationPoint,
    },
    shared::{AutomationId, BusId, ModulationId, ModulationLinkId, TrackId},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
        lane_id: AutomationId,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ModulationSource {
    PeakController { source: PluginTarget },
    Automation { lane_id: AutomationId },
    LFO { rate_hz: f32 },
    // Future: Envelope, MacroKnob, StepSequencer, etc.
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ModulationLink {
    pub id: ModulationLinkId,
    pub source_id: ModulationId,  // Which LFO/Macro is driving this?
    pub target: AutomationTarget, // What parameter is being turned?
    pub depth: f32,               // How much is it turning? (-1.0 to 1.0)
    pub base_value: f32,          // The center point of the parameter
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ModulationLinkForOrderedLaneView {
    pub order_idx: usize,
    pub prop: ModulationLink,
}

impl ModulationEvent {
    pub fn target(&self) -> &AutomationTarget {
        match self {
            ModulationEvent::PeakController { target, .. } => target,
            ModulationEvent::Automation { target, .. } => target,
            ModulationEvent::LFO { target, .. } => target,
        }
    }
}

impl ApplicationState {
    // =========================================================================
    // MODULATION ROUTING (The Generic Master Methods)
    // =========================================================================

    /// Creates a new signal generator (e.g., an LFO)
    pub fn add_modulation_source(&mut self, source: ModulationSource) -> ModulationId {
        let new_id = ModulationId::next(&mut self.modulation_counter);
        self.modulation_sources.insert(new_id, source);
        new_id
    }

    /// Removes a generator AND all cables connected to it
    pub fn remove_modulation_source(&mut self, source_id: ModulationId) {
        self.modulation_sources.remove(&source_id);
        // Cascade delete: Remove any cables that were plugged into this source
        self.modulation_links
            .retain(|_, link| link.prop.source_id != source_id);
    }

    /// Connects a Generator to a Parameter
    pub fn link_modulation(
        &mut self,
        source_id: ModulationId,
        target: AutomationTarget,
        depth: f32,
        base_value: f32,
    ) -> anyhow::Result<ModulationLinkId> {
        // Ensure the source actually exists
        if !self.modulation_sources.contains_key(&source_id) {
            return Err(anyhow::anyhow!("Modulation source not found"));
        }

        // Check if THIS specific source is already linked to THIS specific target.
        if self
            .modulation_links
            .values()
            .any(|l| l.prop.source_id == source_id && l.prop.target == target)
        {
            return Err(anyhow::anyhow!(
                "This source is already linked to this target"
            ));
        }

        let link_id = ModulationLinkId::next(&mut self.modulation_link_counter);

        let order_idx = self
            .modulation_links
            .values()
            .filter(|l| l.prop.target == target)
            .count();

        let link = ModulationLink {
            id: link_id,
            source_id,
            target,
            depth,
            base_value,
        };

        let ordered_link = ModulationLinkForOrderedLaneView {
            order_idx,
            prop: link,
        };

        self.modulation_links.insert(link_id, ordered_link);
        log::info!(
            "Successfully linked source {:?} to target via link2 {:?}",
            source_id,
            link_id
        );

        Ok(link_id)
    }

    /// Change the depth/amount of a specific connection
    pub fn update_link_depth(&mut self, link_id: ModulationLinkId, new_depth: f32) {
        if let Some(link) = self.modulation_links.get_mut(&link_id) {
            link.prop.depth = new_depth;
        }
    }

    /// Removes a specific connection
    pub fn remove_modulation_link(&mut self, link_id: ModulationLinkId) {
        self.modulation_links.remove(&link_id);
    }

    // =========================================================================
    // AUTOMATION POOL MANAGEMENT (Using the Generic Routing)
    // =========================================================================

    /// Add an automation lane to the pool and link it to a target.
    pub fn add_automation_lane(
        &mut self,
        target: AutomationTarget,
        label: impl Into<String>,
        min: f32,
        max: f32,
        default_value: f32,
    ) -> anyhow::Result<(Arc<AutomationLane>, ModulationLinkId)> {
        // 1. Create the Lane (Pure Data)
        let lane_id = AutomationId::next(&mut self.automation_counter);
        let lane = Arc::new(AutomationLane::new(lane_id, label, min, max, default_value));
        self.automation_pool.insert(lane_id, lane.clone());

        // 2. Create the Source Generator
        let source = ModulationSource::Automation { lane_id };
        let source_id = self.add_modulation_source(source);

        // 3. Connect the Cable (Depth 1.0, Base Value 0.0 for standard 1:1 automation tracking)
        let link_id = self.link_modulation(source_id, target, 1.0, 0.0)?;

        log::info!(
            "Added automation lane {:?} mapped via link {:?}",
            lane_id,
            link_id
        );
        Ok((lane, link_id))
    }

    /// Add an automation lane specifically validated for a Track target.
    pub fn add_automation_lane_for_track(
        &mut self,
        track_id: TrackId,
        target: AutomationTarget,
        label: impl Into<String>,
        min: f32,
        max: f32,
        default_value: f32,
    ) -> anyhow::Result<(Arc<AutomationLane>, ModulationLinkId)> {
        if !target.references_track(track_id) {
            return Err(anyhow!("Target does not reference the specified track"));
        }
        self.add_automation_lane(target, label, min, max, default_value)
    }

    /// Get all Modulations AND their associated Automation Lanes for a specific Track.
    pub fn get_automation_lanes_for_track(
        &self,
        track_id: TrackId,
    ) -> Vec<(ModulationLinkId, AutomationId, Arc<AutomationLane>)> {
        let mut lanes: Vec<_> = self
            .modulation_links
            .values()
            .filter_map(|link| {
                if link.prop.target.references_track(track_id) {
                    if let Some(ModulationSource::Automation { lane_id }) =
                        self.modulation_sources.get(&link.prop.source_id)
                    {
                        if let Some(lane) = self.automation_pool.get(lane_id) {
                            return Some((link.order_idx, link.prop.id, *lane_id, lane.clone()));
                        }
                    }
                }
                None
            })
            .collect();

        lanes.sort_by_key(|item| item.0);
        lanes
            .into_iter()
            .map(|item| (item.1, item.2, item.3))
            .collect()
    }

    /// Completely remove all Modulation Links and orphaned Automation Lanes for a Track.
    pub fn remove_modulations_for_track(&mut self, track_id: TrackId) {
        let mut orphaned_lanes = Vec::new();

        self.modulation_links.retain(|_, link| {
            let references = link.prop.target.references_track(track_id);

            if references {
                if let Some(ModulationSource::Automation { lane_id }) =
                    self.modulation_sources.get(&link.prop.source_id)
                {
                    orphaned_lanes.push(*lane_id);
                }
            }
            !references // Keep if it does NOT reference the track
        });

        // Clean up the pure data lanes so we don't leak memory
        for lane_id in orphaned_lanes {
            self.automation_pool.remove(&lane_id);
        }
    }

    /// Add an automation lane specifically validated for a Bus target.
    pub fn add_automation_lane_for_bus(
        &mut self,
        bus_id: BusId,
        target: AutomationTarget,
        label: impl Into<String>,
        min: f32,
        max: f32,
        default_value: f32,
    ) -> anyhow::Result<(Arc<AutomationLane>, ModulationLinkId)> {
        if !target.references_bus(bus_id) {
            return Err(anyhow!("Target does not reference the specified bus"));
        }
        self.add_automation_lane(target, label, min, max, default_value)
    }

    /// Get all Modulations AND their associated Automation Lanes for a specific Bus.
    pub fn get_automation_lanes_for_bus(
        &self,
        bus_id: BusId,
    ) -> Vec<(ModulationLinkId, AutomationId, Arc<AutomationLane>)> {
        let mut lanes: Vec<_> = self
            .modulation_links
            .values()
            .filter_map(|link| {
                if link.prop.target.references_bus(bus_id) {
                    if let Some(ModulationSource::Automation { lane_id }) =
                        self.modulation_sources.get(&link.prop.source_id)
                    {
                        if let Some(lane) = self.automation_pool.get(lane_id) {
                            return Some((link.order_idx, link.prop.id, *lane_id, lane.clone()));
                        }
                    }
                }
                None
            })
            .collect();

        lanes.sort_by_key(|item| item.0);
        lanes
            .into_iter()
            .map(|item| (item.1, item.2, item.3))
            .collect()
    }

    /// Completely remove all Modulations and orphaned Automation Lanes for a Bus.
    pub fn remove_modulations_for_bus(&mut self, bus_id: BusId) {
        let mut orphaned_lanes = Vec::new();

        self.modulation_links.retain(|_, link| {
            let references = link.prop.target.references_bus(bus_id);
            if references {
                if let Some(ModulationSource::Automation { lane_id }) =
                    self.modulation_sources.get(&link.prop.source_id)
                {
                    orphaned_lanes.push(*lane_id);
                }
            }
            !references
        });

        for lane_id in orphaned_lanes {
            self.automation_pool.remove(&lane_id);
        }
    }

    /// Add a Host LFO to control a parameter (Example of generic usage)
    pub fn add_lfo_modulation(
        &mut self,
        target: AutomationTarget,
        rate_hz: f32,
        depth: f32,
        base_value: f32,
    ) -> anyhow::Result<ModulationLinkId> {
        let source_id = self.add_modulation_source(ModulationSource::LFO { rate_hz });
        self.link_modulation(source_id, target, depth, base_value)
    }

    // =========================================================================
    // AUTOMATION POINT MANAGEMENT
    // =========================================================================

    pub fn add_automation_point(
        &mut self,
        lane_id: AutomationId,
        time_ticks: u32,
        value: f32,
    ) -> anyhow::Result<()> {
        let lane_arc = self
            .automation_pool
            .get_mut(&lane_id)
            .ok_or_else(|| anyhow!("Automation lane {:?} not found", lane_id))?;

        let lane = Arc::make_mut(lane_arc);
        let point = AutomationPoint::with_curve(
            time_ticks,
            value,
            AutomationCurveType::Linear,
            lane.min,
            lane.max,
        );
        lane.add_point(point);
        Ok(())
    }

    pub fn remove_automation_point(
        &mut self,
        lane_id: AutomationId,
        point_index: usize,
    ) -> anyhow::Result<AutomationPoint> {
        let lane_arc = self
            .automation_pool
            .get_mut(&lane_id)
            .ok_or_else(|| anyhow!("Automation lane {:?} not found", lane_id))?;

        let lane = Arc::make_mut(lane_arc);
        lane.remove_point(point_index).ok_or_else(|| {
            anyhow!(
                "Point index {} out of bounds (lane has {} points)",
                point_index,
                lane.points.len()
            )
        })
    }

    pub fn update_automation_point(
        &mut self,
        lane_id: AutomationId,
        point_index: usize,
        time_ticks: u32,
        value: f32,
        tension: f32,
    ) -> anyhow::Result<(usize, usize)> {
        let lane_arc = self
            .automation_pool
            .get_mut(&lane_id)
            .ok_or_else(|| anyhow!("Automation lane {:?} not found", lane_id))?;

        let lane = Arc::make_mut(lane_arc);
        match lane.update_point(point_index, time_ticks, value, tension) {
            Some(new_index) => Ok((point_index, new_index)),
            None => Err(anyhow!(
                "Point index {} out of bounds (lane has {} points)",
                point_index,
                lane.points.len()
            )),
        }
    }

    /// ======================================
    /// Update Modulation Link Order (Drag and Drop UI Support)
    /// Scopes the sorting strictly to the UI accordion/drawer.
    /// ======================================
    pub fn update_link_order(
        &mut self,
        link_id: ModulationLinkId,
        new_idx: usize,
    ) -> anyhow::Result<()> {
        // Find the target to determine which UI drawer we are sorting inside
        let target = self
            .modulation_links
            .get(&link_id)
            .map(|l| l.prop.target.clone())
            .ok_or_else(|| anyhow::anyhow!("Link {:?} not found", link_id))?;

        // Gather ALL links that live in this exact same UI drawer
        let mut sibling_links: Vec<_> = self
            .modulation_links
            .values()
            .filter(|l| l.prop.target.belongs_to_same_drawer_as(&target))
            .cloned()
            .collect();

        // Sort them by their current UI order
        sibling_links.sort_by_key(|link| link.order_idx);

        // Find the link's current position among its siblings
        let current_pos = sibling_links
            .iter()
            .position(|link| link.prop.id == link_id)
            .unwrap();

        // Move the link in the sorted vector
        let target_idx = new_idx.min(sibling_links.len().saturating_sub(1));
        let link = sibling_links.remove(current_pos);
        sibling_links.insert(target_idx, link);

        // Write the new sequential indices back to the global HashMap
        for (i, sibling) in sibling_links.iter().enumerate() {
            if let Some(global_link) = self.modulation_links.get_mut(&sibling.prop.id) {
                global_link.order_idx = i;
            }
        }

        Ok(())
    }
}
