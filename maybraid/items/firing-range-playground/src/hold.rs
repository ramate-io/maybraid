//! Hold overlay: overwrite arm bones after walk/run so the gun stays aimed.

use bevy::prelude::*;
use crozon_characters::{AnimBone, AnimMailbox, BoneMap, CharacterMembers};
use crozon_rigs::humanoid::HumanoidRig;
use crozon_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use crozon_rigs::{Name, Side};

use crate::character::PlayerVisual;

/// Body-rig marker: after locomotion, apply the firearm hold.
#[derive(Component, Clone, Copy, Default)]
pub(crate) struct HoldingArms;

const RIGHT_ELBOW: f32 = 0.45;
const LEFT_ELBOW: f32 = 0.95;
const SHOULDER_CARRY: f32 = 0.12;
const HUMERUS_ROLL: f32 = std::f32::consts::FRAC_PI_2;

/// Right arm points body +Z (mesh forward); left reaches a bit across and down.
pub(crate) fn apply_hold_pose(
	visuals: Query<(&GlobalTransform, &CharacterMembers), With<PlayerVisual>>,
	mut rigs: Query<(&mut HumanoidV0Rig, &BoneMap, &AnimMailbox), With<HoldingArms>>,
	mut bones: Query<(&AnimBone, &mut Transform), Without<AnimMailbox>>,
) {
	let Ok((root_global, members)) = visuals.single() else {
		return;
	};
	let forward = root_global.rotation() * Vec3::Z;
	let left_along = (forward * 0.85 + root_global.rotation() * Vec3::new(-0.4, -0.2, 0.0))
		.normalize_or(forward);

	for member in members.iter() {
		let Ok((mut rig, map, mailbox)) = rigs.get_mut(member) else {
			continue;
		};
		if mailbox.output.is_empty() {
			continue;
		}
		rig.pose = mailbox.output.clone();
		pose_arm(&mut rig, Side::Right, forward, HUMERUS_ROLL * Side::Right.sign(), RIGHT_ELBOW);
		pose_arm(
			&mut rig,
			Side::Left,
			left_along,
			HUMERUS_ROLL * Side::Left.sign() * 0.6,
			LEFT_ELBOW,
		);
		write_arm_bones(&rig, map, &mut bones);
	}
}

fn pose_arm(rig: &mut HumanoidV0Rig, side: Side, along_world: Vec3, roll: f32, elbow: f32) {
	let mut arm = rig.arm_pose(side);
	arm.shoulder = rig.articulate_on_rig(arm.shoulder, SHOULDER_CARRY, 0.0);
	arm.humerus = rig.humerus_along_with_roll(side, along_world, roll);
	arm.forearm = rig.articulate_on_rig(arm.forearm, 0.0, elbow);
	rig.pose_arm(arm);
}

fn write_arm_bones(
	rig: &HumanoidV0Rig,
	map: &BoneMap,
	bones: &mut Query<(&AnimBone, &mut Transform), Without<AnimMailbox>>,
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
