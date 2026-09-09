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
		let scratch = Idle::scratch_weight(progress);
		let scratch_side = Idle::scratch_side(progress);

		// Same local swing on both arms — right-side axes are already mirrored.
		apply_idle_arm(rig, Side::Left, arm, scratch, scratch_side, progress, self);
		apply_idle_arm(rig, Side::Right, arm, scratch, scratch_side, progress, self);
		apply_idle_neck(rig, yaw, nod, scratch, scratch_side, self);
		apply_idle_hips(rig, hip, self);
	}
}

fn apply_idle_arm<R: HumanoidRig>(
	rig: &mut R,
	side: Side,
	swing: f32,
	scratch: f32,
	scratch_side: Side,
	progress: f32,
	idle: &Idle,
) {
	let flex_sign = -rig.forearm_flex_sign(side);
	let hang = -side.sign() * idle.arm_hang;
	let shoulder = swing * idle.arm_sway;
	let elbow = flex_sign * (idle.elbow_hang + 0.03 * swing.abs());
	if scratch > 1e-4 && side == scratch_side {
		apply_scratch_arm(rig, side, hang, shoulder, elbow, scratch, progress, flex_sign);
	} else {
		apply_arm(rig, side, shoulder, 0.0, shoulder * 0.5, hang, elbow);
	}
}

fn apply_scratch_arm<R: HumanoidRig>(
	rig: &mut R,
	side: Side,
	hang: f32,
	idle_shoulder: f32,
	idle_elbow: f32,
	scratch: f32,
	progress: f32,
	flex_sign: f32,
) {
	let wiggle = (TAU * progress * 6.0).sin() * 0.1 * scratch;
	let inward = -side.sign() * 0.45;
	apply_arm(
		rig,
		side,
		idle_shoulder * (1.0 - scratch) + inward * scratch,
		0.55 * scratch,
		idle_shoulder * 0.5 * (1.0 - scratch) + inward * 0.35 * scratch,
		hang * (1.0 - 0.85 * scratch),
		idle_elbow * (1.0 - scratch) + flex_sign * (1.25 * scratch + wiggle),
	);
}

fn apply_idle_neck<R: HumanoidRig>(
	rig: &mut R,
	yaw: f32,
	nod: f32,
	scratch: f32,
	scratch_side: Side,
	idle: &Idle,
) {
	let yaw = yaw * idle.neck_roll + scratch * 0.1 * scratch_side.sign();
	let nod = nod * idle.neck_roll + scratch * 0.04;
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
	fn idle_arm_sway_uses_the_same_local_sign() {
		let mut rig = seeded_rig();
		Idle::default().apply(&mut rig, Idle::SCRATCH_DURATION + 0.2);

		let left = rig.pose().get(&Name::from("shoulder.L")).expect("left");
		let right = rig.pose().get(&Name::from("shoulder.R")).expect("right");
		assert!(left.swing.abs() > 0.0);
		assert!((left.swing - right.swing).abs() < 1e-5);
		assert!(left.swing.abs() < 0.1);
	}

	#[test]
	fn idle_rolls_neck() {
		let mut rig = seeded_rig();
		// Next yaw peak after the first scratch burst.
		Idle::default().apply(&mut rig, 1.1 / Idle::NECK_YAW_FREQ);

		let lower = rig.pose().get(&Name::from("lower_neck")).expect("lower");
		assert!(lower.swing.abs() > 0.08);
		assert!(lower.swing.abs() < 0.25);
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
		assert!((a.swing - b.swing).abs() < 0.02);
		assert!((a.twist - b.twist).abs() < 0.02);
	}

	#[test]
	fn idle_scratch_raises_one_hand() {
		let mut idle_pose = seeded_rig();
		let mut scratch_pose = seeded_rig();
		let quiet = Idle::SCRATCH_DURATION + 0.2;
		let peak = Idle::SCRATCH_DURATION * 0.5;
		Idle::default().apply(&mut idle_pose, quiet);
		Idle::default().apply(&mut scratch_pose, peak);

		let side = Idle::scratch_side(peak);
		let forearm = match side {
			Side::Right => "forearm.R",
			Side::Left => "forearm.L",
		};
		let rest = idle_pose.pose().get(&Name::from(forearm)).expect("rest").flex.abs();
		let scratch = scratch_pose.pose().get(&Name::from(forearm)).expect("scratch").flex.abs();
		assert!(scratch > rest + 0.4);
		assert_eq!(Idle::scratch_weight(quiet), 0.0);
	}

	#[test]
	fn idle_hip_shift_is_lighter_than_arms() {
		let mut rig = seeded_rig();
		Idle::default().apply(&mut rig, Idle::SCRATCH_DURATION + 0.2);

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
