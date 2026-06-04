use karbeat_utils::define_id;
use serde::{Deserialize, Serialize};

define_id!(TrackId);
define_id!(ClipId);
define_id!(AutomationId);
define_id!(EffectId);
define_id!(BusId);
define_id!(PatternId);
define_id!(AudioSourceId);
define_id!(GeneratorId);
define_id!(SourceId);
define_id!(NoteId);

define_id!(ModulationId);

define_id!(ModulationLinkId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SidechainRouteId {
    Generator(GeneratorId),
    TrackEffect(TrackId, EffectId),
    BusEffect(BusId, EffectId),
    MasterEffect(EffectId),
}
