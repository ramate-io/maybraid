//! Starter held kit for the vegetation player capsule.

use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	Player as VegetationPlayer, PlayerVisual as VegetationPlayerVisual, PlaygroundMode,
};
use crozon_characters::CharacterRoot;
use firearm_user::{spawn_held_firearm, spawn_reticle, FirearmUser};
use player::{
	CameraFollow, Player as MaybraidPlayer, PlayerCameraAim, PlayerLook,
	PlayerVisual as MaybraidPlayerVisual, PlayerYawOwner,
};

use crate::control::WorldGameplayEnabled;

/// Give the world player a bullpup kit once the Crozon visual exists.
///
/// [`firearm_user`] fire/pose query [`MaybraidPlayer`] / [`PlayerLook`]. Those
/// markers are not on the vegetation capsule, so stamp them here without the
/// player-crate locomotion controller (world already drives that capsule).
pub(crate) fn arm_world_player(
	mut commands: Commands,
	mode: Res<PlaygroundMode>,
	gameplay: Res<WorldGameplayEnabled>,
	players: Query<(Entity, Has<FirearmUser>), With<VegetationPlayer>>,
	visuals: Query<
		(Entity, &ChildOf),
		(With<VegetationPlayerVisual>, With<CharacterRoot>, Without<MaybraidPlayerVisual>),
	>,
) {
	for (player, armed) in &players {
		let Some((visual, _)) = visuals.iter().find(|(_, child)| child.parent() == player) else {
			continue;
		};
		commands.entity(visual).insert((MaybraidPlayerVisual, PlayerYawOwner::Wish));
		if armed {
			continue;
		}
		let mut player_commands = commands.entity(player);
		player_commands.insert((
			MaybraidPlayer,
			PlayerLook::default(),
			PlayerCameraAim::default(),
			PlayerYawOwner::Wish,
		));
		if *mode == PlaygroundMode::Character && gameplay.0 {
			player_commands.insert(CameraFollow);
		}
		spawn_held_firearm(&mut commands, player);
	}
}

fn spawn_world_reticle(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	spawn_reticle(&mut commands, &mut meshes, &mut materials);
}

pub(crate) fn configure(app: &mut App) {
	app.add_systems(Startup, spawn_world_reticle)
		.add_systems(Update, arm_world_player);
}
