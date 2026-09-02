//! Firearm combatant intelligence.
//!
//! [`FirearmIntelligence`] fields [`FirearmObjective`] and writes look / trigger.
//! [`FirearmMovementIntelligence`] fields [`FirearmMovementObjective`] and writes
//! [`movement_intelligence::MovementObjective`] plus
//! [`movement_intelligence::ReplanMovement`]. Neither crate locks onto entities
//! itself: perception fills the target lists.

mod combat;
mod movement;
mod plugin;
mod target;

pub use combat::{FirearmIntelligence, FirearmIntelligenceSettings};
pub use movement::{FirearmMovementIntelligence, FirearmMovementIntelligenceSettings};
pub use plugin::{FirearmIntelligencePlugin, FirearmIntelligenceSystems};
pub use target::{pick_target, CombatTarget, FirearmMovementObjective, FirearmObjective};
