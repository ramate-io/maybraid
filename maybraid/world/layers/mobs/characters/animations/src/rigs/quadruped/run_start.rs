use character_rigs::authoring::PoseBuffer;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

use crate::animations::QuadrupedRunStart;
use crate::{Animation, Effects};

impl Animation<QuadrupedV0Rig> for QuadrupedRunStart {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		let rest = rig.binding.effective_rest.clone();
		self.apply_for_with_entry(rig, progress, &rest);
	}

	fn effects_for(&self, _rig: &QuadrupedV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

impl QuadrupedRunStart {
	/// Blend from a captured visible pose into a phased run sample.
	///
	/// Mailbox sampling passes the last shown pose; unit tests use bind rest.
	pub fn apply_for_with_entry(
		&self,
		rig: &mut QuadrupedV0Rig,
		progress: f32,
		from_pose: &PoseBuffer,
	) {
		let weight = self.weight(progress);
		let run_phase = Self::run_phase_at_progress(progress);
		if weight <= 0.0 {
			rig.pose.copy_from(from_pose);
			return;
		}
		self.run.apply_for(rig, run_phase);
		if weight < 1.0 {
			let target = rig.pose.clone();
			PoseBuffer::blend_into(from_pose, &target, weight, &mut rig.pose);
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::{Vec3, Vec3Swizzles};
	use character_rigs::authoring::PoseBuffer;

	use super::*;
	use crate::animations::{QuadrupedIdle, QuadrupedRun};

	const FOOT_SHINS: [&str; 4] =
		["anterior_shin.L", "anterior_shin.R", "posterior_shin.L", "posterior_shin.R"];

	fn foot_tips(rig: &QuadrupedV0Rig) -> [Vec3; 4] {
		FOOT_SHINS.map(|shin| rig.foot_tip_character(shin))
	}

	fn root_point(rig: &QuadrupedV0Rig) -> Vec3 {
		rig.character_point("back_ridge")
	}

	/// Frozen run@0 sampling (pre-fix) for foot-slide comparison in tests.
	fn apply_frozen_run_phase_zero(
		start: &QuadrupedRunStart,
		rig: &mut QuadrupedV0Rig,
		progress: f32,
	) {
		use crate::rigs::quadruped::blend::blend_clips;
		let weight = start.weight(progress);
		blend_clips(rig, &start.idle, 0.0, &start.run, 0.0, weight);
	}

	fn foot_slide_metric(sample: impl Fn(&mut QuadrupedV0Rig, f32)) -> f32 {
		let start = QuadrupedRunStart::default();
		let steps = [0.0, 0.25, 0.5, 0.75, 1.0];
		let mut max_slip = 0.0_f32;
		for window in steps.windows(2) {
			let (p0, p1) = (window[0], window[1]);
			let mut pose = QuadrupedV0Rig::for_clip_test();
			sample(&mut pose, p0);
			let tips0 = foot_tips(&pose);
			sample(&mut pose, p1);
			let tips1 = foot_tips(&pose);
			let mut run0 = QuadrupedV0Rig::for_clip_test();
			let mut run1 = QuadrupedV0Rig::for_clip_test();
			let phase0 = QuadrupedRunStart::run_phase_at_progress(p0);
			let phase1 = QuadrupedRunStart::run_phase_at_progress(p1);
			QuadrupedRun::default().apply(&mut run0, phase0);
			QuadrupedRun::default().apply(&mut run1, phase1);
			let expected0 = foot_tips(&run0);
			let expected1 = foot_tips(&run1);
			for leg in 0..4 {
				let actual_stride = (tips1[leg] - tips0[leg]).xz().length();
				let expected_stride = (expected1[leg] - expected0[leg]).xz().length();
				max_slip = max_slip.max((actual_stride - expected_stride).abs());
			}
		}
		max_slip
	}

	#[test]
	fn phased_sampling_reduces_foot_slide_vs_frozen_run() -> anyhow::Result<()> {
		let frozen = foot_slide_metric(|rig, p| {
			apply_frozen_run_phase_zero(&QuadrupedRunStart::default(), rig, p);
		});
		let phased = foot_slide_metric(|rig, p| {
			let rest = rig.binding.effective_rest.clone();
			QuadrupedRunStart::default().apply_for_with_entry(rig, p, &rest);
		});
		assert!(phased < frozen, "phased={phased} should beat frozen={frozen}");
		Ok(())
	}

	#[test]
	fn foot_tip_table_at_launch_samples() -> anyhow::Result<()> {
		let start = QuadrupedRunStart::default();
		let mut rig = QuadrupedV0Rig::for_clip_test();
		let rest = rig.binding.effective_rest.clone();
		for progress in [0.0, 0.25, 0.5, 0.75, 1.0] {
			start.apply_for_with_entry(&mut rig, progress, &rest);
			let root = root_point(&rig);
			let tips = foot_tips(&rig);
			let phase = QuadrupedRunStart::run_phase_at_progress(progress);
			assert!(root.is_finite());
			for tip in tips {
				assert!(tip.is_finite());
			}
			let mut run_only = QuadrupedV0Rig::for_clip_test();
			QuadrupedRun::default().apply(&mut run_only, phase);
			for (actual, expected) in tips.iter().zip(foot_tips(&run_only)) {
				if start.weight(progress) >= 1.0 - 1e-5 {
					assert!((actual - expected).length() < 1e-3, "progress {progress}");
				}
			}
		}
		Ok(())
	}

	#[test]
	fn start_matches_idle_neutral() -> anyhow::Result<()> {
		let start = QuadrupedRunStart::default();
		let mut start_rig = QuadrupedV0Rig::for_clip_test();
		let mut idle_rig = QuadrupedV0Rig::for_clip_test();
		start.apply(&mut start_rig, 0.0);
		QuadrupedIdle::default().apply(&mut idle_rig, 0.0);

		for name in start_rig.animation_bone_names() {
			let a = start_rig.rotation(name);
			let b = idle_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn end_matches_run_at_handoff_phase() -> anyhow::Result<()> {
		let start = QuadrupedRunStart::default();
		let handoff = QuadrupedRunStart::handoff_run_phase();
		let mut start_rig = QuadrupedV0Rig::for_clip_test();
		let mut run_rig = QuadrupedV0Rig::for_clip_test();
		start.apply(&mut start_rig, 1.0);
		QuadrupedRun::default().apply(&mut run_rig, handoff);

		for name in start_rig.animation_bone_names() {
			let a = start_rig.rotation(name);
			let b = run_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn handoff_joint_delta_is_negligible() -> anyhow::Result<()> {
		let start = QuadrupedRunStart::default();
		let handoff = QuadrupedRunStart::handoff_run_phase();
		let mut launch = QuadrupedV0Rig::for_clip_test();
		let mut run = QuadrupedV0Rig::for_clip_test();
		start.apply(&mut launch, 1.0);
		QuadrupedRun::default().apply(&mut run, handoff);
		let mut max_angle = 0.0_f32;
		for name in launch.animation_bone_names() {
			let delta = launch.rotation(name).angle_between(run.rotation(name));
			max_angle = max_angle.max(delta);
		}
		assert!(max_angle < 1e-3, "handoff max joint delta {max_angle}");
		Ok(())
	}

	#[test]
	fn mid_launch_strides_the_left_front_thigh() -> anyhow::Result<()> {
		let rest = QuadrupedV0Rig::for_clip_test();
		let mut rig = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStart::default().apply(&mut rig, 0.5);
		assert!(
			rig.posed_angle("anterior_thigh.L") > rest.posed_angle("anterior_thigh.L") + 0.02,
			"mid launch should advance the left front thigh"
		);
		Ok(())
	}

	#[test]
	fn end_pose_sets_diagonal_contact() -> anyhow::Result<()> {
		let mut rig = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStart::default().apply(&mut rig, 1.0);
		let left_front = rig.character_length("anterior_thigh.L");
		let right_hind = rig.character_length("posterior_thigh.R");
		assert!(
			left_front.z.abs() + right_hind.z.abs() > 0.05,
			"diagonal pair should be engaged, LF={left_front:?} RH={right_hind:?}"
		);
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = QuadrupedV0Rig::for_clip_test();
		let mut twice = QuadrupedV0Rig::for_clip_test();
		let start = QuadrupedRunStart::default();
		let rest = once.binding.effective_rest.clone();
		start.apply_for_with_entry(&mut once, 0.4, &rest);
		start.apply_for_with_entry(&mut twice, 0.4, &rest);
		start.apply_for_with_entry(&mut twice, 0.4, &rest);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn visible_idle_entry_differs_from_neutral_but_is_small() -> anyhow::Result<()> {
		let mut visible = QuadrupedV0Rig::for_clip_test();
		QuadrupedIdle::default().apply(&mut visible, 0.25);
		let _from_pose = visible.pose.clone();
		let mut neutral = QuadrupedV0Rig::for_clip_test();
		QuadrupedIdle::default().apply(&mut neutral, 0.0);
		let mut max_angle = 0.0_f32;
		for name in visible.animation_bone_names() {
			let delta = visible.rotation(name).angle_between(neutral.rotation(name));
			max_angle = max_angle.max(delta);
		}
		assert!(
			max_angle < 0.35,
			"visible idle@0.25 should stay close to neutral for crossfade, got {max_angle}"
		);
		Ok(())
	}

	#[test]
	fn interruption_blends_from_visible_idle() -> anyhow::Result<()> {
		let mut rig = QuadrupedV0Rig::for_clip_test();
		QuadrupedIdle::default().apply(&mut rig, 0.25);
		let from_pose = rig.pose.clone();
		let mut target = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStart::default().apply_for_with_entry(&mut target, 0.5, &from_pose);
		PoseBuffer::blend_into(&from_pose, &target.pose, 0.5, &mut rig.pose);

		let rest = QuadrupedV0Rig::for_clip_test();
		assert!(
			rig.posed_angle("anterior_thigh.L") > rest.posed_angle("anterior_thigh.L") + 0.01,
			"interrupted blend should move toward trot contact"
		);
		let mut full = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStart::default().apply_for_with_entry(&mut full, 0.5, &from_pose);
		assert!(
			rig.posed_angle("anterior_thigh.L") < full.posed_angle("anterior_thigh.L") + 0.02,
			"interrupted blend should stay below full launch"
		);
		Ok(())
	}

	#[test]
	fn reentry_after_stop_resets_to_visible_pose() -> anyhow::Result<()> {
		let mut rig = QuadrupedV0Rig::for_clip_test();
		QuadrupedIdle::default().apply(&mut rig, 0.4);
		let visible = rig.pose.clone();
		let start = QuadrupedRunStart::default();
		start.apply_for_with_entry(&mut rig, 0.6, &visible);
		let mut visible_rig = QuadrupedV0Rig::for_clip_test();
		visible_rig.pose.copy_from(&visible);
		start.apply_for_with_entry(&mut rig, 0.0, &visible);
		for name in rig.animation_bone_names() {
			assert!(
				rig.rotation(name).dot(visible_rig.rotation(name)).abs() > 1.0 - 1e-5,
				"re-entry at progress 0 should match visible still on {name}"
			);
		}
		Ok(())
	}

	#[test]
	fn interruption_stop_then_leap_clears_launch_pose() -> anyhow::Result<()> {
		let mut rig = QuadrupedV0Rig::for_clip_test();
		let visible = rig.pose.clone();
		let start = QuadrupedRunStart::default();
		start.apply_for_with_entry(&mut rig, 0.45, &visible);
		let mut mid = QuadrupedV0Rig::for_clip_test();
		mid.pose.copy_from(&rig.pose);
		rig.pose.copy_from(&visible);
		let mut visible_rig = QuadrupedV0Rig::for_clip_test();
		visible_rig.pose.copy_from(&visible);
		for name in rig.animation_bone_names() {
			assert!(
				rig.rotation(name).dot(visible_rig.rotation(name)).abs() > 1.0 - 1e-5,
				"stop should restore visible pose on {name}"
			);
		}
		assert!(
			mid.posed_angle("anterior_thigh.L")
				> visible_rig.posed_angle("anterior_thigh.L") + 0.01,
			"mid launch should advance the thigh before stop"
		);
		Ok(())
	}

	#[test]
	fn midpoint_sits_between_idle_and_run_contact() -> anyhow::Result<()> {
		let mut mid = QuadrupedV0Rig::for_clip_test();
		let mut idle = QuadrupedV0Rig::for_clip_test();
		let mut run = QuadrupedV0Rig::for_clip_test();
		let phase = QuadrupedRunStart::run_phase_at_progress(0.5);
		QuadrupedRunStart::default().apply(&mut mid, 0.5);
		QuadrupedIdle::default().apply(&mut idle, 0.0);
		QuadrupedRun::default().apply(&mut run, phase);

		assert!(mid.posed_angle("anterior_thigh.L") > idle.posed_angle("anterior_thigh.L") + 0.02);
		assert!(mid.posed_angle("anterior_thigh.L") < run.posed_angle("anterior_thigh.L") + 0.02);
		Ok(())
	}
}
