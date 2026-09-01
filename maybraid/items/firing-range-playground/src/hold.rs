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

const RIGHT_ELBOW: f32 = 1.0;
const LEFT_ELBOW: f32 = 1.3;
const HUMERUS_ROLL: f32 = std::f32::consts::FRAC_PI_2;
const RIGHT_POLE: Vec3 = Vec3::new(0.25, -1.0, -0.1);
const LEFT_POLE: Vec3 = Vec3::new(-0.4, -1.0, 0.05);
const RIGHT_TARGET: Vec3 = Vec3::new(0.12, 0.0, 1.0);
const LEFT_TARGET: Vec3 = Vec3::new(-0.25, 0.0, 1.0);

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
		reset_arm_to_rest(&mut rig, map, &bones, Side::Right);
		reset_arm_to_rest(&mut rig, map, &bones, Side::Left);
		let right_target =
			target_from(body_rot, shoulder_world(map, &globals, "shoulder.R"), trigger)
				.unwrap_or(RIGHT_TARGET);
		let left_target = target_from(body_rot, shoulder_world(map, &globals, "shoulder.L"), grip)
			.unwrap_or(LEFT_TARGET);
		let right = two_bone_uppercut(right_target, RIGHT_POLE, RIGHT_ELBOW)
			.unwrap_or(TwoBoneReach::fallback(Vec3::new(0.12, -0.45, 0.88), RIGHT_ELBOW));
		let left = two_bone_uppercut(left_target, LEFT_POLE, LEFT_ELBOW)
			.unwrap_or(TwoBoneReach::fallback(Vec3::new(-0.2, -0.65, 0.72), LEFT_ELBOW));
		pose_arm(&mut rig, Side::Right, right);
		pose_arm(&mut rig, Side::Left, left);
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

fn target_from(body_rot: Quat, from: Option<Vec3>, to: Option<Vec3>) -> Option<Vec3> {
	let dir = to? - from?;
	if dir.length_squared() < 1e-6 {
		return None;
	}
	let target = body_rot.inverse() * dir;
	(target.z > 0.0).then_some(target)
}

#[derive(Debug, Clone, Copy)]
struct TwoBoneReach {
	upper_along: Vec3,
	lower_along: Vec3,
	bend: f32,
}

impl TwoBoneReach {
	fn fallback(upper_along: Vec3, bend: f32) -> Self {
		Self { upper_along: upper_along.normalize_or(Vec3::Z), lower_along: Vec3::Z, bend }
	}
}

/// Equal-segment analytic reach with a pole below the target.
///
/// `bend` is the elbow angle: zero is straight. The segment directions put
/// the elbow on the pole side while preserving the target line.
fn two_bone_uppercut(target: Vec3, pole: Vec3, bend: f32) -> Option<TwoBoneReach> {
	let distance = target.length();
	if distance <= 1e-5 {
		return None;
	}
	let toward = target / distance;
	let pole = pole - toward * pole.dot(toward);
	if pole.length_squared() <= 1e-6 {
		return None;
	}
	let half = distance * 0.5;
	let elbow_height = half * (bend * 0.5).tan();
	let elbow = toward * half + pole.normalize() * elbow_height;
	Some(TwoBoneReach {
		upper_along: elbow.normalize_or(Vec3::Z),
		lower_along: (target - elbow).normalize_or(Vec3::Z),
		bend,
	})
}

fn pose_arm(rig: &mut HumanoidV0Rig, side: Side, reach: TwoBoneReach) {
	let roll = best_humerus_roll(rig, side, reach, HUMERUS_ROLL * side.sign());
	let mut arm = rig.arm_pose(side);
	arm.humerus = rig.humerus_along_with_roll(side, reach.upper_along, roll);
	rig.pose_arm(arm);
	let mut arm = rig.arm_pose(side);
	arm.forearm = rig.articulate_on_rig(arm.forearm, 0.0, reach.bend);
	rig.pose_arm(arm);
}

/// Pick the long-axis roll whose elbow flex points the forearm at the target.
///
/// This tiny one-dimensional IK solve handles the mirrored forearm hinge axes
/// without baking another left/right sign rule into the playground.
fn best_humerus_roll(rig: &HumanoidV0Rig, side: Side, reach: TwoBoneReach, fallback: f32) -> f32 {
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

fn lower_arm_direction(rig: &HumanoidV0Rig, side: Side, reach: TwoBoneReach, roll: f32) -> Vec3 {
	let mut trial = rig.clone();
	let mut arm = trial.arm_pose(side);
	arm.humerus = trial.humerus_along_with_roll(side, reach.upper_along, roll);
	trial.pose_arm(arm);
	let mut arm = trial.arm_pose(side);
	arm.forearm = trial.articulate_on_rig(arm.forearm, 0.0, reach.bend);
	trial.pose_arm(arm.clone());
	let parent = trial.parent_world_rotation(&arm.forearm.name);
	(parent * arm.forearm.transform.rotation * Vec3::Y).normalize_or(Vec3::Z)
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

	#[test]
	fn uppercut_solver_puts_elbow_below_target() {
		let reach = two_bone_uppercut(Vec3::new(-0.2, 0.1, 0.8), Vec3::NEG_Y, 1.2).expect("reach");
		assert!(reach.upper_along.y < 0.0, "{:?}", reach.upper_along);
		assert!(reach.upper_along.z > 0.0, "{:?}", reach.upper_along);
	}

	#[test]
	fn tighter_uppercut_drops_upper_arm_further() {
		let target = Vec3::Z;
		let open = two_bone_uppercut(target, Vec3::NEG_Y, 0.5).expect("open");
		let tucked = two_bone_uppercut(target, Vec3::NEG_Y, 1.3).expect("tucked");
		assert!(
			tucked.upper_along.y < open.upper_along.y,
			"{:?} vs {:?}",
			tucked.upper_along,
			open.upper_along
		);
	}

	#[test]
	fn roll_search_points_forearm_toward_target() {
		let rig = HumanoidV0Rig::imported();
		let reach = two_bone_uppercut(Vec3::new(0.15, 0.0, 0.8), RIGHT_POLE, 1.0).expect("reach");
		let roll = best_humerus_roll(&rig, Side::Right, reach, -HUMERUS_ROLL);
		let lower = lower_arm_direction(&rig, Side::Right, reach, roll);
		assert!(lower.dot(reach.lower_along) > 0.95, "{lower:?} vs {reach:?}");
	}
}
