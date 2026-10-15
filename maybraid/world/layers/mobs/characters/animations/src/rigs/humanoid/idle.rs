use std::f32::consts::TAU;

use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Idle;
use crate::rigs::humanoid::write_masks::{debug_assert_pose_within_mask, idle_write_mask};
use crate::Animation;

impl Idle {
	/// Authored semantic pose at unwrapped `progress`. Rest is applied later by the rig.
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		let arm = (TAU * (progress * Idle::ARM_FREQ)).sin();
		let yaw = Idle::look_wave(progress, Idle::NECK_YAW_FREQ, 0.15);
		let nod = Idle::look_wave(progress, Idle::NECK_PITCH_FREQ, 0.41);
		let hip = (TAU * (progress * Idle::HIP_FREQ + Idle::HIP_WEIGHT_PHASE)).sin();
		let scratch = Idle::scratch_weight(progress);
		let scratch_side = Idle::scratch_side(progress);

		// Same forward swing on both arms. Opposite hang signs are a rest bias.
		apply_idle_arm(&mut pose, Side::Left, arm, scratch, scratch_side, progress, self);
		apply_idle_arm(&mut pose, Side::Right, arm, scratch, scratch_side, progress, self);
		apply_idle_neck(&mut pose, yaw, nod, scratch, scratch_side, self);
		apply_idle_hips(&mut pose, hip, self);
		pose
	}
}

impl Animation<HumanoidV0Rig> for Idle {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let pose = self.sample_pose(progress);
		debug_assert_pose_within_mask(&pose, idle_write_mask(), "idle");
		rig.apply_masked_pose(&pose, idle_write_mask());
	}
}

fn apply_idle_arm(
	pose: &mut HumanoidPose,
	side: Side,
	swing: f32,
	scratch: f32,
	scratch_side: Side,
	progress: f32,
	idle: &Idle,
) {
	// Both elbows share one flexion sign.
	let flex_sign = -1.0;
	let hang = -side.sign() * idle.arm_hang;
	let shoulder = swing * idle.arm_sway;
	let elbow = flex_sign * (idle.elbow_hang + 0.03 * swing.abs());
	if scratch > 1e-4 && side == scratch_side {
		apply_scratch_arm(pose, side, hang, shoulder, elbow, scratch, progress, flex_sign);
	} else {
		pose.apply_arm(side, shoulder, 0.0, shoulder * 0.5, hang, elbow);
	}
}

fn apply_scratch_arm(
	pose: &mut HumanoidPose,
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
	pose.apply_arm(
		side,
		idle_shoulder * (1.0 - scratch) + inward * scratch,
		0.55 * scratch,
		idle_shoulder * 0.5 * (1.0 - scratch) + inward * 0.35 * scratch,
		hang * (1.0 - 0.85 * scratch),
		idle_elbow * (1.0 - scratch) + flex_sign * (1.25 * scratch + wiggle),
	);
}

fn apply_idle_neck(
	pose: &mut HumanoidPose,
	yaw: f32,
	nod: f32,
	scratch: f32,
	scratch_side: Side,
	idle: &Idle,
) {
	let yaw = yaw * idle.neck_roll + scratch * 0.1 * scratch_side.sign();
	let nod = nod * idle.neck_roll + scratch * 0.04;
	pose.apply_neck_twisted(yaw * 0.65, 0.0, nod * 0.35, yaw * 0.35, 0.0, nod * 0.65);
}

