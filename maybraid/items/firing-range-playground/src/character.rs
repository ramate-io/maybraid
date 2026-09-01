//! Held firearm: chest pose, look-yaw clamp, handheld scale.

use avian3d::prelude::LinearVelocity;
use bevy::ecs::query::Has;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use crozon_characters::{
	character_bounds, species::braidman::BraidmanConfig, AnimBone, AnimClip, AnimRef, AnimRefRoot,
	BoneMap, CharacterMembers, CharacterRecipe, CharacterRig, CharacterRigRole, CharacterRoot,
	ComponentsOnly, RigSkeletonKind,
};
use firearms::{
	firearm_bounds, spawn_firearm_components, FireOnTrigger, FirearmConcept, FirearmMembers,
	FirearmRoot, Weapon,
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
/// Place the firing line between the right pectoral and eye, near `shoulder.R`.
const RIGHT_ALONG_SHOULDERS: f32 = 0.72;
/// Prefer a comfortably bent arm and reserve the final reach for aiming.
const PREFERRED_REACH: f32 = 0.48;
const MAX_REACH: f32 = 0.88;
/// Look yaw may lead the body by this much (full cone is 2×).
pub(crate) const AIM_YAW_LIMIT: f32 = FRAC_PI_6;

/// Nested character host on the player capsule.
#[derive(Component)]
pub(crate) struct PlayerVisual;

#[derive(Component)]
pub(crate) struct HeldFirearm {
	pub scale: f32,
}

#[derive(Debug, Clone, Copy)]
struct HandReach {
	origin: Vec3,
	socket_offset: Vec3,
	length: f32,
}

#[derive(Debug, Clone, Copy)]
struct AimLine {
	origin: Vec3,
	direction: Vec3,
}

impl AimLine {
	/// Pick the common point on the firing line that both socket reaches support.
	fn lock_for(self, hands: [HandReach; 2]) -> Vec3 {
		let direction = self.direction.normalize_or(Vec3::Z);
		let mut lower = 0.0_f32;
		let mut upper = f32::INFINITY;
		let mut preferred = 0.0;

		for hand in hands {
			let relative = self.origin + hand.socket_offset - hand.origin;
			let center = -relative.dot(direction);
			let perpendicular = relative + direction * center;
			let perpendicular_sq = perpendicular.length_squared();
			let maximum = hand.length * MAX_REACH;
			let radius = (maximum * maximum - perpendicular_sq).max(0.0).sqrt();
			lower = lower.max(center - radius);
			upper = upper.min(center + radius);

			let comfortable = hand.length * PREFERRED_REACH;
			let forward = (comfortable * comfortable - perpendicular_sq).max(0.0).sqrt();
			preferred += center + forward;
		}

		preferred /= hands.len() as f32;
		let along = if lower <= upper {
			preferred.clamp(lower, upper)
		} else {
			// No exact shared interval: split the least-overreach gap.
			(lower + upper) * 0.5
		}
		.max(0.0);
		self.origin + direction * along
	}
}

pub(crate) fn authored_length(bounds: Aabb3d) -> f32 {
	let size = bounds.max - bounds.min;
	size.x.max(size.y).max(size.z).max(1e-3)
}

pub(crate) fn held_scale_from_bounds(bounds: Aabb3d) -> f32 {
	(HELD_LENGTH / authored_length(bounds)).clamp(0.15, 1.0)
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
	let clothed = CharacterRecipe::clothed(&BraidmanConfig::default_preview());
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
	let scale = held_scale_from_bounds(bounds);
	let entities = spawn_firearm_components(&mut commands, &kit, Transform::IDENTITY, bounds);
	for entity in entities {
		commands.entity(entity).insert((
			Name::new("held-bullpup"),
			Weapon::bolt(),
			FireOnTrigger,
			HeldFirearm { scale },
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

fn shoulder_line_origin(shoulder_l: Vec3, shoulder_r: Vec3) -> Vec3 {
	let mid = (shoulder_l + shoulder_r) * 0.5;
	mid + (shoulder_r - mid) * RIGHT_ALONG_SHOULDERS
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
		(&FirearmMembers, &HeldFirearm, &GlobalTransform, &mut Transform),
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
	let Some((right_origin, right_length)) = arm_measure(members, &maps, &globals, "R") else {
		return;
	};
	let Some((left_origin, left_length)) = arm_measure(members, &maps, &globals, "L") else {
		return;
	};
	let facing = visual.rotation * Vec3::Z;
	let look = Quat::from_axis_angle(Vec3::Y, camera.yaw) * -Vec3::Z;
	let rotation = gun_aim_rotation(facing, look, camera.pitch);
	for (gun_members, held, previous_root, mut transform) in &mut guns {
		let Some(trigger_local) =
			firearm_landmark_local(gun_members, previous_root, &maps, &globals, "trigger_point")
		else {
			continue;
		};
		let Some(grip_local) =
			firearm_landmark_local(gun_members, previous_root, &maps, &globals, "grip_point")
		else {
			continue;
		};
		let line = AimLine {
			origin: shoulder_line_origin(shoulder_l, shoulder_r),
			direction: rotation * Vec3::Z,
		};
		let translation = line.lock_for([
			HandReach {
				origin: right_origin,
				socket_offset: rotation * (trigger_local * held.scale),
				length: right_length,
			},
			HandReach {
				origin: left_origin,
				socket_offset: rotation * (grip_local * held.scale),
				length: left_length,
			},
		]);
		*transform = Transform { translation, rotation, scale: Vec3::splat(held.scale) };
	}
}

/// Humerus origin and shoulder-to-palm length for one side.
fn arm_measure(
	members: &CharacterMembers,
	maps: &Query<&BoneMap, Without<HeldFirearm>>,
	globals: &Query<&GlobalTransform, Without<HeldFirearm>>,
	suffix: &str,
) -> Option<(Vec3, f32)> {
	let humerus = named_translation(members, maps, globals, &format!("humerus.{suffix}"))?;
	let forearm = named_translation(members, maps, globals, &format!("forearm.{suffix}"))?;
	Some((humerus, humerus.distance(forearm) * 2.0))
}

fn firearm_landmark_local(
	members: &FirearmMembers,
	root: &GlobalTransform,
	maps: &Query<&BoneMap, Without<HeldFirearm>>,
	globals: &Query<&GlobalTransform, Without<HeldFirearm>>,
	name: &str,
) -> Option<Vec3> {
	let world = named_translation_from(members.iter(), maps, globals, name)?;
	Some(root.affine().inverse().transform_point3(world))
}

fn named_translation(
	members: &CharacterMembers,
	maps: &Query<&BoneMap, Without<HeldFirearm>>,
	globals: &Query<&GlobalTransform, Without<HeldFirearm>>,
	name: &str,
) -> Option<Vec3> {
	named_translation_from(members.iter(), maps, globals, name)
}

fn named_translation_from(
	members: impl Iterator<Item = Entity>,
	maps: &Query<&BoneMap, Without<HeldFirearm>>,
	globals: &Query<&GlobalTransform, Without<HeldFirearm>>,
	name: &str,
) -> Option<Vec3> {
	for member in members {
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
	fn hold_locks_to_firing_line_at_comfortable_reach() {
		let line = AimLine { origin: Vec3::new(0.1, 1.9, 0.0), direction: Vec3::Z };
		let hands = [
			HandReach { origin: Vec3::new(0.3, 1.9, 0.0), socket_offset: Vec3::ZERO, length: 0.7 },
			HandReach { origin: Vec3::new(-0.3, 1.9, 0.0), socket_offset: Vec3::ZERO, length: 0.7 },
		];
		let at = line.lock_for(hands);
		assert!((at.x - line.origin.x).abs() < 1e-4, "{at}");
		assert!((at.y - line.origin.y).abs() < 1e-4, "{at}");
		assert!(at.z > 0.1 && at.z < 0.6, "{at}");
	}

	#[test]
	fn forward_socket_offset_tucks_root_back() {
		let line = AimLine { origin: Vec3::ZERO, direction: Vec3::Z };
		let centered = [
			HandReach { origin: Vec3::ZERO, socket_offset: Vec3::ZERO, length: 0.7 },
			HandReach { origin: Vec3::ZERO, socket_offset: Vec3::ZERO, length: 0.7 },
		];
		let forward = [
			HandReach { socket_offset: Vec3::Z * 0.2, ..centered[0] },
			HandReach { socket_offset: Vec3::Z * 0.2, ..centered[1] },
		];
		assert!(line.lock_for(forward).z < line.lock_for(centered).z);
	}

	#[test]
	fn pose_held_firearm_queries_are_disjoint() -> Result<(), bevy::ecs::system::RunSystemError> {
		let mut world = World::new();
		world.run_system_once(pose_held_firearm)?;
		Ok(())
	}

	#[test]
	fn held_scale_shrinks_meter_kit() {
		let bounds = Aabb3d::from_min_max(Vec3::new(-0.5, -0.5, -2.2), Vec3::new(0.5, 1.4, 0.5));
		let scale = held_scale_from_bounds(bounds);
		assert!(scale < 0.5, "{scale}");
		assert!((scale * 2.7 - HELD_LENGTH).abs() < 0.05, "{scale}");
	}
}
