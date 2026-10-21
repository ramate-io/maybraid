//! Cell / encounter population demand and retirement.
//!
//! Streamed mob presentation and roster spawn stay in `mob-scenes` / `barking`.
//! This crate owns demand identity and cancellation so a retired cell cannot
//! realize a later placement or actor.

pub mod controller;
pub mod ownership;

pub use controller::{
	begin_population_placements, retire_population, PopulationController, PopulationDemand,
	PopulationRetired, TrackedDemand,
};
pub use ownership::{PopulationOwner, PopulationOwnership};

use bevy::prelude::*;
use maybraid_spawns_actors::CancelActorSpawn;
use maybraid_spawns_placement::{CancelPlacement, ResolvePlacement};

#[derive(Resource, Default)]
struct PopulationSpawnRuntime;

pub fn register_population_spawn_runtime(app: &mut App) {
	if app.world().contains_resource::<PopulationSpawnRuntime>() {
		return;
	}
	app.init_resource::<PopulationSpawnRuntime>()
		.init_resource::<PopulationOwnership>()
		.add_message::<PopulationDemand>()
		.add_message::<PopulationRetired>()
		.add_message::<ResolvePlacement>()
		.add_message::<CancelPlacement>()
		.add_message::<CancelActorSpawn>()
		.add_systems(Update, (begin_population_placements, retire_population).chain());
}

pub struct PopulationSpawnPlugin;

impl Plugin for PopulationSpawnPlugin {
	fn build(&self, app: &mut App) {
		register_population_spawn_runtime(app);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn register_population_spawn_runtime_is_idempotent() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_population_spawn_runtime(&mut app);
		register_population_spawn_runtime(&mut app);
		assert!(app.world().contains_resource::<PopulationOwnership>());
	}
}
