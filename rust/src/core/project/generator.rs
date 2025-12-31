use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::{
    core::project::{plugin::instance::PluginInstance, ApplicationState},
    define_id,
};

define_id!(GeneratorId);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeneratorInstance {
    pub id: GeneratorId,
    pub effects: Arc<Vec<PluginInstance>>,
    pub instance_type: GeneratorInstanceType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GeneratorInstanceType {
    // A Synth (Internal or VST)
    Plugin(PluginInstance),

    // A Sampler (Plays a file from AssetLibrary)
    Sampler { asset_id: u32, root_note: u8 },

    // Audio Input (Microphone / Line In)
    AudioInput { device_channel_index: u32 },
}

impl ApplicationState {
    pub fn add_generator(&mut self, instance_type: GeneratorInstanceType) -> GeneratorId {
        let id = GeneratorId::next(&mut self.generator_counter);

        // Ensure the inner instance knows its ID
        let instance = GeneratorInstance {
            id,
            instance_type,
            effects: Arc::new(Default::default()),
        };

        self.generator_pool
            .insert(id, Arc::new(RwLock::new(instance)));
        id
    }

    /// Deletes a generator source and removes all clips referencing it.
    pub fn remove_generator(&mut self, generator_id: GeneratorId) -> Option<GeneratorId> {
        if self.generator_pool.remove(&generator_id).is_none() {
            return None;
        }

        for track_arc in self.tracks.values_mut() {
            let track = Arc::make_mut(track_arc);
            track.remove_clip_by_source_id(generator_id, true);
        }

        self.update_max_sample_index();
        Some(generator_id)
    }
}
