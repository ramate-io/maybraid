//! Held firearm: chest pose, look-yaw clamp, handheld scale.

use avian3d::prelude::LinearVelocity;
use bevy::ecs::query::Has;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use crozon_characters::{
	character_bounds, species::brodler::BrodlerConfig, AnimBone, AnimClip, AnimRef, AnimRefRoot,
	BoneMap, CharacterMembers, CharacterRecipe, CharacterRig, CharacterRigRole, CharacterRoot,
	ComponentsOnly, RigSkeletonKind,
};
use firearms::{
	firearm_bounds, spawn_firearm_components, FireOnTrigger, FirearmConcept, FirearmRoot, Weapon,
};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_6, PI, TAU};

use crate::camera::CameraController;
use crate::hold::HoldingArms;
use crate::player::{CharacterController, Jumping, MoveWish, Player};

const WALK_SPEED: f32 = 1.0;
const RUN_SPEED: f32 = 5.0;
/// Kit GLBs are meter-authored; a held bullpup should be about this long.
const HELD_LENGTH: f32 = 0.72;
/// Receiver sits this fraction of the held gun's world length in front of the chest.
const FORWARD_OF_LENGTH: f32 = 0.4;
/// Fraction of the way from chest-mid to `shoulder.R`.
const RIGHT_ALONG_SHOULDERS: f32 = 0.4;
/// Look yaw may lead the body by this much (full cone is 2×).
pub(crate) const AIM_YAW_LIMIT: f32 = FRAC_PI_6;

/// Nested character host on the player capsule.
#[derive(Component)]
pub(crate) struct PlayerVisual;

#[derive(Component)]
pub(crate) struct HeldFirearm {
	pub scale: f32,
	/// Longest AABB side of the meter-authored kit, before handheld scale.
	pub authored_length: f32,
}

pub(crate) fn authored_length(bounds: Aabb3d) -> f32 {
	let size = bounds.max - bounds.min;
	size.x.max(size.y).max(size.z).max(1e-3)
}

pub(crate) fn held_scale_from_bounds(bounds: Aabb3d) -> f32 {
	(HELD_LENGTH / authored_length(bounds)).clamp(0.15, 1.0)
}

pub(crate) fn gun_world_length(held: &HeldFirearm) -> f32 {
	held.scale * held.authored_length
}

pub(crate) fn spawn_player_character(
	mut commands: Commands,
	players: Query<Entity, With<Player>>,
	visuals: Query<Entity, With<PlayerVisual>>,
) {
	if !visuals.is_empty() {
		return;
	}
	let Ok(player) = players.single() else {
		return;
	};
	let clothed = CharacterRecipe::clothed(&BrodlerConfig::default_preview());
	let bounds = character_bounds(&clothed);
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let host = ComponentsOnly(clothed);
	let facing = Quat::from_rotation_y(-FRAC_PI_2);
	let visual = commands
		.spawn_scene((
			host.host(&lod_ref),
			bsn! {
				template_value(Transform::from_rotation(facing))
			},
		))
		.id();
	commands
		.entity(visual)
		.insert((ChildOf(player), PlayerVisual, Name::new("player-visual")));
}

pub(crate) fn spawn_held_firearm(mut commands: Commands) {
	let kit = FirearmConcept::Bullpup.kit();
	let bounds = firearm_bounds(&kit);
	let length = authored_length(bounds);
	let scale = held_scale_from_bounds(bounds);
	let entities = spawn_firearm_components(&mut commands, &kit, Transform::IDENTITY, bounds);
	for entity in entities {
		commands.entity(entity).insert((
			Name::new("held-bullpup"),
			Weapon::bolt(),
			FireOnTrigger,
			HeldFirearm { scale, authored_length: length },
		));
	}
}

pub(crate) fn stamp_holding_arms(
	mut commands: Commands,
	visuals: Query<&CharacterMembers, With<PlayerVisual>>,
	rigs: Query<(Entity, &CharacterRig), Without<HoldingArms>>,
) {
	for members in &visuals {
		for member in members.iter() {
			let Ok((entity, rig)) = rigs.get(member) else {
				continue;
			};
			if rig.role == CharacterRigRole::Body && rig.skeleton == RigSkeletonKind::Humanoid {
				commands.entity(entity).insert(HoldingArms);
			}
		}
	}
}

