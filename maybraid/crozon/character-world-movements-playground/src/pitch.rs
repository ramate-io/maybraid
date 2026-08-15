//! Avian pitch apply + copy jump-in-flight onto [`SuspendTerrainPitch`].

use bevy::ecs::query::Has;
use bevy::prelude::*;
use crozon_characters::{apply_terrain_pitch, SuspendTerrainPitch, TerrainPitch};
use ground_avian::AvianElevationProbe;
use lod::{LodLevelRoot, LodLevelRoots};

use crate::player::{Jumping, Player};

pub(crate) fn sync_suspend_terrain_pitch(
	mut commands: Commands,
	players: Query<(Entity, Has<Jumping>), With<Player>>,
) {
	for (entity, jumping) in &players {
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
	visuals: Query<(Entity, &mut Transform, &mut TerrainPitch, Option<&ChildOf>)>,
	parents: Query<(&GlobalTransform, Has<SuspendTerrainPitch>)>,
	children: Query<&Children>,
	level_roots_bags: Query<(), With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	visibilities: Query<&Visibility>,
	apply_pitch: Query<(), With<crozon_characters::ApplyTerrainPitch>>,
) {
	apply_terrain_pitch(
		time,
		probe,
		visuals,
		parents,
		children,
		level_roots_bags,
		root_keys,
		visibilities,
		apply_pitch,
	);
}
