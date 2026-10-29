use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::WalkStop;
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl WalkStop {
	/// First walk contact pose at cycle phase zero.
	pub fn walk_pose(&self) -> HumanoidPose {
		self.walk.sample_pose(0.0)
	}

	/// Neutral idle pose at progress zero.
	pub fn idle_pose(&self) -> HumanoidPose {
		self.idle.sample_pose(0.0)
	}
}

impl Animation<HumanoidV0Rig> for WalkStop {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		blend_clips(rig, &self.walk, 0.0, &self.idle, 0.0, self.weight(progress));
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use crate::animations::DEFAULT_WALK_STOP_SPEED;
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
		WalkStop::default().apply(&mut rig, progress);
		rig
	}

	#[test]
	fn endpoint_poses_clamp_and_match_walk_and_idle() -> anyhow::Result<()> {
		let mut start = HumanoidV0Rig::for_clip_test();
		let mut walk = HumanoidV0Rig::for_clip_test();
		WalkStop::default().apply(&mut start, 0.0);
		Walk::default().apply(&mut walk, 0.0);
		assert_pose_near(&start, &walk, 1e-6)?;
		let mut below = HumanoidV0Rig::for_clip_test();
		WalkStop::default().apply(&mut below, -0.3);
		assert_pose_near(&below, &walk, 1e-6)?;

		let mut end = HumanoidV0Rig::for_clip_test();
		let mut idle = HumanoidV0Rig::for_clip_test();
		WalkStop::default().apply(&mut end, 1.0);
		Idle::default().apply(&mut idle, 0.0);
		assert_pose_near(&end, &idle, 1e-6)?;
		let mut above = HumanoidV0Rig::for_clip_test();
		WalkStop::default().apply(&mut above, 1.2);
		assert_pose_near(&above, &idle, 1e-6)?;
		Ok(())
	}

	#[test]
	fn production_bind_foot_moves_continuously_at_sixty_hz() -> anyhow::Result<()> {
		let start = foot(&sampled(0.0));
		let end = foot(&sampled(1.0));
		assert!(start.distance(end) > 0.025, "settle must move the actual foot endpoint");
		let mut previous = start;
		let mut largest_step = 0.0_f32;
		for frame in 1..=17 {
			let pose = foot(&sampled(frame as f32 * DEFAULT_WALK_STOP_SPEED / 60.0));
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
		let mut changed = HumanoidV0Rig::for_clip_test();
		let default_lengths = changed.segment_lengths;
		changed.seed_rest("femur.L", Transform::from_translation(Vec3::Y * 0.8));
		changed.seed_rest("shin.L", Transform::from_translation(Vec3::Y * 0.65));
		if changed.segment_lengths.femur <= default_lengths.femur + 0.1 {
			anyhow::bail!("fixture must use a longer measured femur segment");
		}
		let mut direct = changed.clone();
		let clip = WalkStop::default();
		for progress in [0.8, 0.0, 0.4, 1.0, 0.4] {
			clip.apply(&mut changed, progress);
			assert_eq!(changed.scratch.depth, 0);
			assert!(changed.pose.local.iter().all(|t| t.translation.is_finite()
				&& t.rotation.is_finite()
				&& t.scale.is_finite()));
		}
		clip.apply(&mut direct, 0.4);
		assert_pose_near(&changed, &direct, 1e-5)?;
		let mut default_rig = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut default_rig, 0.4);
		if foot(&changed).distance(foot(&default_rig)) <= 0.05 {
			anyhow::bail!("walk-stop foot endpoint should respond to changed limb lengths");
		}
		assert_eq!(clip.effects_for(&direct, 0.4), ArmatureOffset::IDENTITY);
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
			anyhow::bail!("walk-to-idle must retain authored upper-back counter-motion");
		}
		Ok(())
	}

	#[test]
	fn visible_pose_transition_preserves_start_and_reaches_target() -> anyhow::Result<()> {
		let mut visible = HumanoidV0Rig::for_clip_test();
		Walk::default().apply(&mut visible, 0.37);
		let captured = visible.pose.clone();
		let transition = Transition::from_visible(
			WalkStop::default(),
			captured.clone(),
			ArmatureOffset::IDENTITY,
		);
		transition.apply(&mut visible, 0.5, 0.0);
		assert_eq!(visible.pose, captured);
		let mut full = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut full, 0.5, 1.0);
		let mut target = HumanoidV0Rig::for_clip_test();
		WalkStop::default().apply(&mut target, 0.5);
		assert_pose_near(&full, &target, 1e-6)?;
		let from = full.pose.clone();
		Transition::from_visible(Idle::default(), from.clone(), ArmatureOffset::IDENTITY)
			.apply(&mut full, 0.0, 0.0);
		assert_eq!(full.pose, from, "interrupt begins at latest visible pose");
		Ok(())
	}
}
