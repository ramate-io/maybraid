//! After walk/run, aim both arms at the held firearm's hand landmarks.

use bevy::prelude::*;
use crozon_characters::{AnimBone, AnimMailbox, BoneMap, CharacterMembers};
use crozon_rigs::humanoid::HumanoidRig;
use crozon_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use crozon_rigs::{Name, Side};
use firearms::{FirearmMembers, FirearmRoot};

use crate::character::{HeldFirearm, PlayerVisual};

/// Body-rig marker: after locomotion, apply the firearm hold.
#[derive(Component, Clone, Copy, Default)]
pub(crate) struct HoldingArms;

const RIGHT_ELBOW: f32 = 0.95;
const LEFT_ELBOW: f32 = 1.15;
const SHOULDER_CARRY: f32 = 0.28;
const HUMERUS_ROLL: f32 = std::f32::consts::FRAC_PI_2;
const RIGHT_ALONG: Vec3 = Vec3::new(0.18, -0.12, 1.0);
const LEFT_ALONG: Vec3 = Vec3::new(-0.42, -0.18, 0.82);

/// Point both arms at `trigger_point` / `grip_point` (body-space fallback if missing).
pub(crate) fn sync_hands_to_firearm(
	visuals: Query<(&Transform, &CharacterMembers), (With<PlayerVisual>, Without<AnimBone>)>,
	guns: Query<&FirearmMembers, (With<HeldFirearm>, With<FirearmRoot>)>,
	gun_maps: Query<&BoneMap, Without<HoldingArms>>,
	globals: Query<&GlobalTransform>,
	mut rigs: Query<(&mut HumanoidV0Rig, &BoneMap, &AnimMailbox), With<HoldingArms>>,
	mut bones: Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<PlayerVisual>)>,
) {
	let Ok((visual, members)) = visuals.single() else {
		return;
	};
	let body_rot = visual.rotation;
	let trigger = gun_landmark(&guns, &gun_maps, &globals, "trigger_point");
	let grip = gun_landmark(&guns, &gun_maps, &globals, "grip_point");

	for member in members.iter() {
		let Ok((mut rig, map, mailbox)) = rigs.get_mut(member) else {
			continue;
		};
		if mailbox.output.is_empty() {
			continue;
		}
		rig.pose = mailbox.output.clone();
		let right_along = along_to(body_rot, shoulder_world(map, &globals, "humerus.R"), trigger)
			.unwrap_or(RIGHT_ALONG.normalize());
		let left_along = along_to(body_rot, shoulder_world(map, &globals, "humerus.L"), grip)
			.unwrap_or(LEFT_ALONG.normalize());
		pose_arm(
			&mut rig,
			Side::Right,
			right_along,
			HUMERUS_ROLL * Side::Right.sign(),
			RIGHT_ELBOW,
		);
		pose_arm(
			&mut rig,
			Side::Left,
			left_along,
			HUMERUS_ROLL * Side::Left.sign() * 0.55,
			LEFT_ELBOW,
		);
		write_arm_bones(&rig, map, &mut bones);
	}
}

fn gun_landmark(
	guns: &Query<&FirearmMembers, (With<HeldFirearm>, With<FirearmRoot>)>,
	maps: &Query<&BoneMap, Without<HoldingArms>>,
	globals: &Query<&GlobalTransform>,
	name: &str,
) -> Option<Vec3> {
	let members = guns.single().ok()?;
	named_translation(members.iter(), maps, globals, name)
}

fn shoulder_world(map: &BoneMap, globals: &Query<&GlobalTransform>, name: &str) -> Option<Vec3> {
	let entity = *map.by_name.get(name)?;
	globals.get(entity).ok().map(|global| global.translation())
}

fn named_translation(
	members: impl Iterator<Item = Entity>,
	maps: &Query<&BoneMap, Without<HoldingArms>>,
	globals: &Query<&GlobalTransform>,
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

/// Direction in body space (rig identity = mesh +Z forward).
fn along_to(body_rot: Quat, from: Option<Vec3>, to: Option<Vec3>) -> Option<Vec3> {
	let dir = to? - from?;
	if dir.length_squared() < 1e-6 {
		return None;
	}
	Some((body_rot.inverse() * dir).normalize_or(Vec3::Z))
}

fn pose_arm(rig: &mut HumanoidV0Rig, side: Side, along_body: Vec3, roll: f32, elbow: f32) {
	let mut arm = rig.arm_pose(side);
	arm.shoulder = rig.articulate_on_rig(arm.shoulder, SHOULDER_CARRY, 0.0);
	arm.humerus = rig.humerus_along_with_roll(side, along_body, roll);
	arm.forearm = rig.articulate_on_rig(arm.forearm, 0.0, elbow);
	rig.pose_arm(arm);
}

fn write_arm_bones(
	rig: &HumanoidV0Rig,
	map: &BoneMap,
	bones: &mut Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<PlayerVisual>)>,
) {
	for side in [Side::Left, Side::Right] {
		let arm = rig.arm(side);
		for name in
			[arm.shoulder.name.as_str(), arm.humerus.name.as_str(), arm.forearm.name.as_str()]
		{
			let Some(&entity) = map.by_name.get(name) else {
				continue;
			};
			let Some(pose) = rig.pose().get(&Name::from(name)) else {
				continue;
			};
			let Ok((_, mut transform)) = bones.get_mut(entity) else {
				continue;
			};
			*transform = pose.transform;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

	#[test]
	fn sync_hands_queries_are_disjoint() {
		let mut world = World::new();
		world
			.run_system_once(sync_hands_to_firearm)
			.expect("disjoint Transform / BoneMap queries");
	}
}
