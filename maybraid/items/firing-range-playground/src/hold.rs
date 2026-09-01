//! After walk/run, aim both arms at the held firearm's hand landmarks.

use bevy::prelude::*;
use crozon_characters::{AnimBone, AnimMailbox, BoneMap, CharacterMembers};
use crozon_rigs::articulation::{TwoBoneAim, BONE_LENGTH_AXIS};
use crozon_rigs::humanoid::HumanoidRig;
use crozon_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use crozon_rigs::{Name, Side};
use firearms::{FirearmMembers, FirearmRoot};

use crate::character::{HeldFirearm, PlayerVisual};

/// Body-rig marker: after locomotion, apply the firearm hold.
#[derive(Component, Clone, Copy, Default)]
pub(crate) struct HoldingArms;

const HUMERUS_ROLL: f32 = std::f32::consts::FRAC_PI_2;
const RIGHT_POLE: Vec3 = Vec3::new(0.25, -1.0, -0.1);
const LEFT_POLE: Vec3 = Vec3::new(-0.4, -1.0, 0.05);
/// Imported humanoid articulation needs negative swing to draw its right shoulder rearward.
const FIRING_TORSO_YAW: f32 = -0.84;

/// Point both arms at `trigger_point` / `grip_point`.
pub(crate) fn sync_hands_to_firearm(
	visuals: Query<(&GlobalTransform, &CharacterMembers), (With<PlayerVisual>, Without<AnimBone>)>,
	guns: Query<
		(&FirearmMembers, &Transform, &GlobalTransform),
		(With<HeldFirearm>, With<FirearmRoot>, Without<AnimBone>),
	>,
	gun_maps: Query<&BoneMap, Without<HoldingArms>>,
	globals: Query<&GlobalTransform>,
	mut rigs: Query<(&mut HumanoidV0Rig, &BoneMap, &AnimMailbox), With<HoldingArms>>,
	mut bones: Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<PlayerVisual>)>,
) {
	let Ok((visual, members)) = visuals.single() else {
		return;
	};
	let body_rot = visual.rotation();
	let trigger = gun_landmark(&guns, &gun_maps, &globals, "trigger_point");
	let grip = gun_landmark(&guns, &gun_maps, &globals, "grip_point");
	let (Some(trigger), Some(grip)) = (trigger, grip) else {
		return;
	};

	for member in members.iter() {
		let Ok((mut rig, map, mailbox)) = rigs.get_mut(member) else {
			continue;
		};
		if mailbox.output.is_empty() {
			continue;
		}
		rig.pose = mailbox.output.clone();
		pose_firing_torso(&mut rig);
		reset_arm_to_rest(&mut rig, map, &bones, Side::Right);
		reset_arm_to_rest(&mut rig, map, &bones, Side::Left);
		let Some(right) = arm_reach(
			&rig,
			Side::Right,
			target_from(body_rot, bone_world(map, &globals, "humerus.R"), trigger),
			RIGHT_POLE,
		) else {
			continue;
		};
		let Some(left) = arm_reach(
			&rig,
			Side::Left,
			target_from(body_rot, bone_world(map, &globals, "humerus.L"), grip),
			LEFT_POLE,
		) else {
			continue;
		};
		pose_arm(&mut rig, Side::Right, right);
		pose_arm(&mut rig, Side::Left, left);
		write_hold_bones(&rig, map, &mut bones);
	}
}

fn gun_landmark(
	guns: &Query<
		(&FirearmMembers, &Transform, &GlobalTransform),
		(With<HeldFirearm>, With<FirearmRoot>, Without<AnimBone>),
	>,
	maps: &Query<&BoneMap, Without<HoldingArms>>,
	globals: &Query<&GlobalTransform>,
	name: &str,
) -> Option<Vec3> {
	let (members, current_root, previous_root) = guns.single().ok()?;
	let previous_landmark = named_translation(members.iter(), maps, globals, name)?;
	let landmark_local = previous_root.affine().inverse().transform_point3(previous_landmark);
	Some(current_root.compute_affine().transform_point3(landmark_local))
}

