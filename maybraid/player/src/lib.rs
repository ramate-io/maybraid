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
	apply_character_controller, apply_character_mobility, apply_locomotion_capsule,
	ground_plane_for_wish, tick_jump, walkable_contact_normal, wish_on_ground, CharacterController,
	CharacterLocomotion, Grounded, JumpPhase, JumpWish, Jumping, MoveWish, PlayerControlSystems,
	WalkableGround,
};
pub use identity::{
	CameraFollow, Npc, Player, PlayerCameraAim, PlayerCameraPose, PlayerCapsule, PlayerLook,
	PlayerUse, PlayerVisual, PlayerYawOwner,
};
pub use locomotion::drive_player_locomotion;
pub use spawn::{
	capsule_spawn_height, needs_npc_visual, needs_player_visual, spawn_npc, spawn_npc_visual,
	spawn_npc_with_hidden_capsule, spawn_npc_with_hull, spawn_player, spawn_player_visual,
	spawn_player_with_hidden_capsule, spawn_player_with_hull, LocomotionCapsule, CAPSULE_LENGTH,
	CAPSULE_RADIUS,
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

/// Locomotion-independent item presentation schedule.
///
/// Applications with their own capsule controller can install this without
/// adding [`PlayerPlugin`].
pub struct PlayerPresentationPlugin;

impl Plugin for PlayerPresentationPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(
				PlayerPoseSystems::Item.after(CharacterMotionSystems::Elevation),
				PlayerPoseSystems::Overlay.after(PlayerPoseSystems::Item),
			),
		);
	}
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<PlayerPresentationPlugin>() {
			app.add_plugins(PlayerPresentationPlugin);
		}
		app.init_resource::<CharacterLocomotion>()
			.configure_sets(
				Update,
				(
					PlayerSystems::Intent.after(CharacterControlSystems),
					PlayerSystems::Body.after(PlayerSystems::Intent).in_set(PlayerControlSystems),
					PlayerSystems::Locomotion
						.after(PlayerSystems::Body)
						.before(CharacterMotionSystems::Anim),
				),
			)
			.add_systems(Update, intent::apply_move_intents.in_set(PlayerSystems::Intent))
			.add_systems(
				Update,
				(
					body::update_grounded,
					body::apply_wish_movement,
					body::apply_wish_jump,
					body::advance_jump_phases,
					body::apply_movement_damping,
				)
					.chain()
					.in_set(PlayerSystems::Body),
			)
			.add_systems(PostUpdate, body::sync_character_locomotion)
			.add_systems(
				Update,
				(locomotion::face_wish_yaw, drive_player_locomotion)
					.in_set(PlayerSystems::Locomotion),
			);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Resource, Default)]
	struct PresentationOrder(Vec<&'static str>);

	fn note_elevation(mut order: ResMut<PresentationOrder>) {
		order.0.push("elevation");
	}

	fn note_item(mut order: ResMut<PresentationOrder>) {
		order.0.push("item");
	}

	fn note_overlay(mut order: ResMut<PresentationOrder>) {
		order.0.push("overlay");
	}

	#[test]
	fn presentation_runs_after_elevation_in_order() {
		let mut app = App::new();
		app.init_resource::<PresentationOrder>()
			.add_plugins(PlayerPresentationPlugin)
			.add_systems(
				Update,
				(
					note_elevation.in_set(CharacterMotionSystems::Elevation),
					note_item.in_set(PlayerPoseSystems::Item),
					note_overlay.in_set(PlayerPoseSystems::Overlay),
				),
			);
		app.update();
		assert_eq!(app.world().resource::<PresentationOrder>().0, ["elevation", "item", "overlay"]);
	}
}
