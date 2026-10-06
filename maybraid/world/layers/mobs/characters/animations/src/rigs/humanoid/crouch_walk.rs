use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{CrouchedWalk, Squat, UprightWalk};
use crate::rigs::humanoid::apply::{apply_hip_fold, apply_leg, apply_spine_pitch};
use crate::rigs::humanoid::walk::{overlay_walk_arm, overlay_walk_leg, thigh_swing};
use crate::{Animation, Progress};

impl Animation<HumanoidV0Rig> for CrouchedWalk {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		sample_crouched_walk(self, progress, &mut pose);
		rig.write_pose(&pose);
	}
}

fn sample_crouched_walk(crouch: &CrouchedWalk, progress: f32, pose: &mut HumanoidPose) {
	let squat = Squat::held();
	let depth = crouch.depth;
	let femur_base = squat.femur_swing(depth);
	let shin_base = squat.shin_flex(depth);

	apply_leg(pose, Side::Left, femur_base, shin_base);
	apply_leg(pose, Side::Right, femur_base, shin_base);
	let hip = squat.hip_fold(depth);
	if hip.abs() > f32::EPSILON {
		apply_hip_fold(pose, Side::Left, hip);
		apply_hip_fold(pose, Side::Right, hip);
	}
	apply_spine_pitch(pose, squat.root_swing(depth));

	let leg_cycle = crouch.leg_cycle;
	if leg_cycle <= f32::EPSILON {
		return;
	}

	let walk = crouched_upright_walk(&crouch.walk);
	let phase = Progress(progress).cycle();
	overlay_walk_leg(pose, Side::Left, phase, -1.0, &walk, leg_cycle);
	overlay_walk_leg(pose, Side::Right, phase, 1.0, &walk, leg_cycle);

	let left_arm_swing = -arm_swing(phase);
	let right_arm_swing = arm_swing(phase + 0.5);
	let arm_scale = leg_cycle * 0.35;
	overlay_walk_arm(pose, Side::Left, left_arm_swing, phase, -walk.arm_down, &walk, arm_scale);
	overlay_walk_arm(pose, Side::Right, right_arm_swing, phase, walk.arm_down, &walk, arm_scale);
}

/// Crouch gait keeps the folded torso; only leg stride and a small arm counter-swing cycle.
fn crouched_upright_walk(walk: &crate::animations::Walk) -> UprightWalk {
	let upright = UprightWalk::from_walk(walk);
	UprightWalk { hip_lift: 0.0, torso_lean: 0.0, shoulder_lift: 0.0, ..upright }
}

