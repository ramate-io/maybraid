use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{Run, UprightRun};
use crate::rigs::humanoid::apply::{apply_arm, apply_root};
use crate::rigs::humanoid::gait_knee::lerp_swing_knee;
use crate::{Animation, Progress};

impl Run {
	/// Authored semantic pose at `progress`. Rest is applied later by the rig.
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		UprightRun::from_run(self).sample_pose(progress)
	}
}

impl Animation<HumanoidV0Rig> for Run {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		rig.write_pose(&self.sample_pose(progress))
	}
}

impl UprightRun {
	/// Authored semantic pose at `progress`. Rest is applied later by the rig.
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		let phase = Progress(progress).cycle();
		let left_arm_swing = -arm_swing(phase);
		let right_arm_swing = arm_swing(phase + 0.5);

		apply_root(&mut pose, self.torso_lean);
		apply_leg(&mut pose, Side::Left, phase, -1.0, self);
		apply_leg(&mut pose, Side::Right, phase, 1.0, self);
		// Both elbows share one flexion sign. Opposite arm_down values are a hang bias.
		apply_run_arm(&mut pose, Side::Left, left_arm_swing, phase, -self.arm_down, self);
		apply_run_arm(&mut pose, Side::Right, right_arm_swing, phase, self.arm_down, self);
		pose
	}
}

impl Animation<HumanoidV0Rig> for UprightRun {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		rig.write_pose(&self.sample_pose(progress));
	}
}

fn apply_leg(pose: &mut HumanoidPose, side: Side, phase: f32, lift_sign: f32, run: &UprightRun) {
	let phase = if side == Side::Left { phase } else { phase + 0.5 };
	let swing = thigh_swing(phase);
	let leg = pose.leg_mut(side);
	// Pelvis yaw stays axial. Hip lift is a lateral hike; lift_sign is gait bias.
	leg.pelvis_turn = swing * run.hip_swing;
	leg.pelvis_lateral = hip_lift(swing, run.hip_lift) * lift_sign;
	leg.hip_flexion = swing * run.stride;
	leg.knee_flexion = knee_flex(phase, run) - run.knee_extended;
}

fn apply_run_arm(
	pose: &mut HumanoidPose,
	side: Side,
	arm_swing_value: f32,
	phase: f32,
	humerus_flex: f32,
	run: &UprightRun,
) {
	apply_arm(
		pose,
		side,
		arm_swing_value * run.shoulder_swing,
		-shoulder_lift(arm_swing_value, run.shoulder_lift),
		arm_swing_value * run.humerus_swing_scale,
		humerus_flex,
		elbow_flex(arm_swing_value, phase, -1.0, run),
	);
}

fn thigh_swing(phase: f32) -> f32 {
	let p = phase.fract();
	if p < 0.5 {
		4.0 * p - 1.0
	} else {
		3.0 - 4.0 * p
	}
}

fn arm_swing(phase: f32) -> f32 {
	thigh_swing(phase) * 0.75
}

fn elbow_flex(arm_swing: f32, phase: f32, flex_sign: f32, run: &UprightRun) -> f32 {
	let pump = arm_swing.abs();
	let cycle = ((phase + arm_swing.signum() * 0.125) * std::f32::consts::PI * 4.0).sin().abs();
	flex_sign * (run.elbow_bend + pump * run.elbow_pump + cycle * run.elbow_cycle)
}

fn shoulder_lift(arm_swing: f32, amplitude: f32) -> f32 {
	arm_swing * amplitude
}

fn hip_lift(leg_swing: f32, amplitude: f32) -> f32 {
	leg_swing * amplitude
}

