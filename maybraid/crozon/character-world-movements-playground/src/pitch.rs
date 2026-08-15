//! Avian pitch apply + copy jump-in-flight onto [`SuspendTerrainPitch`].

use bevy::ecs::query::Has;
use bevy::prelude::*;
use crozon_characters::{
	apply_terrain_pitch, ApplyTerrainPitch, SuspendTerrainPitch, TerrainPitch,
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
		(Entity, &mut Transform, &mut TerrainPitch, Option<&ChildOf>),
		With<ApplyTerrainPitch>,
	>,
	parents: Query<(&GlobalTransform, Has<SuspendTerrainPitch>)>,
) {
	apply_terrain_pitch(time, probe, visuals, parents);
}
