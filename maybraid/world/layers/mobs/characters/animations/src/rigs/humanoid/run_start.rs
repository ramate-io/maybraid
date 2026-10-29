use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::RunStart;
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl RunStart {
	/// Neutral idle pose at progress zero.
	pub fn idle_pose(&self) -> HumanoidPose {
		self.idle.sample_pose(0.0)
	}

	/// First run contact pose at cycle phase zero.
	pub fn run_pose(&self) -> HumanoidPose {
		self.run.sample_pose(0.0)
	}
}

impl Animation<HumanoidV0Rig> for RunStart {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		blend_clips(rig, &self.idle, 0.0, &self.run, 0.0, self.weight(progress));
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use crate::animations::DEFAULT_RUN_START_SPEED;
	use crate::animations::{Idle, Run};
	use bevy::prelude::{Transform, Vec3};
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::animations::Transition;

	fn assert_pose_near(
		a: &HumanoidV0Rig,
		b: &HumanoidV0Rig,
		tolerance: f32,
	) -> anyhow::Result<()> {
		for name in a.animation_bone_names() {
			let id = a
				.binding
				.definition
				.id(name)
				.ok_or_else(|| anyhow::anyhow!("missing animation bone {name}"))?;
			let left = a.pose.local[id.index()];
			let right = b.pose.local[id.index()];
			assert!(
				left.translation.distance(right.translation) <= tolerance,
				"translation: {name}"
			);
			assert!(left.rotation.dot(right.rotation).abs() >= 1.0 - tolerance, "rotation: {name}");
			assert!(left.scale.distance(right.scale) <= tolerance, "scale: {name}");
		}
		Ok(())
	}

	fn foot(rig: &HumanoidV0Rig) -> Vec3 {
		rig.character_point("shin.L") + rig.character_length("shin.L") * rig.segment_lengths.shin
	}

	fn sampled(progress: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		RunStart::default().apply(&mut rig, progress);
		rig
	}

	#[test]
	fn endpoint_poses_clamp_and_match_idle_and_run() -> anyhow::Result<()> {
		let mut start = HumanoidV0Rig::for_clip_test();
		let mut idle = HumanoidV0Rig::for_clip_test();
		RunStart::default().apply(&mut start, 0.0);
		Idle::default().apply(&mut idle, 0.0);
		assert_pose_near(&start, &idle, 1e-6)?;
		let mut below = HumanoidV0Rig::for_clip_test();
		RunStart::default().apply(&mut below, -0.5);
		assert_pose_near(&below, &idle, 1e-6)?;

		let mut end = HumanoidV0Rig::for_clip_test();
		let mut run = HumanoidV0Rig::for_clip_test();
		RunStart::default().apply(&mut end, 1.0);
		Run::default().apply(&mut run, 0.0);
		assert_pose_near(&end, &run, 1e-6)?;
		let mut above = HumanoidV0Rig::for_clip_test();
		RunStart::default().apply(&mut above, 1.5);
		assert_pose_near(&above, &run, 1e-6)?;
		Ok(())
	}

	#[test]
	fn production_bind_foot_moves_continuously_at_sixty_hz() -> anyhow::Result<()> {
		let start = foot(&sampled(0.0));
		let end = foot(&sampled(1.0));
		assert!(start.distance(end) > 0.035, "run launch must move the actual foot endpoint");
		let mut previous = start;
		let mut largest_step = 0.0_f32;
		for frame in 1..=14 {
			let pose = foot(&sampled(frame as f32 * DEFAULT_RUN_START_SPEED / 60.0));
			largest_step = largest_step.max(previous.distance(pose));
			previous = pose;
		}
		assert!(largest_step < 0.11, "60 Hz foot movement jumped by {largest_step}");
		let epsilon = 1.0 / 240.0;
		assert!(foot(&sampled(0.0)).distance(foot(&sampled(epsilon))) < 0.005);
		assert!(foot(&sampled(1.0 - epsilon)).distance(foot(&sampled(1.0))) < 0.005);
		Ok(())
	}

	#[test]
	fn changed_limb_lengths_resample_safely_without_root_effects() -> anyhow::Result<()> {
		let mut changed = HumanoidV0Rig::for_clip_test();
		changed.seed_rest("femur.L", Transform::from_translation(Vec3::Y * 0.8));
		changed.seed_rest("femur.R", Transform::from_translation(Vec3::Y * 0.8));
		changed.seed_rest("shin.L", Transform::from_translation(Vec3::Y * 0.65));
		changed.seed_rest("shin.R", Transform::from_translation(Vec3::Y * 0.65));
		let mut direct = changed.clone();
		let clip = RunStart::default();
		for progress in [0.6, 0.0, 0.4, 1.0, 0.4] {
			clip.apply(&mut changed, progress);
			assert_eq!(changed.scratch.depth, 0);
			assert!(changed.pose.local.iter().all(|t| t.translation.is_finite()
				&& t.rotation.is_finite()
				&& t.scale.is_finite()));
		}
		clip.apply(&mut direct, 0.4);
		assert_pose_near(&changed, &direct, 1e-5)?;
		assert_eq!(clip.effects_for(&direct, 0.4), ArmatureOffset::IDENTITY);
		Ok(())
	}

	#[test]
	fn visible_pose_transition_and_interrupt_keep_captured_pose() -> anyhow::Result<()> {
		let mut visible = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut visible, 0.33);
		let captured = visible.pose.clone();
		let transition = Transition::from_visible(
			RunStart::default(),
			captured.clone(),
			ArmatureOffset::IDENTITY,
		);
		transition.apply(&mut visible, 0.5, 0.0);
		assert_eq!(visible.pose, captured);
		let mut full = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut full, 0.5, 1.0);
		let mut target = HumanoidV0Rig::for_clip_test();
		RunStart::default().apply(&mut target, 0.5);
		assert_pose_near(&full, &target, 1e-6)?;
		let from = full.pose.clone();
		Transition::from_visible(Idle::default(), from.clone(), ArmatureOffset::IDENTITY)
			.apply(&mut full, 0.0, 0.0);
		assert_eq!(full.pose, from, "interrupt starts at latest visible pose");
		Ok(())
	}
}
