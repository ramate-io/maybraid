//! After walk/run, aim both arms at the held firearm's hand landmarks.

use bevy::prelude::*;
use crozon_characters::{
	AnimBone, AnimMailbox, AnimateBones, BoneMap, CharacterMembers, CharacterRoot, SuspendAnimation,
};
use crozon_rigs::articulation::{TwoBoneAim, BONE_LENGTH_AXIS};
use crozon_rigs::humanoid::HumanoidRig;
use crozon_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use crozon_rigs::{Name, Side};
use firearms::{FirearmMembers, FirearmRoot};

use crate::pose::HeldFirearm;
use crate::FirearmUser;

/// Body-rig marker: after locomotion, apply the firearm hold.
#[derive(Component, Clone, Copy, Default)]
pub struct HoldingArms;

/// Point the trigger hand at `trigger_point` and the support hand at the grip socket.
pub fn sync_hands_to_firearm(
	users: Query<&FirearmUser>,
	visuals: Query<
		(&GlobalTransform, &CharacterMembers, &ChildOf),
		(With<CharacterRoot>, Without<AnimBone>),
	>,
	guns: Query<
		(&FirearmMembers, &Transform, &GlobalTransform),
		(With<HeldFirearm>, With<FirearmRoot>, Without<AnimBone>),
	>,
	gun_maps: Query<&BoneMap, Without<HoldingArms>>,
	globals: Query<&GlobalTransform>,
	mut rigs: Query<
		(&mut HumanoidV0Rig, &BoneMap, &AnimMailbox),
		(With<HoldingArms>, With<AnimateBones>, Without<SuspendAnimation>),
	>,
	mut bones: Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<CharacterRoot>)>,
) {
	for (visual, members, child_of) in &visuals {
		let Ok(user) = users.get(child_of.parent()) else {
			continue;
		};
		let body_rot = visual.rotation();
		let settings = user.settings;
		let trigger = gun_landmark(user.held, &guns, &gun_maps, &globals, "trigger_point");
		let grip = gun_landmark(user.held, &guns, &gun_maps, &globals, settings.grip_socket);
		let (Some(trigger), Some(grip)) = (trigger, grip) else {
			continue;
		};

		for member in members.iter() {
			let Ok((mut rig, map, mailbox)) = rigs.get_mut(member) else {
				continue;
			};
			if mailbox.output.is_empty() {
				continue;
			}
			rig.pose.clone_from(&mailbox.output);
			pose_firing_torso(&mut rig, settings.firing_torso_yaw);
			reset_arm_to_rest(&mut rig, map, &bones, Side::Right);
			reset_arm_to_rest(&mut rig, map, &bones, Side::Left);
			let right = arm_reach(
				&rig,
				Side::Right,
				target_from(body_rot, bone_world(map, &globals, "humerus.R"), trigger),
				settings.right_pole,
				1.0,
			);
			let left = arm_reach(
				&rig,
				Side::Left,
				target_from(body_rot, bone_world(map, &globals, "humerus.L"), grip),
				settings.left_pole,
				settings.left_reach_stretch,
			);
			if let Some(right) = right {
				pose_arm(&mut rig, Side::Right, right, 1.0, settings.humerus_roll);
			}
			if let Some(left) = left {
				pose_arm(
					&mut rig,
					Side::Left,
					left,
					settings.left_reach_stretch,
					settings.humerus_roll,
				);
			}
			write_hold_bones(&rig, map, &mut bones);
		}
	}
}

fn gun_landmark(
	held: Entity,
	guns: &Query<
		(&FirearmMembers, &Transform, &GlobalTransform),
		(With<HeldFirearm>, With<FirearmRoot>, Without<AnimBone>),
	>,
	maps: &Query<&BoneMap, Without<HoldingArms>>,
	globals: &Query<&GlobalTransform>,
	name: &str,
) -> Option<Vec3> {
	let (members, current_root, previous_root) = guns.get(held).ok()?;
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
	stretch: f32,
) -> Option<TwoBoneAim> {
	let arm = rig.arm_pose(side);
	let upper_length = arm.forearm.transform.translation.length() * stretch;
	let target = target?;
	TwoBoneAim::reach(target, pole, upper_length, upper_length).or_else(|| {
		TwoBoneAim::reach(target, Vec3::X * side.sign() + Vec3::Z, upper_length, upper_length)
	})
}

fn pose_arm(
	rig: &mut HumanoidV0Rig,
	side: Side,
	reach: TwoBoneAim,
	stretch: f32,
	humerus_roll: f32,
) {
	let roll = humerus_roll_for_reach(rig, side, reach, humerus_roll * side.sign());
	let mut arm = rig.arm_pose(side);
	arm.humerus = rig.humerus_along_with_roll(side, reach.upper_along, roll);
	arm.humerus.transform.translation *= stretch;
	rig.pose_arm(arm);
	let mut arm = rig.arm_pose(side);
	arm.forearm = rig.articulate_on_rig(arm.forearm, 0.0, reach.flex);
	arm.forearm.transform.translation *= stretch;
	rig.pose_arm(arm);
}

