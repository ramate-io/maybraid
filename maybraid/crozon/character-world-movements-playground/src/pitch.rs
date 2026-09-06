//! Avian pitch apply + copy jump-in-flight onto [`SuspendTerrainPitch`].

use bevy::ecs::query::Has;
use bevy::prelude::*;
use crozon_characters::{
	apply_terrain_pitch, ApplyTerrainPitch, SuspendTerrainPitch, TerrainPitch,
	TerrainPitchUsesVisualYaw,
};
use ground_avian::AvianElevationProbe;

use crate::player::{CharacterController, Jumping};

pub(crate) fn sync_suspend_terrain_pitch(
	mut commands: Commands,
	controllers: Query<(Entity, Has<Jumping>), With<CharacterController>>,
) {
	for (entity, jumping) in &controllers {
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