/// Horizontal yaw with +Z = 0, +X = +π/2.
pub(crate) fn yaw_xz(dir: Vec3) -> f32 {
	let xz = Vec3::new(dir.x, 0.0, dir.z);
	if xz.length_squared() < 1e-8 {
		0.0
	} else {
		let n = xz.normalize();
		n.x.atan2(n.z)
	}
}

pub(crate) fn wrap_pi(angle: f32) -> f32 {
	(angle + PI).rem_euclid(TAU) - PI
}

/// Look yaw, clamped to ±[`AIM_YAW_LIMIT`] of body facing.
pub(crate) fn clamped_aim_yaw(facing: Vec3, look: Vec3) -> f32 {
	let face = yaw_xz(facing);
	let delta = wrap_pi(yaw_xz(look) - face).clamp(-AIM_YAW_LIMIT, AIM_YAW_LIMIT);
	face + delta
}

pub(crate) fn gun_aim_rotation(facing: Vec3, look: Vec3, pitch: f32) -> Quat {
	Quat::from_rotation_y(clamped_aim_yaw(facing, look)) * Quat::from_rotation_x(-pitch)
}

pub(crate) fn hold_translation(
	shoulder_l: Vec3,
	shoulder_r: Vec3,
	facing: Vec3,
	gun_world_length: f32,
) -> Vec3 {
	let mid = (shoulder_l + shoulder_r) * 0.5;
	let facing = Vec3::new(facing.x, 0.0, facing.z).normalize_or(Vec3::Z);
	mid + facing * (gun_world_length * FORWARD_OF_LENGTH)
		+ (shoulder_r - mid) * RIGHT_ALONG_SHOULDERS
}

pub(crate) fn pose_held_firearm(
	cameras: Query<&CameraController, With<Camera3d>>,
	visuals: Query<
		(&Transform, &CharacterMembers),
		(With<PlayerVisual>, Without<HeldFirearm>, Without<AnimBone>),
	>,
	maps: Query<&BoneMap, Without<HeldFirearm>>,
	globals: Query<&GlobalTransform, Without<HeldFirearm>>,
	mut guns: Query<
		(&HeldFirearm, &mut Transform),
		(With<FirearmRoot>, Without<Player>, Without<PlayerVisual>),
	>,
) {
	let Ok(camera) = cameras.single() else {
		return;
	};
	let Ok((visual, members)) = visuals.single() else {
		return;
	};
	let Some(shoulder_l) = named_translation(members, &maps, &globals, "shoulder.L") else {
		return;
	};
	let Some(shoulder_r) = named_translation(members, &maps, &globals, "shoulder.R") else {
		return;
	};
	let facing = visual.rotation * Vec3::Z;
	let look = Quat::from_axis_angle(Vec3::Y, camera.yaw) * -Vec3::Z;
	let rotation = gun_aim_rotation(facing, look, camera.pitch);
	for (held, mut transform) in &mut guns {
		let translation = hold_translation(shoulder_l, shoulder_r, facing, gun_world_length(held));
		*transform = Transform { translation, rotation, scale: Vec3::splat(held.scale) };
	}
}

fn named_translation(
	members: &CharacterMembers,
	maps: &Query<&BoneMap, Without<HeldFirearm>>,
	globals: &Query<&GlobalTransform, Without<HeldFirearm>>,
	name: &str,
) -> Option<Vec3> {
	for member in members.iter() {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let Some(&entity) = map.by_name.get(name) else {
			continue;
		};
		if let Ok(global) = globals.get(entity) {
			return Some(global.translation());
		}
	}
	None
}