fn knee_flex(leg_phase: f32, run: &UprightRun) -> f32 {
	let p = leg_phase.fract();
	if p < 0.5 {
		let t = p * 2.0;
		run.knee_neutral + (t * std::f32::consts::PI).sin() * (run.knee_extended - run.knee_neutral)
	} else {
		lerp_swing_knee(p, run.knee_extended, run.knee_contracted)
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;
	use crate::animations::{Mix, UprightWalk, Walk};

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn bone_length(rig: &HumanoidV0Rig, name: &str) -> f32 {
		let id = rig.binding.definition.id(name).expect(name);
		rig.binding.effective_rest.get(id).expect("rest").translation.length()
	}

	/// Distal end of a bone in character space.
	fn bone_end(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.character_point(name) + rig.character_length(name) * bone_length(rig, name)
	}

	fn run_without_lean() -> UprightRun {
		UprightRun { torso_lean: 0.0, ..UprightRun::default() }
	}

	fn walk_without_lean() -> UprightWalk {
		UprightWalk { torso_lean: 0.0, ..UprightWalk::default() }
	}

	fn apply_upright_run(rig: &mut HumanoidV0Rig, run: &UprightRun, phase: f32) {
		run.apply_for(rig, phase);
	}

	fn assert_pose_matches_at_phases(phases: &[f32]) {
		for &phase in phases {
			let mut from_run = HumanoidV0Rig::imported();
			let mut from_upright = HumanoidV0Rig::imported();
			Run::default().apply(&mut from_run, phase);
			UprightRun::default().apply(&mut from_upright, phase);

			for name in from_run.animation_bone_names() {
				let run_rot = from_run.rotation(name);
				let upright_rot = from_upright.rotation(name);
				assert!(
					run_rot.dot(upright_rot).abs() > 1.0 - 1e-5,
					"rotation mismatch on {name} at {phase}"
				);
			}
		}
	}

	#[test]
	fn run_delegates_to_upright_default() {
		assert_pose_matches_at_phases(&[0.0, 0.25, 0.5, 0.75]);
	}

	#[test]
	fn run_flexes_the_left_femur_sagittally() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("femur.L") > 0.2, "stride leaves rest");
	}

	#[test]
	fn run_right_leg_uses_half_cycle_phase_offset() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		UprightRun::default().apply(&mut rig, 0.0);

		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!((left.z - right.z).abs() > 0.05, "legs are out of phase, L={left:?} R={right:?}");
	}

	#[test]
	fn run_applies_knee_flex_to_shin() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.25);
		let extended = rig.posed_angle("shin.L");
		UprightRun::default().apply(&mut rig, 0.75);
		assert!(extended < 0.05, "expected a straight knee at this phase, got {extended}");
		assert!(rig.posed_angle("shin.L") > 0.5, "expected knee tuck later in the stride");
	}

	#[test]
	fn run_applies_elbow_bend_to_forearm() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("forearm.L") > 0.9, "expected elbow bend baseline");
	}

	#[test]
	fn run_applies_shoulder_and_hip_lift() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.0);

		let shoulder = tip(&rig, "shoulder.L");
		let pelvis = tip(&rig, "pelvis.L");
		assert!(shoulder.x.abs() > 0.0, "expected shoulder bounce, got {shoulder:?}");
		assert!(pelvis.x.abs() > 0.0, "expected hip bounce, got {pelvis:?}");
	}

	#[test]
	fn character_forward_is_positive_z_on_clip_fixture() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let mut pose = character_rigs::authoring::HumanoidPose::default();
		pose.spine.add_root_forward(0.1);
		rig.write_pose(&pose);
		let forward = rig.character_length("root");
		assert!(forward.z > 0.0, "+Z is fight-forward on for_clip_test, got {forward:?}");
		assert!(forward.x.abs() < 1e-3, "forward lean stays sagittal, got {forward:?}");
	}

	#[test]
	fn run_forward_pitch_moves_upper_body_down_and_ahead() {
		let phases = [0.0, 0.25, 0.5, 0.75];
		let before = run_without_lean();
		let after = UprightRun::default();
		let walk = UprightWalk::default();

		for phase in phases {
			let mut main = HumanoidV0Rig::for_clip_test();
			apply_upright_run(&mut main, &after, phase);
			let mut prev = HumanoidV0Rig::for_clip_test();
			apply_upright_run(&mut prev, &before, phase);
			let mut walk_rig = HumanoidV0Rig::for_clip_test();
			walk.apply_for(&mut walk_rig, phase);

			for name in ["upper_back", "lower_neck", "upper_neck"] {
				let main_pt = bone_end(&main, name);
				let prev_pt = bone_end(&prev, name);
				let walk_pt = bone_end(&walk_rig, name);
				assert!(
					main_pt.z > prev_pt.z + 0.03,
					"{name} at {phase}: run after {main_pt:?} should be ahead of before {prev_pt:?}"
				);
				assert!(
					main_pt.y < prev_pt.y + 1e-3,
					"{name} at {phase}: forward pitch lowers the upper body, after {main_pt:?} vs before {prev_pt:?}"
				);
				assert!(
					main_pt.z > walk_pt.z,
					"{name} at {phase}: run after {main_pt:?} should pitch past walk {walk_pt:?}"
				);
			}
		}
	}

	#[test]
	fn run_lean_foot_shift_matches_walk_lean_pattern() {
		const FOOT_TOLERANCE: f32 = 0.02;
		let phases = [0.0, 0.25, 0.5, 0.75];
		let run_before = run_without_lean();
		let run_after = UprightRun::default();
		let walk_before = walk_without_lean();
		let walk_after = UprightWalk::default();

		for phase in phases {
			let mut run_prev = HumanoidV0Rig::for_clip_test();
			let mut run_next = HumanoidV0Rig::for_clip_test();
			apply_upright_run(&mut run_prev, &run_before, phase);
			apply_upright_run(&mut run_next, &run_after, phase);

			let mut walk_prev = HumanoidV0Rig::for_clip_test();
			let mut walk_next = HumanoidV0Rig::for_clip_test();
			walk_before.apply_for(&mut walk_prev, phase);
			walk_after.apply_for(&mut walk_next, phase);

			for (side, shin) in [("L", "shin.L"), ("R", "shin.R")] {
				let run_delta = bone_end(&run_next, shin) - bone_end(&run_prev, shin);
				let walk_delta = bone_end(&walk_next, shin) - bone_end(&walk_prev, shin);
				assert!(
					run_delta.length() < FOOT_TOLERANCE,
					"{shin} at {phase}: run foot should stay near pre-change run, delta {run_delta:?}"
				);
				assert!(
					walk_delta.length() < FOOT_TOLERANCE,
					"{shin} at {phase}: walk shows the same lean artifact, delta {walk_delta:?}"
				);
				assert!(
					(run_delta - walk_delta).length() < FOOT_TOLERANCE,
					"{side} foot at {phase}: run and walk lean shifts should match, run {run_delta:?} walk {walk_delta:?}"
				);
			}
		}
	}

	#[test]
	fn run_head_neck_forward_relative_to_main() {
		let before = run_without_lean();
		let after = UprightRun::default();
		for phase in [0.0, 0.25, 0.5, 0.75] {
			let mut main = HumanoidV0Rig::for_clip_test();
			let mut prev = HumanoidV0Rig::for_clip_test();
			apply_upright_run(&mut main, &after, phase);
			apply_upright_run(&mut prev, &before, phase);

			let neck_delta = bone_end(&main, "lower_neck").z - bone_end(&prev, "lower_neck").z;
			let head_delta = bone_end(&main, "upper_neck").z - bone_end(&prev, "upper_neck").z;
			assert!(neck_delta > 0.04, "neck should move forward at {phase}, delta {neck_delta}");
			assert!(head_delta > 0.04, "head should move forward at {phase}, delta {head_delta}");
		}
	}

	#[test]
	fn walk_run_blend_interpolates_forward_lean() {
		let mix = Mix::new(Walk::default(), Run::default(), 0.5);
		for phase in [0.0, 0.25, 0.5, 0.75] {
			let mut walk = HumanoidV0Rig::for_clip_test();
			let mut run = HumanoidV0Rig::for_clip_test();
			let mut blended = HumanoidV0Rig::for_clip_test();
			Walk::default().apply(&mut walk, phase);
			Run::default().apply(&mut run, phase);
			mix.apply_for(&mut blended, phase);

			let walk_z = bone_end(&walk, "upper_back").z;
			let run_z = bone_end(&run, "upper_back").z;
			let blend_z = bone_end(&blended, "upper_back").z;
			assert!(
				blend_z > walk_z && blend_z < run_z,
				"50/50 blend at {phase} should sit between walk {walk_z} and run {run_z}, got {blend_z}"
			);
			assert!(
				walk_z > 0.0 && run_z > 0.0 && blend_z > 0.0,
				"lean stays forward (+Z) at {phase}: walk {walk_z}, blend {blend_z}, run {run_z}"
			);
		}
	}

	#[test]
	#[ignore = "manual evidence table for stylization reviews"]
	fn run_lean_evidence_dump() {
		let phases = [0.0, 0.25, 0.5, 0.75];
		let before = run_without_lean();
		let after = UprightRun::default();
		eprintln!("bone | phase | walk (x,y,z) | run before | run after");
		for phase in phases {
			let mut walk_rig = HumanoidV0Rig::for_clip_test();
			Walk::default().apply(&mut walk_rig, phase);
			let mut prev = HumanoidV0Rig::for_clip_test();
			let mut next = HumanoidV0Rig::for_clip_test();
			apply_upright_run(&mut prev, &before, phase);
			apply_upright_run(&mut next, &after, phase);
			for name in ["upper_back", "lower_neck", "upper_neck", "shin.L", "shin.R"] {
				let walk_pt = bone_end(&walk_rig, name);
				let before_pt = bone_end(&prev, name);
				let after_pt = bone_end(&next, name);
				eprintln!(
					"{name} | {phase:.2} | ({wx:.4}, {wy:.4}, {wz:.4}) | ({bx:.4}, {by:.4}, {bz:.4}) | ({ax:.4}, {ay:.4}, {az:.4})",
					wx = walk_pt.x,
					wy = walk_pt.y,
					wz = walk_pt.z,
					bx = before_pt.x,
					by = before_pt.y,
					bz = before_pt.z,
					ax = after_pt.x,
					ay = after_pt.y,
					az = after_pt.z,
				);
			}
		}
		let mix = Mix::new(Walk::default(), Run::default(), 0.5);
		eprintln!("50/50 blend upper_back.z by phase:");
		for phase in phases {
			let mut walk = HumanoidV0Rig::for_clip_test();
			let mut run = HumanoidV0Rig::for_clip_test();
			let mut blended = HumanoidV0Rig::for_clip_test();
			Walk::default().apply(&mut walk, phase);
			Run::default().apply(&mut run, phase);
			mix.apply_for(&mut blended, phase);
			eprintln!(
				"phase {phase:.2}: walk {walk_z:.4} blend {blend_z:.4} run {run_z:.4}",
				walk_z = bone_end(&walk, "upper_back").z,
				blend_z = bone_end(&blended, "upper_back").z,
				run_z = bone_end(&run, "upper_back").z,
			);
		}
	}

	#[test]
	fn run_applies_forward_torso_lean() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		UprightRun::default().apply(&mut rig, 0.25);

		let root = tip(&rig, "root");
		assert!(root.z > 0.08, "forward lean goes to +Z, got {root:?}");
		assert!(root.x.abs() < 1e-3, "lean must not yaw, got {root:?}");
	}

	#[test]
	fn run_lean_exceeds_walk_for_matching_stride_scale() {
		let mut walk = HumanoidV0Rig::for_clip_test();
		Walk::default().apply(&mut walk, 0.25);
		let mut run = HumanoidV0Rig::for_clip_test();
		Run::default().apply(&mut run, 0.25);

		assert!(
			tip(&run, "root").z > tip(&walk, "root").z + 0.02,
			"run should pitch farther forward than walk"
		);
	}
}
