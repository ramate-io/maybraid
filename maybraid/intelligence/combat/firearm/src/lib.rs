//! Firearm combatant intelligence.
//!
//! [`FirearmSpotting`] candidates become remembered [`SpottedTarget`] snapshots.
//! [`FirearmIntelligence`] fields [`FirearmObjective`] and writes look / trigger.
//! [`FirearmMovementIntelligence`] fields [`FirearmMovementObjective`] and writes
//! [`movement_intelligence::MovementObjective`] plus
//! [`movement_intelligence::ReplanMovement`].

mod combat;
mod movement;
mod plugin;
mod spotting;
mod target;

pub use combat::{FirearmIntelligence, FirearmIntelligenceSettings};
pub use movement::{FirearmMovementIntelligence, FirearmMovementIntelligenceSettings};
pub use plugin::{FirearmIntelligencePlugin, FirearmIntelligenceSystems};
pub use target::{
	pick_target, CombatTarget, FirearmMovementObjective, FirearmObjective, FirearmSpotting,
	SpottedTarget, TargetCapsule,
};
