//! Environment-driven groups that place generated semantic mob scenes.

mod generation;
mod plugin;

pub use generation::{
	GroupKind, MobEnvironmentSample, MobGroup, MobWorldSample, PlacedMob, DEFAULT_GROUP_EXTENT,
};
pub use plugin::{MobGroupsPlugin, PendingMobGroups};
