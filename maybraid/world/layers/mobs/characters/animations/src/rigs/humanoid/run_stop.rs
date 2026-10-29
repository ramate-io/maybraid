use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{RunStop, RunStopSegment};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for RunStop {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let phase = self.phase.rem_euclid(1.0);
		match self.segment(progress) {
			RunStopSegment::RunToWalk(weight) => {
				blend_clips(rig, &self.run, phase, &self.walk, phase, weight);
			}
			RunStopSegment::WalkToIdle(weight) => {
				blend_clips(rig, &self.walk, phase, &self.idle, 0.0, weight);
			}
		}
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use crate::animations::DEFAULT_RUN_STOP_SPEED;
	use bevy::prelude::{Transform, Vec3};
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::animations::{Idle, Run, Transition, Walk, RUN_STOP_HANDOFF_SPEED, RUN_TO_WALK_END};

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

	fn sampled(progress: f32, phase: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		RunStop { phase, ..RunStop::default() }.apply(&mut rig, progress);
		rig
	}

	#[test]
	fn endpoint_poses_match_run_phase_and_idle_and_clamp() -> anyhow::Result<()> {
		let phase = 0.37;
		let mut start = HumanoidV0Rig::for_clip_test();
		let mut run = HumanoidV0Rig::for_clip_test();
		RunStop { phase, ..RunStop::default() }.apply(&mut start, 0.0);
		Run::default().apply(&mut run, phase);
		assert_pose_near(&start, &run, 1e-6)?;
		let mut below = HumanoidV0Rig::for_clip_test();
		RunStop { phase, ..RunStop::default() }.apply(&mut below, -0.2);
		assert_pose_near(&below, &run, 1e-6)?;

		let mut end = HumanoidV0Rig::for_clip_test();
		let mut idle = HumanoidV0Rig::for_clip_test();
		RunStop { phase, ..RunStop::default() }.apply(&mut end, 1.0);
		Idle::default().apply(&mut idle, 0.0);
		assert_pose_near(&end, &idle, 1e-6)?;
		let mut above = HumanoidV0Rig::for_clip_test();
		RunStop { phase, ..RunStop::default() }.apply(&mut above, 1.3);
		assert_pose_near(&above, &idle, 1e-6)?;

		let mut coast = HumanoidV0Rig::for_clip_test();
		let mut walk = HumanoidV0Rig::for_clip_test();
		RunStop { phase, from_run: false, ..RunStop::default() }.apply(&mut coast, 0.0);
		Walk::default().apply(&mut walk, phase);
		assert_pose_near(&coast, &walk, 1e-6)?;
		Ok(())
	}

	#[test]
	fn run_to_walk_handoff_is_pose_continuous_at_the_segment_boundary() -> anyhow::Result<()> {
		let phase = 0.42;
		let mut left = HumanoidV0Rig::for_clip_test();
		let mut boundary = HumanoidV0Rig::for_clip_test();
		let mut right = HumanoidV0Rig::for_clip_test();
		let epsilon = 1.0 / 1000.0;
		let stop = RunStop { phase, ..RunStop::default() };
		stop.apply(&mut left, RUN_TO_WALK_END - epsilon);
		stop.apply(&mut boundary, RUN_TO_WALK_END);
		stop.apply(&mut right, RUN_TO_WALK_END + epsilon);
		assert_pose_near(&left, &boundary, 0.004)?;
		assert_pose_near(&boundary, &right, 0.004)?;
		let speed_at_handoff =
			crate::animations::speed_for_run_stop_progress(RUN_TO_WALK_END, 10.5);
		assert!((speed_at_handoff - RUN_STOP_HANDOFF_SPEED).abs() < 0.001);
		Ok(())
	}

	#[test]
	fn real_foot_converges_smoothly_and_speed_helpers_remain_finite() -> anyhow::Result<()> {
		let phase = 0.31;
		let start = foot(&sampled(0.0, phase));
		let end = foot(&sampled(1.0, phase));
		assert!(start.distance(end) > 0.04, "run stop must move the actual foot endpoint");
		let mut previous = start;
		let mut largest_step = 0.0_f32;
		for frame in 1..=100 {
			let progress = frame as f32 * DEFAULT_RUN_STOP_SPEED / 60.0;
			let current = foot(&sampled(progress, phase));
			largest_step = largest_step.max(previous.distance(current));
			previous = current;
		}
		assert!(largest_step < 0.13, "60 Hz foot movement jumped by {largest_step}");
		for progress in [0.0, 0.15, 0.35, RUN_TO_WALK_END, 0.8, 1.0] {
			let speed = crate::animations::speed_for_run_stop_progress(progress, 8.0);
			let mapped = crate::animations::run_stop_progress_from_speed(speed, 8.0);
			assert!((mapped - progress.min(RUN_TO_WALK_END)).abs() < 0.001);
			let cadence = crate::animations::run_stop_cycle_speed(speed.max(0.1), progress);
			assert!(cadence.is_finite() && cadence > 0.0);
		}
		Ok(())
	}

	#[test]
	fn changed_limb_lengths_nonaccumulation_scratch_and_identity_effects() -> anyhow::Result<()> {
		let mut changed = HumanoidV0Rig::for_clip_test();
		let default_lengths = changed.segment_lengths;
		changed.seed_rest("femur.L", Transform::from_translation(Vec3::Y * 0.8));
		changed.seed_rest("shin.L", Transform::from_translation(Vec3::Y * 0.65));
		if changed.segment_lengths.femur <= default_lengths.femur + 0.1 {
			anyhow::bail!("fixture must use a longer measured femur segment");
		}
		let mut direct = changed.clone();
		let clip = RunStop { phase: 0.42, ..RunStop::default() };
		for progress in [0.8, 0.0, 0.3, RUN_TO_WALK_END, 1.0, 0.3] {
			clip.apply(&mut changed, progress);
			assert_eq!(changed.scratch.depth, 0);
			assert!(changed.pose.local.iter().all(|t| t.translation.is_finite()
				&& t.rotation.is_finite()
				&& t.scale.is_finite()));
		}
		clip.apply(&mut direct, 0.3);
		assert_pose_near(&changed, &direct, 1e-5)?;
		let mut default_rig = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut default_rig, 0.3);
		if foot(&changed).distance(foot(&default_rig)) <= 0.05 {
			anyhow::bail!("run-stop foot endpoint should respond to changed limb lengths");
		}
		assert_eq!(clip.effects_for(&direct, 0.3), ArmatureOffset::IDENTITY);
		Ok(())
	}

	#[test]
	fn visible_pose_transition_and_interrupt_preserve_the_latest_pose() -> anyhow::Result<()> {
		let mut visible = HumanoidV0Rig::for_clip_test();
		Run::default().apply(&mut visible, 0.42);
		let captured = visible.pose.clone();
		let transition = Transition::from_visible(
			RunStop { phase: 0.42, ..RunStop::default() },
			captured.clone(),
			ArmatureOffset::IDENTITY,
		);
		transition.apply(&mut visible, 0.4, 0.0);
		assert_eq!(visible.pose, captured);
		let mut full = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut full, 0.4, 1.0);
		let mut target = HumanoidV0Rig::for_clip_test();
		RunStop { phase: 0.42, ..RunStop::default() }.apply(&mut target, 0.4);
		assert_pose_near(&full, &target, 1e-6)?;
		let from = full.pose.clone();
		Transition::from_visible(Walk::default(), from.clone(), ArmatureOffset::IDENTITY)
			.apply(&mut full, 0.0, 0.0);
		assert_eq!(full.pose, from, "interruption starts at latest visible pose");
		Ok(())
	}
}