pub(crate) fn drive_player_locomotion(
	mut commands: Commands,
	controllers: Query<(&LinearVelocity, &MoveWish, Has<Jumping>), With<CharacterController>>,
	visuals: Query<(&CharacterMembers, &ChildOf), (With<PlayerVisual>, With<CharacterRoot>)>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	for (members, child_of) in &visuals {
		let Ok((velocity, _wish, jumping)) = controllers.get(child_of.parent()) else {
			continue;
		};
		let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
		for member in members.iter() {
			let Ok(rig) = rigs.get(member) else {
				continue;
			};
			if rig.role != CharacterRigRole::Body {
				continue;
			}
			let clip = if jumping && speed > RUN_SPEED {
				AnimClip::leap()
			} else if jumping {
				AnimClip::jump()
			} else if speed > RUN_SPEED {
				AnimClip::run()
			} else if speed > WALK_SPEED {
				AnimClip::walk()
			} else {
				AnimClip::still()
			};
			let desired = AnimRef::new(clip);
			let needs = match anims.get(member) {
				Ok(root) => root.0 != desired,
				Err(_) => true,
			};
			if needs {
				commands.entity(member).insert(AnimRefRoot(desired));
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

	#[test]
	fn facing_plus_x_sends_bore_plus_x() {
		let q = gun_aim_rotation(Vec3::X, Vec3::X, 0.0);
		assert!((q * Vec3::Z - Vec3::X).length() < 1e-4, "bore {}", q * Vec3::Z);
		assert!((q * Vec3::NEG_Y - Vec3::NEG_Y).length() < 1e-4, "grip {}", q * Vec3::NEG_Y);
	}

	#[test]
	fn look_within_limit_tracks() {
		let look = Quat::from_rotation_y(0.2) * Vec3::Z;
		let yaw = clamped_aim_yaw(Vec3::Z, look);
		assert!((yaw - 0.2).abs() < 1e-4, "{yaw}");
	}

	#[test]
	fn look_beyond_limit_clamps() {
		let yaw = clamped_aim_yaw(Vec3::Z, Vec3::X);
		assert!((yaw - AIM_YAW_LIMIT).abs() < 1e-4, "{yaw}");
		let yaw = clamped_aim_yaw(Vec3::Z, Vec3::NEG_X);
		assert!((yaw + AIM_YAW_LIMIT).abs() < 1e-4, "{yaw}");
	}

	#[test]
	fn look_down_pitches_bore_down() {
		let q = gun_aim_rotation(Vec3::Z, Vec3::Z, -0.4);
		let bore = q * Vec3::Z;
		assert!(bore.y < 0.0, "bore {bore}");
		assert!(bore.z > 0.0, "bore {bore}");
	}

	#[test]
	fn hold_sits_at_shoulder_height() {
		let left = Vec3::new(-0.4, 1.9, 0.0);
		let right = Vec3::new(0.4, 1.9, 0.0);
		let at = hold_translation(left, right, Vec3::Z, 0.72);
		assert!((at.y - 1.9).abs() < 1e-4, "{at}");
		assert!(at.z > 0.2, "{at}");
		assert!(at.x > 0.0, "{at}");
	}

	#[test]
	fn hold_forward_scales_with_gun_length() {
		let left = Vec3::new(-0.2, 1.0, 0.0);
		let right = Vec3::new(0.2, 1.0, 0.0);
		let short = hold_translation(left, right, Vec3::Z, 0.5);
		let long = hold_translation(left, right, Vec3::Z, 1.0);
		assert!((long.z - short.z - 0.5 * FORWARD_OF_LENGTH).abs() < 1e-4);
	}

	#[test]
	fn pose_held_firearm_queries_are_disjoint() {
		let mut world = World::new();
		world
			.run_system_once(pose_held_firearm)
			.expect("disjoint player / visual / gun Transform queries");
	}

	#[test]
	fn held_scale_shrinks_meter_kit() {
		let bounds = Aabb3d::from_min_max(Vec3::new(-0.5, -0.5, -2.2), Vec3::new(0.5, 1.4, 0.5));
		let scale = held_scale_from_bounds(bounds);
		assert!(scale < 0.5, "{scale}");
		assert!((scale * 2.7 - HELD_LENGTH).abs() < 0.05, "{scale}");
	}
}
