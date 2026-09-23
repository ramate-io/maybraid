//! Training Ground runs the firing-range free-for-all inside Maybraid.
//!
//! The shell sets [`TrainingGroundActive`]. This plugin mounts the Les Halles
//! roster while that flag is on and parks the streamed-world body so it is not
//! a second combatant.

use avian3d::prelude::{ColliderDisabled, RigidBodyDisabled};
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::Player as WorldPlayer;
use firing_range_playground::FreeForAllPlugin;
pub use firing_range_playground::{FreeForAllLive, FreeForAllMounted};

/// The game shell sets this while Training Ground is the live world session.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrainingGroundActive(pub bool);

/// Stamped on the streamed-world body so Leave can restore its collider.
#[derive(Component, Clone, Copy, Debug, Default)]
struct ParkedWorldPlayer;

pub struct TrainingGroundPlugin;

impl Plugin for TrainingGroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<TrainingGroundActive>()
			.add_plugins(FreeForAllPlugin)
			.add_systems(Update, (sync_free_for_all_gate, park_world_player).chain());
	}
}

fn sync_free_for_all_gate(
	active: Res<TrainingGroundActive>,
	live: Option<Res<FreeForAllLive>>,
	mut commands: Commands,
) {
	if active.0 && live.is_none() {
		commands.insert_resource(FreeForAllLive);
	} else if !active.0 && live.is_some() {
		commands.remove_resource::<FreeForAllLive>();
	}
}

fn park_world_player(
	live: Option<Res<FreeForAllLive>>,
	mut commands: Commands,
	waiting: Query<Entity, (With<WorldPlayer>, Without<ParkedWorldPlayer>)>,
	parked: Query<Entity, With<ParkedWorldPlayer>>,
) {
	if live.is_some() {
		for entity in &waiting {
			commands.entity(entity).insert((
				RigidBodyDisabled,
				ColliderDisabled,
				ParkedWorldPlayer,
			));
		}
		return;
	}
	for entity in &parked {
		commands
			.entity(entity)
			.remove::<(RigidBodyDisabled, ColliderDisabled, ParkedWorldPlayer)>();
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;

	use super::*;

	#[test]
	fn inactive_gate_leaves_the_roster_down() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(TrainingGroundActive(false));
		world
			.run_system_once(sync_free_for_all_gate)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get_resource::<FreeForAllLive>().is_none());
		Ok(())
	}
}
