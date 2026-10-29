use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::WalkStart;
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl WalkStart {
	/// Neutral idle pose at progress zero.
	pub fn idle_pose(&self) -> HumanoidPose {
		self.idle.sample_pose(0.0)
	}

	/// First walk contact pose at cycle phase zero.
	pub fn walk_pose(&self) -> HumanoidPose {
		self.walk.sample_pose(0.0)
	}
}

impl Animation<HumanoidV0Rig> for WalkStart {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		blend_clips(rig, &self.idle, 0.0, &self.walk, 0.0, self.weight(progress));
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use crate::animations::DEFAULT_WALK_START_SPEED;
	use crate::animations::{Idle, Walk};
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
			let left_rest = a
				.binding
				.effective_rest
				.get(id)
				.ok_or_else(|| anyhow::anyhow!("missing effective rest for {name}"))?;
			let right_rest = b
				.binding
				.effective_rest
				.get(id)
				.ok_or_else(|| anyhow::anyhow!("missing effective rest for {name}"))?;
			assert!(
				left.translation.distance(left_rest.translation) <= tolerance
					&& right.translation.distance(right_rest.translation) <= tolerance,
				"{name} translation must remain at its own effective rest"
			);
			if left_rest.translation.distance(right_rest.translation) <= tolerance {
				assert!(
					left.translation.distance(right.translation) <= tolerance,
					"translation: {name}"
				);
			}
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
		WalkStart::default().apply(&mut rig, progress);
		rig
	}

	#[test]
	fn endpoint_poses_clamp_and_match_idle_and_walk() -> anyhow::Result<()> {
		let mut start = HumanoidV0Rig::for_clip_test();
		let mut idle = HumanoidV0Rig::for_clip_test();
		WalkStart::default().apply(&mut start, 0.0);
		Idle::default().apply(&mut idle, 0.0);
		assert_pose_near(&start, &idle, 1e-6)?;
		let mut below = HumanoidV0Rig::for_clip_test();
		WalkStart::default().apply(&mut below, -0.4);
		assert_pose_near(&below, &idle, 1e-6)?;

		let mut end = HumanoidV0Rig::for_clip_test();
		let mut walk = HumanoidV0Rig::for_clip_test();
		WalkStart::default().apply(&mut end, 1.0);
		Walk::default().apply(&mut walk, 0.0);
		assert_pose_near(&end, &walk, 1e-6)?;
		let mut above = HumanoidV0Rig::for_clip_test();
		WalkStart::default().apply(&mut above, 1.4);
		assert_pose_near(&above, &walk, 1e-6)?;
		Ok(())
	}

	#[test]
	fn production_bind_foot_moves_continuously_at_sixty_hz() -> anyhow::Result<()> {
		let start = foot(&sampled(0.0));
		let end = foot(&sampled(1.0));
		assert!(start.distance(end) > 0.025, "step-off must move the actual foot endpoint");
		let mut previous = start;
		let mut largest_step = 0.0_f32;
		for frame in 1..=17 {
			let pose = foot(&sampled(frame as f32 * DEFAULT_WALK_START_SPEED / 60.0));
			largest_step = largest_step.max(previous.distance(pose));
			previous = pose;
		}
		assert!(largest_step < 0.09, "60 Hz foot movement jumped by {largest_step}");
		let epsilon = 1.0 / 240.0;
		assert!(foot(&sampled(0.0)).distance(foot(&sampled(epsilon))) < 0.005);
		assert!(foot(&sampled(1.0 - epsilon)).distance(foot(&sampled(1.0))) < 0.005);
		Ok(())
	}

	#[test]
	fn resampling_scaled_fk_scratch_and_root_effects_are_safe() -> anyhow::Result<()> {
		let mut rest_seeded = HumanoidV0Rig::for_clip_test();
		rest_seeded.seed_rest("femur.L", Transform::from_translation(Vec3::Y * 0.8));
		rest_seeded.seed_rest("shin.L", Transform::from_translation(Vec3::Y * 0.65));
		let mut direct = rest_seeded.clone();
		let clip = WalkStart::default();
		for progress in [0.8, 0.0, 0.35, 1.0, 0.35] {
			clip.apply(&mut rest_seeded, progress);
			assert_eq!(rest_seeded.scratch.depth, 0);
			assert!(rest_seeded.pose.local.iter().all(|t| t.translation.is_finite()
				&& t.rotation.is_finite()
				&& t.scale.is_finite()));
		}
		clip.apply(&mut direct, 0.35);
		assert_pose_near(&rest_seeded, &direct, 1e-5)?;
		assert_eq!(clip.effects_for(&direct, 0.35), ArmatureOffset::IDENTITY);
		let mut torso = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut torso, 0.5);
		let upper_back = torso
			.binding
			.definition
			.id("upper_back")
			.ok_or_else(|| anyhow::anyhow!("upper_back missing"))?;
		let posed = torso.pose.rotation(upper_back);
		let rest = torso.binding.effective_rest.rotation(upper_back);
		if posed.dot(rest).abs() >= 1.0 - 1e-6 {
			anyhow::bail!("walk-start should engage the authored upper-back counter-motion");
		}
		Ok(())
	}

	#[test]
	fn visible_pose_transition_preserves_start_and_reaches_target() -> anyhow::Result<()> {
		let mut visible = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut visible, 0.31);
		let captured = visible.pose.clone();
		let transition = Transition::from_visible(
			WalkStart::default(),
			captured.clone(),
			ArmatureOffset::IDENTITY,
		);
		let zero_effects = transition.apply(&mut visible, 0.6, 0.0);
		assert_eq!(visible.pose, captured);
		assert_eq!(zero_effects, ArmatureOffset::IDENTITY);
		let mut full = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut full, 0.6, 1.0);
		let mut target = HumanoidV0Rig::for_clip_test();
		WalkStart::default().apply(&mut target, 0.6);
		assert_pose_near(&full, &target, 1e-6)?;
		let from = full.pose.clone();
		let interrupt =
			Transition::from_visible(Idle::default(), from.clone(), ArmatureOffset::IDENTITY);
		interrupt.apply(&mut full, 0.0, 0.0);
		assert_eq!(full.pose, from, "interrupt begins at latest visible pose");
		Ok(())
	}
}
