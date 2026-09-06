//! Avian pitch apply + jump suspend for the vegetation player and NPC capsules.

use bevy::ecs::query::Has;
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{Jumping as VegetationJumping, Player};
use crozon_characters::{
	apply_terrain_pitch, ApplyTerrainPitch, SuspendTerrainPitch, TerrainPitch,
	TerrainPitchUsesVisualYaw,
};
use ground_avian::AvianElevationProbe;
use player::{CharacterController, Jumping};

pub(crate) fn sync_suspend_terrain_pitch(
	mut commands: Commands,
	players: Query<(Entity, Has<VegetationJumping>), With<Player>>,
	npcs: Query<(Entity, Has<Jumping>), (With<CharacterController>, Without<Player>)>,
) {
	for (entity, jumping) in players.iter().chain(npcs.iter()) {
		if jumping {
			commands.entity(entity).insert(SuspendTerrainPitch);
		} else {
			commands.entity(entity).remove::<SuspendTerrainPitch>();
		}
	}
}

/// Concrete system: [`AvianElevationProbe`] is a [`bevy::ecs::system::SystemParam`].
pub(crate) fn apply_avian_terrain_pitch(
	time: Res<Time>,
	probe: AvianElevationProbe,
	visuals: Query<
		(
			Entity,
			&mut Transform,
			&GlobalTransform,
			&mut TerrainPitch,
			Has<TerrainPitchUsesVisualYaw>,
		),
		With<ApplyTerrainPitch>,
	>,
	child_of: Query<&ChildOf>,
	parents: Query<&GlobalTransform, Without<ApplyTerrainPitch>>,
	suspended: Query<(), With<SuspendTerrainPitch>>,
) {
	apply_terrain_pitch(time, probe, visuals, child_of, parents, suspended);
}
