//! Environment-driven groups that place generated semantic mob scenes.
//!
//! [`Barking<G>`] is the mob model. It samples lower layers through traits it
//! owns, implemented here for the concrete stack it sits on.

mod config;
mod generation;
mod index;
mod model;
mod plugin;
mod present;
mod sample;
mod stream;

pub use config::BarkingConfig;
pub use generation::{
	GroupKind, MobEnvironmentSample, MobGroup, MobPlantHost, MobWorldHosts, MobWorldSample,
	PlacedMob, DEFAULT_GROUP_EXTENT,
};
pub use index::{MobCell, MobCellExtent, MobIndex};
pub use model::Barking;
pub use plugin::{MobGroupsPlugin, PendingMobGroups};
pub use present::{MobCellRoot, MobGroupRoot};
pub use sample::{
	DiscoverablePlaces, ForestSelection, PlantHosts, SelectUrbanization, UrbanSelection,
};
pub use stream::{install_mob_grid_stream, MobCellWrites, MobLodChan};

#[cfg(test)]
mod present_tests;
#[cfg(test)]
mod tests;