fn bone_world(map: &BoneMap, globals: &Query<&GlobalTransform>, name: &str) -> Option<Vec3> {
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

fn target_from(body_rot: Quat, from: Option<Vec3>, to: Vec3) -> Option<Vec3> {
	let dir = to - from?;
	if dir.length_squared() < 1e-6 {
		return None;
	}
	Some(body_rot.inverse() * dir)
}

fn arm_reach(
	rig: &HumanoidV0Rig,
	side: Side,
	target: Option<Vec3>,
	pole: Vec3,
) -> Option<TwoBoneAim> {
	let arm = rig.arm_pose(side);
	let upper_length = arm.forearm.transform.translation.length();
	TwoBoneAim::reach(target?, pole, upper_length, upper_length)
}

fn pose_arm(rig: &mut HumanoidV0Rig, side: Side, reach: TwoBoneAim) {
	let roll = best_humerus_roll(rig, side, reach, HUMERUS_ROLL * side.sign());
	let mut arm = rig.arm_pose(side);
	arm.humerus = rig.humerus_along_with_roll(side, reach.upper_along, roll);
	rig.pose_arm(arm);
	let mut arm = rig.arm_pose(side);
	arm.forearm = rig.articulate_on_rig(arm.forearm, 0.0, reach.flex);
	rig.pose_arm(arm);
}

fn pose_firing_torso(rig: &mut HumanoidV0Rig) {
	let mut spine = rig.spine_pose();
	// Keep the hips nearly square and blade the shoulder girdle from the chest.
	spine.lumbar = rig.articulate_on_rig(spine.lumbar, FIRING_TORSO_YAW * 0.05, 0.0);
	spine.midback = rig.articulate_on_rig(spine.midback, FIRING_TORSO_YAW * 0.20, 0.0);
	spine.upper_back = rig.articulate_on_rig(spine.upper_back, FIRING_TORSO_YAW * 0.75, 0.0);
	rig.pose_spine(spine);
}

/// Pick the long-axis roll whose elbow flex points the forearm at the target.
///
/// This tiny one-dimensional IK solve handles the mirrored forearm hinge axes
/// without baking another left/right sign rule into the playground.
fn best_humerus_roll(rig: &HumanoidV0Rig, side: Side, reach: TwoBoneAim, fallback: f32) -> f32 {
	const STEPS: usize = 96;
	let mut best = fallback;
	let mut best_dot = -1.0;
	for step in 0..STEPS {
		let roll = -std::f32::consts::PI + std::f32::consts::TAU * step as f32 / STEPS as f32;
		let dot = lower_arm_direction(rig, side, reach, roll).dot(reach.lower_along);
		if dot > best_dot {
			best_dot = dot;
			best = roll;
		}
	}
	best
}

fn lower_arm_direction(rig: &HumanoidV0Rig, side: Side, reach: TwoBoneAim, roll: f32) -> Vec3 {
	let mut trial = rig.clone();
	let mut arm = trial.arm_pose(side);
	arm.humerus = trial.humerus_along_with_roll(side, reach.upper_along, roll);
	trial.pose_arm(arm);
	let mut arm = trial.arm_pose(side);
	arm.forearm = trial.articulate_on_rig(arm.forearm, 0.0, reach.flex);
	trial.pose_arm(arm.clone());
	let parent = trial.parent_world_rotation(&arm.forearm.name);
	(parent * arm.forearm.transform.rotation * BONE_LENGTH_AXIS).normalize_or(Vec3::Z)
}

fn reset_arm_to_rest(
	rig: &mut HumanoidV0Rig,
	map: &BoneMap,
	bones: &Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<PlayerVisual>)>,
	side: Side,
) {
	let mut arm = rig.arm(side);
	for pose in [&mut arm.shoulder, &mut arm.humerus, &mut arm.forearm] {
		let Some(&entity) = map.by_name.get(pose.name.as_str()) else {
			continue;
		};
		let Ok((bone, _)) = bones.get(entity) else {
			continue;
		};
		pose.transform = bone.rest;
		pose.swing = 0.0;
		pose.flex = 0.0;
		pose.twist = 0.0;
	}
	rig.pose_arm(arm);
}

fn write_hold_bones(
	rig: &HumanoidV0Rig,
	map: &BoneMap,
	bones: &mut Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<PlayerVisual>)>,
) {
	let spine = rig.spine();
	for name in
		[spine.lumbar.name.as_str(), spine.midback.name.as_str(), spine.upper_back.name.as_str()]
	{
		write_bone(rig, map, bones, name);
	}
	for side in [Side::Left, Side::Right] {
		let arm = rig.arm(side);
		for name in
			[arm.shoulder.name.as_str(), arm.humerus.name.as_str(), arm.forearm.name.as_str()]
		{
			write_bone(rig, map, bones, name);
		}
	}
}

fn write_bone(
	rig: &HumanoidV0Rig,
	map: &BoneMap,
	bones: &mut Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<PlayerVisual>)>,
	name: &str,
) {
	let Some(&entity) = map.by_name.get(name) else {
		return;
	};
	let Some(pose) = rig.pose().get(&Name::from(name)) else {
		return;
	};
	let Ok((_, mut transform)) = bones.get_mut(entity) else {
		return;
	};
	*transform = pose.transform;
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

	#[test]
	fn sync_hands_queries_are_disjoint() -> Result<(), bevy::ecs::system::RunSystemError> {
		let mut world = World::new();
		world.run_system_once(sync_hands_to_firearm)?;
		Ok(())
	}

	#[test]
	fn roll_search_points_forearm_toward_target() -> Result<(), &'static str> {
		let rig = HumanoidV0Rig::imported();
		let reach = TwoBoneAim::reach(Vec3::new(0.15, 0.0, 0.8), RIGHT_POLE, 0.5, 0.5)
			.ok_or("missing reach")?;
		let roll = best_humerus_roll(&rig, Side::Right, reach, -HUMERUS_ROLL);
		let lower = lower_arm_direction(&rig, Side::Right, reach, roll);
		assert!(lower.dot(reach.lower_along) > 0.95, "{lower:?} vs {reach:?}");
		Ok(())
	}

	#[test]
	fn firing_torso_turns_right_shoulder_back() -> Result<(), &'static str> {
		let mut rig = HumanoidV0Rig::imported();
		pose_firing_torso(&mut rig);
		let spine = rig.spine_pose();
		assert!(spine.lumbar.swing < 0.0);
		assert!(spine.midback.swing.abs() > spine.lumbar.swing.abs());
		assert!(spine.upper_back.swing.abs() > spine.midback.swing.abs());
		Ok(())
	}
}
