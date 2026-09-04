//! Firearm hold, pose, fire, and sight-aim for any [`FirearmUser`].

mod aim;
mod fire;
mod hold;
mod pose;
mod reticle;
mod rumble;
mod weapon;

use bevy::prelude::*;
use crozon_characters::CharacterMotionSystems;
use damage::DamageSystems;
use firearms::FirearmWeaponSystems;
use maybraid_input::PadRumbleSystems;
use player::{PlayerPoseSystems, PlayerSystems};
use player_camera::PlayerCameraSystems;
use std::f32::consts::FRAC_PI_2;

pub use hold::{sync_hands_to_firearm, HoldingArms};
pub use pose::{
	pose_held_firearm, spawn_held_firearm, spawn_held_firearm_with, spawn_held_kit,
	stamp_holding_arms, HeldFirearm,
};
pub use reticle::{spawn_reticle, Reticle};
pub use weapon::{live_weapon_from_stats, LiveWeapon, RECOIL_PITCH_PER_UNIT};

/// Firearm-user schedule points other combat systems can order against.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FirearmUserSystems {
	/// Travel the queued recoil path into look / camera.
	Recoil,
}

/// Capsule/NPC using a firearm.
///
/// This is a 1:1 Bevy relationship onto the kit (`held`). Inserting it stamps
/// [`HeldBy`] on the gun, so despawn/replace stays consistent and queries can
/// go either way. A raw `Entity` field is better when the link is ephemeral or
/// many-to-many; a hold is neither.
#[derive(Component, Debug, Clone, Copy)]
#[relationship(relationship_target = HeldBy)]
pub struct FirearmUser {
	#[relationship]
	pub held: Entity,
	pub settings: FirearmUserSettings,
}

impl FirearmUser {
	pub fn holding(held: Entity) -> Self {
		Self { held, settings: FirearmUserSettings::default() }
	}
}

/// Per-user hold / aim knobs. Defaults match the firing-range bullpup.
#[derive(Clone, Copy, Debug)]
pub struct FirearmUserSettings {
	/// Sit this far behind `sight_camera_socket` along the bore.
	pub sight_camera_back: f32,
	/// Target world length of a held kit (meters).
	pub held_length: f32,
	/// Fraction of the humerus-to-humerus half-width toward the trigger arm.
	pub stock_along_right_chest: f32,
	/// Clearance forward of the shoulder pocket, as a fraction of arm length.
	pub stock_forward_of_arm_reach: f32,
	/// Look yaw may lead the body by this much in third person (full cone is 2×).
	pub aim_yaw_limit: f32,
	pub right_pole: Vec3,
	pub left_pole: Vec3,
	pub left_reach_stretch: f32,
	pub firing_torso_yaw: f32,
	pub humerus_roll: f32,
	pub grip_socket: &'static str,
}

impl Default for FirearmUserSettings {
	fn default() -> Self {
		Self {
			sight_camera_back: 0.05,
			held_length: 0.72,
			stock_along_right_chest: 0.82,
			stock_forward_of_arm_reach: 0.3,
			aim_yaw_limit: std::f32::consts::FRAC_PI_6 / 2.0,
			right_pole: Vec3::new(-1.0, -1.0, -0.1),
			left_pole: Vec3::new(0.65, -0.55, 0.5),
			left_reach_stretch: 1.15,
			firing_torso_yaw: -0.84,
			humerus_roll: FRAC_PI_2,
			grip_socket: "grip",
		}
	}
}

/// Gun-side 1:1 target of [`FirearmUser`].
#[derive(Component, Debug)]
#[relationship_target(relationship = FirearmUser)]
pub struct HeldBy(Entity);

pub struct FirearmUserPlugin;

impl Plugin for FirearmUserPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<damage::DamagePlugin>() {
			app.add_plugins(damage::DamagePlugin);
		}
		app.add_message::<maybraid_input::PadRumble>()
			.add_systems(Update, fire::apply_fire_intents.in_set(PlayerSystems::Intent))
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
			.add_systems(
				PostUpdate,
				(reticle::ingest_hit_markers, reticle::update_reticle)
					.chain()
					.after(TransformSystems::Propagate)
					.after(DamageSystems::Apply),
			)
			.add_systems(
				PostUpdate,
				rumble::pulse_combat_rumble
					.after(FirearmWeaponSystems::Fire)
					.after(DamageSystems::Apply)
					.before(PadRumbleSystems::FanOut),
			)
			.add_systems(PostUpdate, fire::queue_weapon_recoil.after(FirearmWeaponSystems::Fire))
			.add_systems(
				Update,
				fire::advance_weapon_recoil
					.in_set(PlayerCameraSystems::Body)
					.in_set(FirearmUserSystems::Recoil),
			);
	}
}