fn pose_firing_torso(rig: &mut HumanoidV0Rig, firing_torso_yaw: f32) {
	let mut spine = rig.spine_pose();
	// Keep the hips nearly square and blade the shoulder girdle from the chest.
	spine.lumbar = rig.articulate_on_rig(spine.lumbar, firing_torso_yaw * 0.05, 0.0);
	spine.midback = rig.articulate_on_rig(spine.midback, firing_torso_yaw * 0.20, 0.0);
	spine.upper_back = rig.articulate_on_rig(spine.upper_back, firing_torso_yaw * 0.75, 0.0);
	rig.pose_spine(spine);
}

/// Solve the long-axis roll that rotates the authored forearm hinge into the
/// elbow plane selected by [`TwoBoneAim`].
fn humerus_roll_for_reach(
	rig: &HumanoidV0Rig,
	side: Side,
	reach: TwoBoneAim,
	fallback: f32,
) -> f32 {
	let arm = rig.arm_pose(side);
	let humerus = rig.humerus_along_with_roll(side, reach.upper_along, 0.0);
	let forearm = rig.articulate_on_rig(arm.forearm, 0.0, reach.flex);
	let humerus_world = rig.parent_world_rotation(&humerus.name) * humerus.transform.rotation;
	let zero_roll_lower = forearm.transform.rotation * BONE_LENGTH_AXIS;
	let desired_lower = humerus_world.inverse() * reach.lower_along;
	signed_angle_about_axis(zero_roll_lower, desired_lower, BONE_LENGTH_AXIS).unwrap_or(fallback)
}

#[cfg(test)]
fn lower_arm_direction(rig: &HumanoidV0Rig, side: Side, reach: TwoBoneAim, roll: f32) -> Vec3 {
	let arm = rig.arm_pose(side);
	let humerus = rig.humerus_along_with_roll(side, reach.upper_along, roll);
	let forearm = rig.articulate_on_rig(arm.forearm, 0.0, reach.flex);
	let forearm_parent = rig.parent_world_rotation(&humerus.name) * humerus.transform.rotation;
	(forearm_parent * forearm.transform.rotation * BONE_LENGTH_AXIS).normalize_or(Vec3::Z)
}

fn signed_angle_about_axis(from: Vec3, to: Vec3, axis: Vec3) -> Option<f32> {
	let axis = axis.try_normalize()?;
	let from = (from - axis * from.dot(axis)).try_normalize()?;
	let to = (to - axis * to.dot(axis)).try_normalize()?;
	Some(axis.dot(from.cross(to)).atan2(from.dot(to)))
}

