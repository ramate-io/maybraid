//! Brodler visual, walk/run clips, gun synced to `forearm.R` in world space.

use avian3d::prelude::LinearVelocity;
use bevy::ecs::query::Has;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use crozon_characters::{
	character_bounds, species::brodler::BrodlerConfig, AnimClip, AnimRef, AnimRefRoot, BoneMap,
	CharacterMembers, CharacterRecipe, CharacterRig, CharacterRigRole, CharacterRoot,
	ComponentsOnly, RigSkeletonKind,
};
use firearms::{
	firearm_bounds, spawn_firearm_components, FireOnTrigger, FirearmConcept, FirearmRoot, Weapon,
};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use std::f32::consts::FRAC_PI_2;

use crate::camera::CameraController;
use crate::hold::HoldingArms;
use crate::player::{CharacterController, Jumping, MoveWish, Player};

const WALK_SPEED: f32 = 1.0;
const RUN_SPEED: f32 = 5.0;
const HAND_BONE: &str = "forearm.R";
/// Meters along unscaled forearm +Y from the joint to the palm.
const FOREARM_TO_PALM: f32 = 0.18;
/// Gun-local offset after aim (right, down, forward along the bore).
const PALM_OFFSET: Vec3 = Vec3::new(0.04, -0.05, 0.08);

/// Nested character host on the player capsule.
#[derive(Component)]
pub(crate) struct PlayerVisual;

#[derive(Component)]
pub(crate) struct HeldFirearm;

/// World-space follow target: the character's `forearm.R` bone.
#[derive(Component)]
pub(crate) struct GunHandSocket(Entity);

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
	let entities = spawn_firearm_components(&mut commands, &kit, Transform::IDENTITY, bounds);
	for entity in entities {
		commands.entity(entity).insert((
			Name::new("held-bullpup"),
			Weapon::bolt(),
			FireOnTrigger,
			HeldFirearm,
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

pub(crate) fn bind_gun_socket(
	mut commands: Commands,
	guns: Query<Entity, (With<HeldFirearm>, With<FirearmRoot>, Without<GunHandSocket>)>,
	visuals: Query<&CharacterMembers, With<PlayerVisual>>,
	rigs: Query<(Entity, &CharacterRig, &BoneMap)>,
) {
	let Ok(gun) = guns.single() else {
		return;
	};
	let Ok(members) = visuals.single() else {
		return;
	};
	let Some(hand) = hand_bone(members, &rigs) else {
		return;
	};
	commands.entity(gun).insert(GunHandSocket(hand));
}

fn hand_bone(
	members: &CharacterMembers,
	rigs: &Query<(Entity, &CharacterRig, &BoneMap)>,
) -> Option<Entity> {
	for member in members.iter() {
		let Ok((_, rig, map)) = rigs.get(member) else {
			continue;
		};
		if rig.role != CharacterRigRole::Body {
			continue;
		}
		return map.by_name.get(HAND_BONE).copied();
	}
	None
}

/// World pose: socket translation, player yaw, camera pitch, authored scale.
pub(crate) fn gun_world_transform(hand: &GlobalTransform, facing: Vec3, pitch: f32) -> Transform {
	let rotation = gun_aim_rotation(facing, pitch);
	let along = (hand.rotation() * Vec3::Y).normalize_or(Vec3::Y);
	let socket = hand.translation() + along * FOREARM_TO_PALM;
	Transform { translation: socket + rotation * PALM_OFFSET, rotation, scale: Vec3::ONE }
}

/// Yaw so rest +Z matches the player's XZ facing; pitch matches the camera (look-down is negative).
pub(crate) fn gun_aim_rotation(facing: Vec3, pitch: f32) -> Quat {
	let xz = Vec3::new(facing.x, 0.0, facing.z);
	let yaw = if xz.length_squared() < 1e-8 {
		0.0
	} else {
		let n = xz.normalize();
		n.x.atan2(n.z)
	};
	Quat::from_rotation_y(yaw) * Quat::from_rotation_x(-pitch)
}

pub(crate) fn sync_gun_to_hand(
	cameras: Query<&CameraController, With<Camera3d>>,
	visuals: Query<&GlobalTransform, With<PlayerVisual>>,
	hands: Query<&GlobalTransform, Without<HeldFirearm>>,
	mut guns: Query<(&GunHandSocket, &mut Transform), With<HeldFirearm>>,
) {
	let Ok(camera) = cameras.single() else {
		return;
	};
	let Ok(visual) = visuals.single() else {
		return;
	};
	let facing = visual.rotation() * Vec3::Z;
	for (socket, mut transform) in &mut guns {
		let Ok(hand) = hands.get(socket.0) else {
			continue;
		};
		*transform = gun_world_transform(hand, facing, camera.pitch);
	}
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

	#[test]
	fn aim_yaw_sends_bore_along_player_facing() {
		let q = gun_aim_rotation(Vec3::X, 0.0);
		assert!((q * Vec3::Z - Vec3::X).length() < 1e-4, "bore {}", q * Vec3::Z);
		assert!((q * Vec3::NEG_Y - Vec3::NEG_Y).length() < 1e-4, "grip {}", q * Vec3::NEG_Y);
	}

	#[test]
	fn look_down_pitches_bore_down() {
		let q = gun_aim_rotation(Vec3::Z, -0.4);
		let bore = q * Vec3::Z;
		assert!(bore.y < 0.0, "bore {}", bore);
		assert!(bore.z > 0.0, "bore {}", bore);
	}
}
