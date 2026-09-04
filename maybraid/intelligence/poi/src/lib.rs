//! Shared POI identity, spatial discovery, retained knowledge, and goal handoff.
//!
//! [`LocalPoi`] values are indexed through Gimme for bounded scans.
//! [`GlobalPoi`] values form a deliberately sparse whole-map set. Both scans
//! feed the same per-user [`PoiKnowledge`] that external systems can augment
//! with [`PoiObservation`] messages.

mod discovery;
mod goal;
mod kind;
mod knowledge;
mod marker;
mod plugin;
mod policy;
mod registry;
mod source;
mod visit;

pub use discovery::{discover_pois, ingest_poi_observations, sync_poi_registry};
pub use goal::{
	begin_poi_goal, complete_poi_goals, drive_poi_goals, refresh_poi_goals, PoiGoal,
	PoiGoalCompleted, PoiGoalState, PoiGoalStatus,
};
pub use kind::{PoiId, PoiKind};
pub use knowledge::{KnownPoi, PoiIntelligenceUser, PoiKnowledge, PoiObservation};
pub use marker::{GlobalPoi, LocalPoi, Poi, MAX_POI_ARRIVAL_RADIUS};
pub use plugin::{PoiIntelligencePlugin, PoiSystems};
pub use policy::{PoiInterest, PoiInterests, PoiLearningPolicy, PoiVisitPolicy};
pub use registry::{PoiRecord, PoiRegistry};
pub use source::PoiSource;
pub use visit::{choose_poi, PoiVisitState};

#[cfg(test)]
mod tests;
