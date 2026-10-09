//! Environment-driven groups that place generated semantic mob scenes.
//!
//! [`Barking<G>`] is the mob model. It samples lower layers through traits it
//! owns, implemented here for the concrete stack it sits on.

mod config;
mod generation;
mod index;
mod plugin;
mod present;
mod sample;
pub mod shared;

pub use config::BarkingConfig;
pub use generation::{
	GroupKind, MobEnvironmentSample, MobGroup, MobPlantHost, MobWorldHosts, MobWorldSample,
	PlacedMob, DEFAULT_GROUP_EXTENT,
};
pub use index::{MobCell, MobCellExtent};
pub use plugin::{MobGroupsPlugin, PendingMobGroups};
pub use present::{MobCellRoot, MobGroupRoot};
pub use shared::{
	BarkingPresentationPlugin, MobGround, MobNeighborhood,
	MobScenePresentationPlugin, PlacedMobCell, MOB_PRESENT_RADIUS,
};

#[cfg(test)]
mod tests;
