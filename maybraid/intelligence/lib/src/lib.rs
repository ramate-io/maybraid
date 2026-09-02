//! Movement location, objective, and per-entity movement intelligence.
//!
//! Install [`MovementIntelligence`] on a capsule and register
//! [`MovementIntelligencePlugin`] with a [`MovementIntelligenceSurface`]
//! [`bevy::ecs::system::SystemParam`]. The brain writes [`player::MoveWish`];
//! it does not own physics or lock onto other entities.

mod ability;
mod candidate;
mod location;
mod objective;
mod plugin;
mod step;
mod surface;
mod user;

use bevy::prelude::*;
use player::PlayerSystems;

pub use ability::{MovementAbility, MovementBody};
pub use candidate::{MovementCandidate, MovementCandidateHints};
pub use location::MovementLocation;
pub use objective::MovementObjective;
pub use plugin::MovementIntelligencePlugin;
pub use step::{MovementDrive, MovementStep};
pub use surface::{CandidateBudget, MovementIntelligenceSurface};
pub use user::{MovementIntelligence, MovementIntelligenceSettings, ReplanMovement};

/// Plan, then write [`player::MoveWish`], before capsule accel.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MovementIntelligenceSystems {
	Replan,
	Drive,
}

pub(crate) fn configure_movement_intelligence_sets(app: &mut App) {
	app.configure_sets(
		Update,
		(MovementIntelligenceSystems::Replan, MovementIntelligenceSystems::Drive)
			.chain()
			.before(PlayerSystems::Body),
	);
}
