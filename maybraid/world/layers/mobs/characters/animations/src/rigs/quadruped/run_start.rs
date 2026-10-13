use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

use crate::animations::QuadrupedRunStart;
use crate::rigs::quadruped::blend::blend_clips;
use crate::{Animation, Effects};

impl Animation<QuadrupedV0Rig> for QuadrupedRunStart {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		let weight = self.weight(progress);
		blend_clips(rig, &self.idle, 0.0, &self.run, 0.0, weight);
	}

	fn effects_for(&self, _rig: &QuadrupedV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::authoring::PoseBuffer;

	use super::*;
	use crate::animations::{QuadrupedIdle, QuadrupedRun};

	fn tip(rig: &QuadrupedV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
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
	fn end_matches_run_phase_zero() -> anyhow::Result<()> {
		let start = QuadrupedRunStart::default();
		let mut start_rig = QuadrupedV0Rig::for_clip_test();
		let mut run_rig = QuadrupedV0Rig::for_clip_test();
		start.apply(&mut start_rig, 1.0);
		QuadrupedRun::default().apply(&mut run_rig, 0.0);

		for name in start_rig.animation_bone_names() {
			let a = start_rig.rotation(name);
			let b = run_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
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
		let left_front = tip(&rig, "anterior_thigh.L");
		let right_hind = tip(&rig, "posterior_thigh.R");
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
		start.apply(&mut once, 0.4);
		start.apply(&mut twice, 0.4);
		start.apply(&mut twice, 0.4);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn interruption_blends_from_visible_idle() -> anyhow::Result<()> {
		let mut rig = QuadrupedV0Rig::for_clip_test();
		QuadrupedIdle::default().apply(&mut rig, 0.25);
		let from_pose = rig.pose.clone();
		let mut target = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStart::default().apply(&mut target, 0.5);
		PoseBuffer::blend_into(&from_pose, &target.pose, 0.5, &mut rig.pose);

		let rest = QuadrupedV0Rig::for_clip_test();
		assert!(
			rig.posed_angle("anterior_thigh.L") > rest.posed_angle("anterior_thigh.L") + 0.01,
			"interrupted blend should move toward trot contact"
		);
		let mut full = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStart::default().apply(&mut full, 0.5);
		assert!(
			rig.posed_angle("anterior_thigh.L") < full.posed_angle("anterior_thigh.L") + 0.02,
			"interrupted blend should stay below full launch"
		);
		Ok(())
	}

	#[test]
	fn midpoint_sits_between_idle_and_run_contact() -> anyhow::Result<()> {
		let mut mid = QuadrupedV0Rig::for_clip_test();
		let mut idle = QuadrupedV0Rig::for_clip_test();
		let mut run = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStart::default().apply(&mut mid, 0.5);
		QuadrupedIdle::default().apply(&mut idle, 0.0);
		QuadrupedRun::default().apply(&mut run, 0.0);

		assert!(mid.posed_angle("anterior_thigh.L") > idle.posed_angle("anterior_thigh.L") + 0.02);
		assert!(mid.posed_angle("anterior_thigh.L") < run.posed_angle("anterior_thigh.L") + 0.02);
		Ok(())
	}
}
