//! Capsule player, visual, and handoff slots for camera / pose drivers.

mod body;
mod identity;
mod intent;
mod locomotion;
mod spawn;

use bevy::prelude::*;
use crozon_characters::CharacterMotionSystems;
use maybraid_character_controller::CharacterControlSystems;

pub use body::{
	CharacterController, Grounded, Jumping, MoveWish, MovementAction, PlayerControlSystems,
};
pub use identity::{
	CameraFollow, Npc, Player, PlayerCameraAim, PlayerCameraPose, PlayerCapsule, PlayerLook,
	PlayerUse, PlayerVisual, PlayerYawOwner,
};
pub use locomotion::drive_player_locomotion;
pub use spawn::{
	capsule_spawn_height, needs_npc_visual, needs_player_visual, spawn_npc, spawn_npc_visual,
	spawn_npc_with_hidden_capsule, spawn_player, spawn_player_visual,
	spawn_player_with_hidden_capsule, CAPSULE_LENGTH, CAPSULE_RADIUS,
};

/// Capsule physics and move/jump intents.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerSystems {
	Intent,
	Body,
	Locomotion,
}

/// Handoff for item-user crates. `Item` poses held meshes; `Overlay` runs after clips.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerPoseSystems {
	Item,
	Overlay,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<MovementAction>()
			.configure_sets(
				Update,
				(
					PlayerSystems::Intent.after(CharacterControlSystems),
					PlayerSystems::Body.after(PlayerSystems::Intent).in_set(PlayerControlSystems),
					PlayerPoseSystems::Item.after(PlayerSystems::Body),
					PlayerSystems::Locomotion
						.after(PlayerPoseSystems::Item)
						.before(CharacterMotionSystems::Anim),
					PlayerPoseSystems::Overlay.after(CharacterMotionSystems::Anim),
				),
			)
			.add_systems(Update, intent::apply_move_intents.in_set(PlayerSystems::Intent))
			.add_systems(
				Update,
				(
					body::update_grounded,
					body::apply_character_movement,
					body::apply_wish_movement,
					body::apply_movement_damping,
				)
					.chain()
					.in_set(PlayerSystems::Body),
			)
			.add_systems(
				Update,
				(locomotion::face_wish_yaw, drive_player_locomotion)
					.in_set(PlayerSystems::Locomotion),
			);
	}
}
