//! Firearm combatant intelligence.
//!
//! Generic spotting writes visual contacts, combat targeting ranks them, and
//! firearm policy realizes the selected contact through movement, aim, and fire.

mod combat;
mod movement;
mod plugin;
mod spotting;
mod targeting;

pub use combat::{FirearmIntelligence, FirearmIntelligenceSettings};
pub use movement::{FirearmMovementIntelligence, FirearmMovementIntelligenceSettings};
pub use plugin::{FirearmIntelligencePlugin, FirearmIntelligenceSystems};
pub use targeting::{AimTrajectory, FirearmTargeting};