fn arm_swing(phase: f32) -> f32 {
	thigh_swing(phase) * 0.75
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Vec3;

	fn crouch_depth_metric(rig: &HumanoidV0Rig) -> f32 {
		rig.posed_angle("femur.L")
			+ rig.posed_angle("shin.L")
			+ rig.posed_angle("root")
			+ rig.posed_angle("pelvis.L")
	}

	fn min_depth_over_cycle(crouch: CrouchedWalk) -> f32 {
		let samples = 64;
		let mut min_depth = f32::MAX;
		for i in 0..samples {
			let phase = i as f32 / samples as f32;
			let mut rig = HumanoidV0Rig::for_clip_test();
			crouch.apply(&mut rig, phase);
			min_depth = min_depth.min(crouch_depth_metric(&rig));
		}
		min_depth
	}

	#[test]
	fn crouch_walk_preserves_squat_depth() -> anyhow::Result<()> {
		let depth = 1.0;
		let max_weight = 0.40;
		let mut squat = HumanoidV0Rig::for_clip_test();
		Squat::held().apply(&mut squat, depth);
		let squat_depth = crouch_depth_metric(&squat);

		let crouch = CrouchedWalk::blended(depth, max_weight);
		let min_depth = min_depth_over_cycle(crouch);
		let drop = (squat_depth - min_depth) / squat_depth;
		assert!(
			drop < 0.10,
			"crouch-walk depth should stay within 10% of held squat (drop={:.1}%, squat={:.4}, min={:.4})",
			drop * 100.0,
			squat_depth,
			min_depth
		);
		Ok(())
	}

	#[test]
	fn leg_cycle_zero_matches_held_squat() -> anyhow::Result<()> {
		let depth = 1.0;
		let mut squat = HumanoidV0Rig::for_clip_test();
		Squat::held().apply(&mut squat, depth);
		let mut crouch = HumanoidV0Rig::for_clip_test();
		CrouchedWalk::blended(depth, 0.0).apply(&mut crouch, 0.35);

		for name in ["femur.L", "femur.R", "shin.L", "shin.R", "root", "lumbar"] {
			assert!(
				squat.rotation(name).dot(crouch.rotation(name)).abs() > 1.0 - 1e-5,
				"leg_cycle 0 should match held squat on {name}"
			);
		}
		Ok(())
	}

	#[test]
	fn leg_cycle_moves_femurs_with_phase() -> anyhow::Result<()> {
		let crouch = CrouchedWalk::blended(1.0, 0.35);
		let mut early = HumanoidV0Rig::for_clip_test();
		crouch.apply(&mut early, 0.0);
		let mut late = HumanoidV0Rig::for_clip_test();
		crouch.apply(&mut late, 0.5);
		assert!(
			early.posed_angle("femur.L") != late.posed_angle("femur.L"),
			"walk phase should move the legs"
		);
		let mut squat = HumanoidV0Rig::for_clip_test();
		Squat::held().apply(&mut squat, 1.0);
		assert!(
			early.posed_angle("root") > squat.posed_angle("root") * 0.85,
			"spine stays mostly folded while legs cycle"
		);
		Ok(())
	}

	#[test]
	fn full_pose_mix_dilutes_depth_more_than_crouched_walk() {
		use crate::animations::{Mix, Walk};

		let depth = 1.0;
		let max_weight = 0.40;
		let walk = Walk { stride: 0.20, bounce: 0.40, rotation: 0.30 };
		let mix = Mix::new(Squat::held(), walk, max_weight);

		let mut squat = HumanoidV0Rig::for_clip_test();
		Squat::held().apply(&mut squat, depth);
		let squat_depth = crouch_depth_metric(&squat);

		let samples = 64;
		let mut mix_min = f32::MAX;
		for i in 0..samples {
			let phase = i as f32 / samples as f32;
			let mut rig = HumanoidV0Rig::for_clip_test();
			mix.apply_at(&mut rig, depth, phase);
			mix_min = mix_min.min(crouch_depth_metric(&rig));
		}
		let mix_drop = (squat_depth - mix_min) / squat_depth;

		let crouch = CrouchedWalk::blended(depth, max_weight);
		let crouch_min = min_depth_over_cycle(crouch);
		let crouch_drop = (squat_depth - crouch_min) / squat_depth;

		assert!(
			mix_drop > 0.10,
			"full-pose Mix should dilute depth (mix_drop={:.1}%)",
			mix_drop * 100.0
		);
		assert!(
			crouch_drop < 0.10,
			"CrouchedWalk should preserve depth (crouch_drop={:.1}%)",
			crouch_drop * 100.0
		);
		eprintln!(
			"depth metric: squat={:.4} mix_min={:.4} ({:.1}% drop) crouch_min={:.4} ({:.1}% drop)",
			squat_depth,
			mix_min,
			mix_drop * 100.0,
			crouch_min,
			crouch_drop * 100.0
		);
	}

	#[test]
	fn crouch_walk_has_no_armature_move() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let effects = CrouchedWalk::blended(1.0, 0.35).apply(&mut rig, 0.25);
		assert!(effects.is_identity());
	}

	#[test]
	fn crouch_walk_keeps_hips_low() {
		let mut squat = HumanoidV0Rig::for_clip_test();
		Squat::held().apply(&mut squat, 1.0);
		let squat_tip = squat.rotation("root") * Vec3::Y;

		let crouch = CrouchedWalk::blended(1.0, 0.40);
		let mut worst = HumanoidV0Rig::for_clip_test();
		let mut worst_root = squat_tip;
		for i in 0..64 {
			let phase = i as f32 / 64.0;
			crouch.apply(&mut worst, phase);
			let root = worst.rotation("root") * Vec3::Y;
			if root.y > worst_root.y {
				worst_root = root;
			}
		}
		assert!(
			worst_root.y <= squat_tip.y + 0.02,
			"spine should not unfold above squat depth, squat_y={:.3} worst_y={:.3}",
			squat_tip.y,
			worst_root.y
		);
	}
}
