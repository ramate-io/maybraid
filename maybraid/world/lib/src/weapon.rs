//! Starter held kit for the vegetation player capsule.

use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	CameraController, Player as VegetationPlayer, PlayerVisual,
};
use crozon_characters::CharacterRoot;
use firearm_user::{spawn_held_firearm, FirearmUser};
use player::{
	CameraFollow, Player as MaybraidPlayer, PlayerCameraAim, PlayerLook, PlayerPoseSystems,
};

use crate::camera::CameraPov;

/// Give the world player a bullpup kit once the Crozon visual exists.
///
/// [`firearm_user`] fire/pose query [`MaybraidPlayer`] / [`PlayerLook`]. Those
/// markers are not on the vegetation capsule, so stamp them here without the
/// player-crate locomotion controller (world already drives that capsule).
pub(crate) fn arm_world_player(
	mut commands: Commands,
	players: Query<Entity, (With<VegetationPlayer>, Without<FirearmUser>)>,
	visuals: Query<&ChildOf, (With<PlayerVisual>, With<CharacterRoot>)>,
) {
	for player in &players {
		if !visuals.iter().any(|child| child.parent() == player) {
			continue;
		}
		commands.entity(player).insert((
			MaybraidPlayer,
			PlayerLook::default(),
			PlayerCameraAim::default(),
			CameraFollow,
		));
		spawn_held_firearm(&mut commands, player);
	}
}

pub(crate) fn sync_player_look_from_camera(
	pov: Res<CameraPov>,
	cameras: Query<&CameraController, With<Camera3d>>,
	mut looks: Query<&mut PlayerLook, With<VegetationPlayer>>,
) {
	let Ok(camera) = cameras.single() else {
		return;
	};
	let first_person = *pov == CameraPov::FirstPerson;
	for mut look in &mut looks {
		look.yaw = camera.yaw;
		look.pitch = camera.pitch;
		look.first_person = first_person;
	}
}

pub(crate) fn configure(app: &mut App) {
	app.add_systems(Update, arm_world_player)
		.add_systems(Update, sync_player_look_from_camera.before(PlayerPoseSystems::Item));
}
