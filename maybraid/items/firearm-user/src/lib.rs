//! Firearm hold, pose, fire, and sight-aim for any [`FirearmUser`].

mod aim;
mod fire;
mod hold;
mod pose;
mod reticle;

use bevy::prelude::*;
use crozon_characters::CharacterMotionSystems;
use maybraid_player::{PlayerPoseSystems, PlayerSystems};
use maybraid_player_camera::PlayerCameraSystems;

pub use hold::HoldingArms;
pub use pose::{spawn_held_firearm, HeldFirearm};
pub use reticle::{spawn_reticle, Reticle};

/// Capsule/NPC that is currently using a firearm (`held` is the [`firearms::FirearmRoot`]).
#[derive(Component, Clone, Copy, Debug)]
pub struct FirearmUser {
	pub held: Entity,
}

pub struct FirearmUserPlugin;

impl Plugin for FirearmUserPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, fire::apply_fire_intents.in_set(PlayerSystems::Intent))
			.add_systems(
				Update,
				(pose::stamp_holding_arms, pose::pose_held_firearm).in_set(PlayerPoseSystems::Item),
			)
			.add_systems(Update, aim::write_sight_aim.in_set(PlayerCameraSystems::Aim))
			.add_systems(
				Update,
				hold::sync_hands_to_firearm
					.in_set(PlayerPoseSystems::Overlay)
					.after(CharacterMotionSystems::Anim),
			)
			.add_systems(PostUpdate, reticle::update_reticle.after(TransformSystems::Propagate));
	}
}
