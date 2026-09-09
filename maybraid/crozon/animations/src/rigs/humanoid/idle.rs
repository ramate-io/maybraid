use std::f32::consts::TAU;

use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::animations::Idle;
use crate::rigs::humanoid::apply::{apply_arm, apply_neck_twisted};
use crate::Animation;

impl<R: HumanoidRig> Animation<R> for Idle {
	fn apply_for(&self, rig: &mut R, progress: f32) {
		let arm = (TAU * (progress * Idle::ARM_FREQ)).sin();
		let yaw = Idle::look_wave(progress, Idle::NECK_YAW_FREQ, 0.15);
		let nod = Idle::look_wave(progress, Idle::NECK_PITCH_FREQ, 0.41);
		let hip = (TAU * (progress * Idle::HIP_FREQ + 0.4)).sin();

		apply_idle_arm(rig, Side::Left, arm, self);
		apply_idle_arm(rig, Side::Right, -arm, self);
		apply_idle_neck(rig, yaw, nod, self);
		apply_idle_hips(rig, hip, self);
	}
}

fn apply_idle_arm<R: HumanoidRig>(rig: &mut R, side: Side, swing: f32, idle: &Idle) {
	let flex_sign = -rig.forearm_flex_sign(side);
	let hang = -side.sign() * idle.arm_hang;
	let shoulder = swing * idle.arm_sway;
	apply_arm(
		rig,
		side,
		shoulder,
		0.0,
		shoulder * 0.5,
		hang,
		flex_sign * (idle.elbow_hang + 0.03 * swing.abs()),
	);
}

fn apply_idle_neck<R: HumanoidRig>(rig: &mut R, yaw: f32, nod: f32, idle: &Idle) {
	let yaw = yaw * idle.neck_roll;
	let nod = nod * idle.neck_roll;
	apply_neck_twisted(rig, yaw * 0.65, 0.0, nod * 0.35, yaw * 0.35, 0.0, nod * 0.65);
}

fn apply_idle_hips<R: HumanoidRig>(rig: &mut R, shift: f32, idle: &Idle) {
	let amount = shift * idle.hip_shift;
	for side in [Side::Left, Side::Right] {
		let mut leg = rig.leg_pose(side);
		leg.pelvis = rig.articulate_on_rig(leg.pelvis, 0.0, amount * side.sign());
		rig.pose_leg(leg);
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use crozon_rigs::{BonePose, Name};

	use super::*;
	use crate::Animation;

	fn seeded_rig() -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::imported();
		for bone in [
			"shoulder.L",
			"shoulder.R",
			"humerus.L",
			"humerus.R",
			"forearm.L",
			"forearm.R",
			"lower_neck",
			"upper_neck",
			"pelvis.L",
			"pelvis.R",
		] {
			rig.pose_mut()
				.insert(BonePose::new(Name::from(bone), bevy::prelude::Transform::IDENTITY));
		}
		rig
	}

	#[test]
	fn idle_hangs_arms_off_the_t_pose() {
		let mut rig = seeded_rig();
		Idle::default().apply(&mut rig, 0.0);

		let left = rig.pose().get(&Name::from("humerus.L")).expect("left");
		let right = rig.pose().get(&Name::from("humerus.R")).expect("right");
		assert!(left.flex.abs() > 1.0);
		assert!(right.flex.abs() > 1.0);
		assert!((left.flex + right.flex).abs() < 1e-5);
	}

	#[test]
	fn idle_sways_opposite_arms() {
		let mut rig = seeded_rig();
		Idle::default().apply(&mut rig, 0.25);

		let left = rig.pose().get(&Name::from("shoulder.L")).expect("left");
		let right = rig.pose().get(&Name::from("shoulder.R")).expect("right");
		assert!(left.swing.abs() > 0.0);
		assert!((left.swing + right.swing).abs() < 1e-5);
		assert!(left.swing.abs() < 0.1);
	}

	#[test]
	fn idle_rolls_neck() {
		let mut rig = seeded_rig();
		// Peak of the yaw glance: progress * 0.35 + 0.15 = 0.25.
		Idle::default().apply(&mut rig, 0.1 / Idle::NECK_YAW_FREQ);

		let lower = rig.pose().get(&Name::from("lower_neck")).expect("lower");
		assert!(lower.swing.abs() > 0.0);
		assert!(lower.swing.abs() < 0.1);
	}

	#[test]
	fn idle_neck_is_continuous_across_unit_progress() {
		let idle = Idle::default();
		let mut before = seeded_rig();
		let mut after = seeded_rig();
		idle.apply(&mut before, 0.999);
		idle.apply(&mut after, 1.001);

		let a = before.pose().get(&Name::from("lower_neck")).expect("before");
		let b = after.pose().get(&Name::from("lower_neck")).expect("after");
		assert!((a.swing - b.swing).abs() < 0.01);
		assert!((a.twist - b.twist).abs() < 0.01);
	}

	#[test]
	fn idle_hip_shift_is_lighter_than_arms() {
		let mut rig = seeded_rig();
		Idle::default().apply(&mut rig, 0.25);

		let hip = rig.pose().get(&Name::from("pelvis.L")).expect("pelvis");
		let shoulder = rig.pose().get(&Name::from("shoulder.L")).expect("shoulder");
		assert!(hip.flex.abs() > 0.0);
		assert!(hip.flex.abs() < shoulder.swing.abs());
	}

	#[test]
	fn idle_does_not_lean_the_spine() {
		let mut rig = seeded_rig();
		rig.pose_mut()
			.insert(BonePose::new(Name::from("root"), bevy::prelude::Transform::IDENTITY));
		Idle::default().apply(&mut rig, 0.25);

		let root = rig.pose().get(&Name::from("root")).expect("root");
		assert!(root.swing.abs() < 1e-5);
		assert!(root.flex.abs() < 1e-5);
	}

	#[test]
	fn idle_phase_offset_changes_pose() {
		let idle = Idle::default();
		let mut a = seeded_rig();
		let mut b = seeded_rig();
		idle.apply(&mut a, 0.1);
		idle.apply(&mut b, 0.1 + Idle::phase_from_entity_bits(7));

		let left_a = a.pose().get(&Name::from("shoulder.L")).expect("a").swing;
		let left_b = b.pose().get(&Name::from("shoulder.L")).expect("b").swing;
		assert_ne!(left_a, left_b);
	}
}
