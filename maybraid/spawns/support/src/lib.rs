//! Shared bounded support geometry and persistent holding for existing actors.
//!
//! Domains own destinations, identities and retries. This leaf only decides
//! whether fixed collision geometry supports an actor's configured capsule.

pub mod actor;
pub mod policy;
pub mod probe;
pub mod readiness;
pub mod search;
pub mod terrain;

pub use actor::{
	register_actor_support, ActorSupportSearchPolicy, ActorSupportSystems, PendingActorSupport,
};
pub use policy::{PlacementPolicy, PlacementSupportHit, SupportPolicy};
pub use probe::{
	clearance_blocked, probe_search, probe_search_with_locomotion, SupportProbeOutcome,
};
pub use readiness::{column_presentation_ready, column_volume, SupportReadiness};
pub use search::{
	PlacementPose, PlacementSearch, INTERIOR_LOWER_MARGIN, INTERIOR_UPPER_MARGIN,
	OUTDOOR_LOWER_MARGIN, OUTDOOR_UPPER_MARGIN,
};
pub use terrain::{holding_elevation, surface_or_hold_at, terrain_height_at};

/// Geometric failure, independent of any player-arrival request vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupportProbeFailure {
	InvalidBounds,
	NoEligibleSupport,
	Clearance,
	Placement,
}
