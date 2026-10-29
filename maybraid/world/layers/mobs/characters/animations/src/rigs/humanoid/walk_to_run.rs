use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::WalkToRun;
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for WalkToRun {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		blend_clips(
			rig,
			&self.walk,
			self.walk_phase_at_progress_for(progress),
			&self.run,
			self.run_phase_at_progress_for(progress),
			self.weight(progress),
		);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use crate::animations::{Run, Walk};
	use bevy::prelude::{Transform, Vec3};
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::animations::{Transition, DEFAULT_WALK_TO_RUN_SPEED};

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

	fn sampled(progress: f32, phase: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		WalkToRun { gait_phase: phase, ..WalkToRun::default() }.apply(&mut rig, progress);
		rig
	}

	fn largest_local_delta(
		from: &HumanoidV0Rig,
		to: &HumanoidV0Rig,
	) -> anyhow::Result<(String, f32)> {
		let mut largest = (String::new(), 0.0_f32);
		for name in from.animation_bone_names() {
			let id = from
				.binding
				.definition
				.id(name)
				.ok_or_else(|| anyhow::anyhow!("missing animation bone {name}"))?;
			let delta = from.pose.local[id.index()]
				.rotation
				.angle_between(to.pose.local[id.index()].rotation);
			if delta > largest.1 {
				largest = (name.to_owned(), delta);
			}
		}
		Ok(largest)
	}

	fn largest_foot_step(phase: f32, sample_rate: u32) -> anyhow::Result<(f32, f32, String, f32)> {
		let duration = 1.0 / DEFAULT_WALK_TO_RUN_SPEED;
		let frames = (duration * sample_rate as f32).ceil() as u32;
		let mut previous_rig = sampled(0.0, phase);
		let mut previous = foot(&previous_rig);
		let mut largest = 0.0_f32;
		let mut worst_progress = 0.0_f32;
		let mut worst_bone = String::new();
		let mut worst_bone_delta = 0.0_f32;
		for frame in 1..=frames {
			let elapsed = frame as f32 / sample_rate as f32;
			let progress = (elapsed * DEFAULT_WALK_TO_RUN_SPEED).min(1.0);
			let current_rig = sampled(progress, phase);
			let current = foot(&current_rig);
			let step = previous.distance(current);
			if step > largest {
				largest = step;
				worst_progress = progress;
				let (bone, delta) = largest_local_delta(&previous_rig, &current_rig)?;
				worst_bone = bone;
				worst_bone_delta = delta;
			}
			previous = current;
			previous_rig = current_rig;
		}
		if previous.distance(foot(&sampled(1.0, phase))) > 1e-5 {
			anyhow::bail!("sample sequence must reach the exact run handoff");
		}
		Ok((largest, worst_progress, worst_bone, worst_bone_delta))
	}

	#[test]
	fn full_handoff_poses_match_walk_and_advanced_run() -> anyhow::Result<()> {
		let shift = WalkToRun { gait_phase: 0.33, ..WalkToRun::default() };
		let mut start = HumanoidV0Rig::for_clip_test();
		let mut walk = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut start, 0.0);
		Walk::default().apply(&mut walk, shift.gait_phase());
		assert_pose_near(&start, &walk, 1e-6)?;
		let mut end = HumanoidV0Rig::for_clip_test();
		let mut run = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut end, 1.0);
		Run::default().apply(&mut run, shift.handoff_run_phase_for());
		assert_pose_near(&end, &run, 1e-6)?;
		assert!((shift.handoff_run_phase_for() - shift.gait_phase()).abs() > 0.05);
		let mut below = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut below, -0.5);
		assert_pose_near(&below, &walk, 1e-6)?;
		let mut above = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut above, 1.5);
		assert_pose_near(&above, &run, 1e-6)?;
		Ok(())
	}

	#[test]
	fn phase_advanced_foot_handoff_and_sixty_hz_motion_are_continuous() -> anyhow::Result<()> {
		let phase = 0.27;
		let shift = WalkToRun { gait_phase: phase, ..WalkToRun::default() };
		let start = foot(&sampled(0.0, phase));
		let end = foot(&sampled(1.0, phase));
		let run_handoff = {
			let mut rig = HumanoidV0Rig::for_clip_test();
			Run::default().apply(&mut rig, shift.handoff_run_phase_for());
			foot(&rig)
		};
		assert!(start.distance(end) > 0.02, "gait shift must move the foot");
		assert!(end.distance(run_handoff) < 1e-5, "end foot must meet the advanced run phase");
		let (step_60, _, _, _) = largest_foot_step(phase, 60)?;
		let (step_120, _, _, _) = largest_foot_step(phase, 120)?;
		let (step_240, progress_240, bone_240, angle_240) = largest_foot_step(phase, 240)?;
		let (step_480, progress_480, bone_480, angle_480) = largest_foot_step(phase, 480)?;
		let ratio_120 = step_120 / step_60;
		let ratio_240 = step_240 / step_120;
		let ratio_480 = step_480 / step_240;
		assert!(step_60 > 0.01, "transition must move the production-bind foot");
		assert!(
			(0.35..0.70).contains(&ratio_120) && (0.35..0.70).contains(&ratio_480),
			"max foot step should converge near 1/rate: 60={step_60:.5}, 120={step_120:.5}, 240={step_240:.5}, 480={step_480:.5}, ratios={ratio_120:.3}/{ratio_240:.3}/{ratio_480:.3}; worst 240Hz step at p={progress_240:.5} (walk={:.5}, run={:.5}), bone={bone_240}, localΔ={angle_240:.5}; worst 480Hz step at p={progress_480:.5} (walk={:.5}, run={:.5}), bone={bone_480}, localΔ={angle_480:.5}",
			shift.walk_phase_at_progress_for(progress_240),
			shift.run_phase_at_progress_for(progress_240),
			shift.walk_phase_at_progress_for(progress_480),
			shift.run_phase_at_progress_for(progress_480),
		);
		let epsilon = 1.0 / 240.0;
		let start_boundary = foot(&sampled(epsilon, phase)).distance(start);
		let end_boundary =
			foot(&sampled(1.0, phase)).distance(foot(&sampled(1.0 - epsilon, phase)));
		if start_boundary >= step_60 * 0.15 || end_boundary >= step_60 * 0.15 {
			anyhow::bail!("fine endpoint samples must converge to the clip handoffs");
		}
		Ok(())
	}

	#[test]
	fn phase_parameter_and_resampling_work_with_changed_proportions() -> anyhow::Result<()> {
		let shift = WalkToRun { gait_phase: -0.35, ..WalkToRun::default() };
		assert!((shift.gait_phase() - 0.65).abs() < 1e-6);
		let mut changed = HumanoidV0Rig::for_clip_test();
		changed.seed_rest("femur.L", Transform::from_translation(Vec3::Y * 0.8));
		changed.seed_rest("shin.L", Transform::from_translation(Vec3::Y * 0.65));
		let mut direct = changed.clone();
		for progress in [0.65, 0.0, 0.4, 1.0, 0.4] {
			shift.apply(&mut changed, progress);
			assert_eq!(changed.scratch.depth, 0);
			assert!(changed.pose.local.iter().all(|t| t.translation.is_finite()
				&& t.rotation.is_finite()
				&& t.scale.is_finite()));
		}
		shift.apply(&mut direct, 0.4);
		assert_pose_near(&changed, &direct, 1e-5)?;
		assert_eq!(shift.effects_for(&direct, 0.4), ArmatureOffset::IDENTITY);
		Ok(())
	}

	#[test]
	fn visible_pose_transition_preserves_entry_and_interrupts_without_pop() -> anyhow::Result<()> {
		let mut visible = HumanoidV0Rig::for_clip_test();
		Walk::default().apply(&mut visible, 0.42);
		let captured = visible.pose.clone();
		let transition = Transition::from_visible(
			WalkToRun::default(),
			captured.clone(),
			ArmatureOffset::IDENTITY,
		);
		transition.apply(&mut visible, 0.0, 0.0);
		assert_eq!(visible.pose, captured);
		let mut end = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut end, 1.0, 1.0);
		let mut expected = HumanoidV0Rig::for_clip_test();
		WalkToRun::default().apply(&mut expected, 1.0);
		assert_pose_near(&end, &expected, 1e-6)?;
		let from = end.pose.clone();
		Transition::from_visible(Run::default(), from.clone(), ArmatureOffset::IDENTITY)
			.apply(&mut end, 0.0, 0.0);
		assert_eq!(end.pose, from);
		Ok(())
	}
}
