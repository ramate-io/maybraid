//! Avian pitch apply + copy jump-in-flight onto [`SuspendTerrainPitch`].

use bevy::ecs::query::Has;
use bevy::prelude::*;
use crozon_characters::{
	apply_terrain_pitch, ApplyTerrainPitch, SuspendTerrainPitch, TerrainPitch,
};
use ground_avian::AvianElevationProbe;
use player::{CharacterController, Jumping};

use crate::playground_player::Player;

pub(crate) fn sync_suspend_terrain_pitch(
	mut commands: Commands,
	players: Query<(Entity, Has<Jumping>), With<Player>>,
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
		(Entity, &mut Transform, &mut TerrainPitch, Option<&ChildOf>),
		With<ApplyTerrainPitch>,
	>,
	parents: Query<(&GlobalTransform, Has<SuspendTerrainPitch>)>,
) {
	apply_terrain_pitch(time, probe, visuals, parents);
}