fn reset_arm_to_rest(
	rig: &mut HumanoidV0Rig,
	map: &BoneMap,
	bones: &Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<CharacterRoot>)>,
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
	bones: &mut Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<CharacterRoot>)>,
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
	bones: &mut Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<CharacterRoot>)>,
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
	use crate::FirearmUserSettings;
	use bevy::ecs::system::RunSystemOnce;

	fn settings() -> FirearmUserSettings {
		FirearmUserSettings::default()
	}

	#[test]
	fn sync_hands_queries_are_disjoint() -> Result<(), bevy::ecs::system::RunSystemError> {
		let mut world = World::new();
		world.run_system_once(sync_hands_to_firearm)?;
		Ok(())
	}

	#[test]
	fn analytical_roll_points_forearm_toward_target() -> Result<(), &'static str> {
		let s = settings();
		let rig = HumanoidV0Rig::imported();
		let reach = TwoBoneAim::reach(Vec3::new(0.15, 0.0, 0.8), s.right_pole, 0.5, 0.5)
			.ok_or("missing reach")?;
		let roll = humerus_roll_for_reach(&rig, Side::Right, reach, -s.humerus_roll);
		let lower = lower_arm_direction(&rig, Side::Right, reach, roll);
		assert!(lower.dot(reach.lower_along) > 0.999, "{lower:?} vs {reach:?}");
		Ok(())
	}

	#[test]
	fn analytical_roll_handles_both_mirrored_arms() -> Result<(), &'static str> {
		let s = settings();
		let rig = HumanoidV0Rig::imported();
		for (side, target, pole, fallback) in [
			(Side::Left, Vec3::new(0.2, -0.15, 0.55), s.left_pole, s.humerus_roll),
			(Side::Right, Vec3::new(-0.2, -0.1, 0.7), s.right_pole, -s.humerus_roll),
		] {
			let reach = TwoBoneAim::reach(target, pole, 0.5, 0.5).ok_or("missing reach")?;
			let roll = humerus_roll_for_reach(&rig, side, reach, fallback);
			let lower = lower_arm_direction(&rig, side, reach, roll);
			assert!(lower.dot(reach.lower_along) > 0.999, "{side:?}: {lower:?} vs {reach:?}");
		}
		Ok(())
	}

	#[test]
	fn signed_roll_uses_fallback_for_a_straight_hinge() {
		assert!(signed_angle_about_axis(Vec3::Y, Vec3::Y, Vec3::Y).is_none());
	}

	#[test]
	fn right_elbow_pole_wings_out_and_down() -> Result<(), &'static str> {
		let reach = TwoBoneAim::reach(Vec3::Z * 0.7, settings().right_pole, 0.5, 0.5)
			.ok_or("missing reach")?;
		assert!(reach.upper_along.x < 0.0, "{reach:?}");
		assert!(reach.upper_along.y < 0.0, "{reach:?}");
		assert!((reach.upper_along.x.abs() - reach.upper_along.y.abs()).abs() < 1e-4, "{reach:?}");
		Ok(())
	}

	#[test]
	fn support_hand_targets_grip_socket() {
		let socket = settings().grip_socket;
		assert_eq!(socket, "grip");
		assert_ne!(socket, "grip_point");
	}

	#[test]
	fn support_arm_stretch_reaches_past_equal_segments() -> Result<(), &'static str> {
		let s = settings();
		let far = Vec3::Z * 1.1;
		let short = TwoBoneAim::reach(far, s.left_pole, 0.5, 0.5).ok_or("missing short reach")?;
		let stretched = TwoBoneAim::reach(
			far,
			s.left_pole,
			0.5 * s.left_reach_stretch,
			0.5 * s.left_reach_stretch,
		)
		.ok_or("missing stretched reach")?;
		assert!(stretched.flex > short.flex, "short {short:?} stretched {stretched:?}");
		Ok(())
	}

	#[test]
	fn left_humerus_swings_forward_to_a_close_grip() -> Result<(), &'static str> {
		// Grip sits in front of the left shoulder, inside rest length.
		let target = Vec3::new(0.2, -0.15, 0.55);
		let reach =
			TwoBoneAim::reach(target, settings().left_pole, 0.5, 0.5).ok_or("missing reach")?;
		assert!(
			reach.upper_along.z > 0.12,
			"humerus should still come forward, got {:?}",
			reach.upper_along
		);
		assert!(
			reach.upper_along.x > 0.15,
			"left elbow should wing out a bit (+X), got {:?}",
			reach.upper_along
		);
		assert!(reach.upper_along.y < -0.25, "elbow should tuck down, got {:?}", reach.upper_along);
		assert!(
			reach.upper_along.y > -0.7,
			"tuck should not hang the humerus on the ribs, got {:?}",
			reach.upper_along
		);
		Ok(())
	}

	#[test]
	fn pose_arm_aims_humerus_length_along_reach() -> Result<(), &'static str> {
		let s = settings();
		let mut rig = HumanoidV0Rig::imported();
		let reach = TwoBoneAim::reach(Vec3::new(0.2, -0.15, 0.55), s.left_pole, 0.5, 0.5)
			.ok_or("missing reach")?;
		pose_arm(&mut rig, Side::Left, reach, 1.0, s.humerus_roll);
		let humerus = rig.arm_pose(Side::Left).humerus;
		let along = (humerus.transform.rotation * BONE_LENGTH_AXIS).normalize_or(Vec3::Y);
		assert!(
			along.dot(reach.upper_along) > 0.97,
			"expected aim along {:?}, got {along:?}",
			reach.upper_along
		);
		Ok(())
	}

	#[test]
	fn downward_pole_would_hang_the_left_humerus() -> Result<(), &'static str> {
		let target = Vec3::new(0.2, -0.15, 0.55);
		let hung = TwoBoneAim::reach(target, Vec3::new(-0.4, -1.0, 0.05), 0.5, 0.5)
			.ok_or("missing hung reach")?;
		assert!(
			hung.upper_along.y < -0.5,
			"old pole is the hang we are leaving, got {:?}",
			hung.upper_along
		);
		Ok(())
	}

	#[test]
	fn firing_torso_turns_right_shoulder_back() -> Result<(), &'static str> {
		let mut rig = HumanoidV0Rig::imported();
		pose_firing_torso(&mut rig, settings().firing_torso_yaw);
		let spine = rig.spine_pose();
		assert!(spine.lumbar.swing < 0.0);
		assert!(spine.midback.swing.abs() > spine.lumbar.swing.abs());
		assert!(spine.upper_back.swing.abs() > spine.midback.swing.abs());
		Ok(())
	}
}