fn apply_idle_hips(pose: &mut HumanoidPose, shift: f32, idle: &Idle) {
	let amount = shift * idle.hip_shift;
	for side in [Side::Left, Side::Right] {
		pose.leg_mut(side).pelvis_lateral = amount * side.sign();
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::Side;

	use super::*;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn bone_endpoint(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		let id = rig.binding.definition.id(name).expect(name);
		let bone_len = rig.binding.effective_rest.get(id).expect("rest").translation.length();
		rig.character_point(name) + rig.character_length(name) * bone_len
	}

	fn pose_at(progress: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut rig, progress);
		rig
	}

	fn foot_end(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		let name = match side {
			Side::Left => "shin.L",
			Side::Right => "shin.R",
		};
		bone_endpoint(rig, name)
	}

	#[test]
	fn idle_hangs_arms_off_the_t_pose() {
		let mut rig = HumanoidV0Rig::imported();
		Idle::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("humerus.L") > 0.2, "left hang leaves rest");
		assert!(rig.posed_angle("humerus.R") > 0.2, "right hang leaves rest");
		assert!(
			(rig.posed_angle("humerus.L") - rig.posed_angle("humerus.R")).abs() < 1e-3,
			"opposite hang bias, same amplitude"
		);
	}

	#[test]
	fn idle_arm_sway_uses_the_same_forward_sign() {
		let mut rig = HumanoidV0Rig::imported();
		Idle::default().apply(&mut rig, Idle::SCRATCH_DURATION + 0.2);

		assert!(rig.posed_angle("shoulder.L") > 0.0, "sway leaves rest");
		assert!(
			(rig.posed_angle("shoulder.L") - rig.posed_angle("shoulder.R")).abs() < 1e-3,
			"same forward sign"
		);
		assert!(rig.posed_angle("shoulder.L") < 0.2, "sway stays small");
	}

	#[test]
	fn idle_rolls_neck() {
		let mut rig = HumanoidV0Rig::imported();
		// Next yaw peak after the first scratch burst.
		Idle::default().apply(&mut rig, 1.1 / Idle::NECK_YAW_FREQ);

		let turned = rig.rotation("lower_neck") * Vec3::Z;
		assert!(turned.x.abs() > 0.08, "neck turn yaws in X, got {turned:?}");
		assert!(turned.x.abs() < 0.25, "turn stays a glance, got {turned:?}");
	}

	#[test]
	fn idle_neck_is_continuous_across_unit_progress() {
		let idle = Idle::default();
		let mut before = HumanoidV0Rig::imported();
		let mut after = HumanoidV0Rig::imported();
		idle.apply(&mut before, 0.999);
		idle.apply(&mut after, 1.001);

		let a_turn = before.rotation("lower_neck") * Vec3::Z;
		let b_turn = after.rotation("lower_neck") * Vec3::Z;
		let a_nod = before.rotation("lower_neck") * Vec3::Y;
		let b_nod = after.rotation("lower_neck") * Vec3::Y;
		assert!((a_turn.x - b_turn.x).abs() < 0.02, "turn {a_turn:?} {b_turn:?}");
		assert!((a_nod.z - b_nod.z).abs() < 0.02, "nod {a_nod:?} {b_nod:?}");
	}

	#[test]
	fn idle_scratch_raises_one_hand() {
		let mut idle_pose = HumanoidV0Rig::imported();
		let mut scratch_pose = HumanoidV0Rig::imported();
		let quiet = Idle::SCRATCH_DURATION + 0.2;
		let peak = Idle::SCRATCH_DURATION * 0.5;
		Idle::default().apply(&mut idle_pose, quiet);
		Idle::default().apply(&mut scratch_pose, peak);

		let side = Idle::scratch_side(peak);
		let forearm = match side {
			Side::Right => "forearm.R",
			Side::Left => "forearm.L",
		};
		assert!(
			scratch_pose.posed_angle(forearm) > idle_pose.posed_angle(forearm) + 0.4,
			"scratch should close the elbow"
		);
		assert_eq!(Idle::scratch_weight(quiet), 0.0);
	}

	fn bind_rig() -> HumanoidV0Rig {
		HumanoidV0Rig::for_clip_test()
	}

	/// Left hip joint +X offset from the neutral idle spread (0.25 m).
	fn left_hip_lateral_x(rig: &HumanoidV0Rig) -> f32 {
		rig.character_point("femur.L").x - 0.25
	}

	fn left_hip_y(rig: &HumanoidV0Rig) -> f32 {
		rig.character_point("femur.L").y
	}

	#[test]
	fn idle_pelvis_lateral_peaks_when_arm_sway_crosses_neutral() {
		let neutral = pose_at(0.25);
		let arm_peak = pose_at(0.25);
		let hip_peak = pose_at(0.5);

		let arm_peak_lateral = left_hip_lateral_x(&arm_peak);
		let hip_peak_lateral = left_hip_lateral_x(&hip_peak);

		assert!(
			arm_peak_lateral.abs() < 1e-4,
			"left hip X at arm-sway peak should match neutral spread: {arm_peak_lateral}"
		);
		assert!(
			hip_peak_lateral.abs() > 5e-5,
			"left hip X should shift at arm-neutral hip peak: {hip_peak_lateral}"
		);
		assert!(
			hip_peak_lateral.abs() > arm_peak_lateral.abs() + 4e-5,
			"hip peak lateral {hip_peak_lateral} vs arm peak {arm_peak_lateral}"
		);

		let bind = bind_rig();
		let arm_sway_z =
			bone_endpoint(&arm_peak, "forearm.L").z - bone_endpoint(&bind, "forearm.L").z;
		let hip_sway_z =
			bone_endpoint(&hip_peak, "forearm.L").z - bone_endpoint(&bind, "forearm.L").z;
		assert!(
			arm_sway_z.abs() > hip_sway_z.abs() + 0.01,
			"forward arm sway at 0.25 ({arm_sway_z}) should beat neutral at 0.5 ({hip_sway_z})"
		);

		let hip_bob = hip_peak.character_point("femur.L").y - neutral.character_point("femur.L").y;
		assert!(hip_bob.abs() > 0.006, "hip vertical bob at peak should be visible: {hip_bob}");
	}

	#[test]
	fn idle_pelvis_shift_opposes_arm_sway_in_character_space() {
		let samples = [0.0, 0.125, 0.25, 0.375, 0.5, 0.75, 1.0];
		let bind = bind_rig();
		let neutral = pose_at(0.25);
		let bind_arm_z = bone_endpoint(&bind, "forearm.L").z;
		let neutral_hip_y = left_hip_y(&neutral);

		let mut hip_dy = [0.0f32; 7];
		let mut arm_dz = [0.0f32; 7];
		for (i, p) in samples.iter().enumerate() {
			let rig = pose_at(*p);
			hip_dy[i] = left_hip_y(&rig) - neutral_hip_y;
			arm_dz[i] = bone_endpoint(&rig, "forearm.L").z - bind_arm_z;
		}

		let peak_hip_i = hip_dy
			.iter()
			.enumerate()
			.max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
			.map(|(i, _)| i)
			.expect("samples");
		assert!(
			matches!(samples[peak_hip_i], 0.0 | 0.5 | 1.0),
			"hip height peaks when arms cross neutral (got {})",
			samples[peak_hip_i]
		);
		assert!(
			hip_dy[2].abs() < 1e-4 && hip_dy[5].abs() < 1e-4,
			"hip height near neutral when arms peak"
		);
		assert!(
			arm_dz[2].abs() > 0.05 && arm_dz[5].abs() > 0.05,
			"arm sway peaks at 0.25 and 0.75"
		);
		assert!(arm_dz[4].abs() < 0.01, "arm forward sway near neutral when hips peak at 0.5");

		let hip_peak_lateral = left_hip_lateral_x(&pose_at(0.5));
		assert!(
			hip_peak_lateral < 0.0,
			"hip peak narrows +X spread (left hip moves toward center); got {}",
			hip_peak_lateral
		);
	}

	#[test]
	fn idle_feet_stay_planted_during_weight_shift() {
		let neutral = pose_at(0.25);
		for side in [Side::Left, Side::Right] {
			let femur = match side {
				Side::Left => "femur.L",
				Side::Right => "femur.R",
			};
			let neutral_hip = neutral.character_point(femur);
			let rest_foot = foot_end(&neutral, side);
			for progress in [0.0, 0.125, 0.25, 0.375, 0.5, 0.75, 1.0] {
				let rig = pose_at(progress);
				let hip = rig.character_point(femur);
				assert!(
					(hip - neutral_hip).length() < 0.01,
					"{side:?} hip joint drift at {progress}: {hip:?} vs neutral {neutral_hip:?}"
				);
				let foot = foot_end(&rig, side);
				assert!(
					(foot.y - rest_foot.y).abs() < 0.01,
					"{side:?} shin-end Y drift at {progress}: {foot:?} vs neutral {rest_foot:?}"
				);
				// Idle does not write femur/shin; ankle X skims ~3 cm at hip peak while Y stays grounded.
				assert!(
					(foot.x - rest_foot.x).abs() < 0.033,
					"{side:?} shin-end X skim at {progress}: {foot:?} vs neutral {rest_foot:?}"
				);
			}
		}
	}

	#[test]
	fn idle_pelvis_continuous_across_wrapped_unit_progress() {
		let before = pose_at(0.999);
		let after = pose_at(1.001);
		let wrap_a = pose_at(1.0);
		let wrap_b = pose_at(0.0);

		let pelvis_a = before.character_point("buttocks");
		let pelvis_b = after.character_point("buttocks");
		assert!(
			(pelvis_a - pelvis_b).length() < 0.002,
			"unwrapped pelvis continuous near 1.0: {pelvis_a:?} vs {pelvis_b:?}"
		);
		assert!(
			(wrap_a.character_point("buttocks") - wrap_b.character_point("buttocks")).length()
				< 0.002,
			"integer HIP_FREQ closes the loop at progress 0 == 1"
		);
	}

	#[test]
	fn idle_scratch_burst_unchanged_by_hip_phase() {
		let peak = Idle::SCRATCH_DURATION * 0.5;
		let quiet = Idle::SCRATCH_DURATION + 0.2;
		assert!(Idle::scratch_weight(quiet) < 1e-5);
		assert!(Idle::scratch_weight(peak) > 0.9);

		let mut scratch_pose = HumanoidV0Rig::for_clip_test();
		let mut idle_pose = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut scratch_pose, peak);
		Idle::default().apply(&mut idle_pose, quiet);

		let side = Idle::scratch_side(peak);
		let forearm = match side {
			Side::Right => "forearm.R",
			Side::Left => "forearm.L",
		};
		assert!(
			scratch_pose.posed_angle(forearm) > idle_pose.posed_angle(forearm) + 0.4,
			"scratch elbow flexion unchanged"
		);
	}

	#[test]
	fn idle_hip_shift_is_lighter_than_arms() {
		let mut rig = HumanoidV0Rig::imported();
		Idle::default().apply(&mut rig, Idle::SCRATCH_DURATION + 0.2);

		let hip = rig.posed_angle("pelvis.L");
		let shoulder = rig.posed_angle("shoulder.L") + rig.posed_angle("humerus.L");
		assert!(hip > 0.0, "pelvis shifts");
		assert!(hip < shoulder, "hip {hip} arms {shoulder}");
	}

	#[test]
	fn idle_does_not_lean_the_spine() {
		let mut rig = HumanoidV0Rig::imported();
		Idle::default().apply(&mut rig, 0.25);

		let root = tip(&rig, "root");
		assert!((root - Vec3::Y).length() < 1e-4, "root stays upright, got {root:?}");
		assert!(root.x.abs() < 1e-4, "no yaw, got {root:?}");
	}

	#[test]
	fn idle_phase_offset_changes_pose() {
		let idle = Idle::default();
		let mut a = HumanoidV0Rig::imported();
		let mut b = HumanoidV0Rig::imported();
		idle.apply(&mut a, 0.1);
		idle.apply(&mut b, 0.1 + Idle::phase_from_entity_bits(7));

		assert!(
			a.rotation("shoulder.L").angle_between(b.rotation("shoulder.L")) > 1e-4,
			"phase should move the shoulder"
		);
	}
}
