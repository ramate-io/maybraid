use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

use crate::animations::QuadrupedRunToGallop;
use crate::rigs::quadruped::blend::blend_clips;
use crate::{Animation, Effects};

impl Animation<QuadrupedV0Rig> for QuadrupedRunToGallop {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		blend_clips(
			rig,
			&self.run,
			self.run_phase_at_progress_for(progress),
			&self.gallop,
			self.gallop_phase_at_progress_for(progress),
			self.weight(progress),
		);
	}

	fn effects_for(&self, _rig: &QuadrupedV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use crate::animations::{Gallop, QuadrupedRun};
	use bevy::prelude::Vec3;
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

	use super::*;
	use crate::animations::{Transition, DEFAULT_QUADRUPED_RUN_TO_GALLOP_SPEED};

	const SHINS: [&str; 4] =
		["anterior_shin.L", "anterior_shin.R", "posterior_shin.L", "posterior_shin.R"];

	fn assert_pose_near(
		a: &QuadrupedV0Rig,
		b: &QuadrupedV0Rig,
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

	fn foot_tip(rig: &QuadrupedV0Rig, name: &str) -> Option<Vec3> {
		let id = rig.binding.definition.id(name)?;
		Some(
			rig.binding.definition.translation_in_character(&rig.pose, id)
				+ rig.binding.definition.rotation_in_character(&rig.pose, id)
					* Vec3::Y * rig.segment_lengths.lower,
		)
	}

	fn feet(rig: &QuadrupedV0Rig) -> Option<[Vec3; 4]> {
		Some([
			foot_tip(rig, SHINS[0])?,
			foot_tip(rig, SHINS[1])?,
			foot_tip(rig, SHINS[2])?,
			foot_tip(rig, SHINS[3])?,
		])
	}

	fn sampled(progress: f32, phase: f32) -> QuadrupedV0Rig {
		let mut rig = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunToGallop { gait_phase: phase, ..QuadrupedRunToGallop::default() }
			.apply(&mut rig, progress);
		rig
	}

	fn max_delta(a: &[Vec3; 4], b: &[Vec3; 4]) -> f32 {
		a.iter().zip(b).map(|(left, right)| left.distance(*right)).fold(0.0, f32::max)
	}

	fn largest_foot_step(gait: f32, sample_rate: u32) -> anyhow::Result<f32> {
		let shift = QuadrupedRunToGallop { gait_phase: gait, ..QuadrupedRunToGallop::default() };
		let duration = 1.0 / DEFAULT_QUADRUPED_RUN_TO_GALLOP_SPEED;
		let frames = (duration * sample_rate as f32).ceil() as u32;
		let mut previous = feet(&sampled(0.0, gait))
			.ok_or_else(|| anyhow::anyhow!("missing feet at transition start"))?;
		let mut largest = 0.0_f32;
		for frame in 1..=frames {
			let elapsed = frame as f32 / sample_rate as f32;
			let progress = (elapsed * DEFAULT_QUADRUPED_RUN_TO_GALLOP_SPEED).min(1.0);
			let current = feet(&sampled(progress, gait))
				.ok_or_else(|| anyhow::anyhow!("missing feet at transition sample"))?;
			largest = largest.max(max_delta(&previous, &current));
			previous = current;
		}
		let end = feet(&sampled(1.0, shift.gait_phase()))
			.ok_or_else(|| anyhow::anyhow!("missing feet at transition end"))?;
		if max_delta(&previous, &end) > 1e-5 {
			anyhow::bail!("sample sequence must reach the exact gallop handoff");
		}
		Ok(largest)
	}

	#[test]
	fn full_endpoint_poses_match_run_and_phase_advanced_gallop() -> anyhow::Result<()> {
		let shift = QuadrupedRunToGallop { gait_phase: 0.28, ..QuadrupedRunToGallop::default() };
		let mut start = QuadrupedV0Rig::for_clip_test();
		let mut run = QuadrupedV0Rig::for_clip_test();
		shift.apply(&mut start, 0.0);
		QuadrupedRun::default().apply(&mut run, shift.gait_phase());
		assert_pose_near(&start, &run, 1e-6)?;
		let mut end = QuadrupedV0Rig::for_clip_test();
		let mut gallop = QuadrupedV0Rig::for_clip_test();
		shift.apply(&mut end, 1.0);
		Gallop::default().apply(&mut gallop, shift.handoff_gallop_phase_for());
		assert_pose_near(&end, &gallop, 1e-6)?;
		assert!((shift.handoff_gallop_phase_for() - shift.gait_phase()).abs() > 0.02);
		let mut below = QuadrupedV0Rig::for_clip_test();
		shift.apply(&mut below, -0.4);
		assert_pose_near(&below, &run, 1e-6)?;
		let mut above = QuadrupedV0Rig::for_clip_test();
		shift.apply(&mut above, 1.4);
		assert_pose_near(&above, &gallop, 1e-6)?;
		Ok(())
	}

	#[test]
	fn production_bind_feet_move_smoothly_and_reach_loop_handoff() -> anyhow::Result<()> {
		let gait = 0.26;
		let shift = QuadrupedRunToGallop { gait_phase: gait, ..QuadrupedRunToGallop::default() };
		let start =
			feet(&sampled(0.0, gait)).ok_or_else(|| anyhow::anyhow!("missing start feet"))?;
		let end = feet(&sampled(1.0, gait)).ok_or_else(|| anyhow::anyhow!("missing end feet"))?;
		assert!(max_delta(&start, &end) > 0.025, "shift must move production-bind feet");
		let mut handoff = QuadrupedV0Rig::for_clip_test();
		Gallop::default().apply(&mut handoff, shift.handoff_gallop_phase_for());
		assert!(
			max_delta(&end, &feet(&handoff).ok_or_else(|| anyhow::anyhow!("missing gallop feet"))?)
				< 1e-5
		);
		let step_60 = largest_foot_step(gait, 60)?;
		let step_120 = largest_foot_step(gait, 120)?;
		let step_240 = largest_foot_step(gait, 240)?;
		assert!(step_60 > 0.01, "transition must move production-bind feet");
		let ratio_120 = step_120 / step_60;
		let ratio_240 = step_240 / step_120;
		assert!(
			(0.35..0.70).contains(&ratio_120) && (0.35..0.70).contains(&ratio_240),
			"max foot step should converge near 1/rate: 60={step_60:.5}, 120={step_120:.5}, 240={step_240:.5}, ratios={ratio_120:.3}/{ratio_240:.3}"
		);
		Ok(())
	}

	#[test]
	fn resampling_scaled_fk_scratch_depth_and_identity_effects() -> anyhow::Result<()> {
		let shift = QuadrupedRunToGallop { gait_phase: -0.32, ..QuadrupedRunToGallop::default() };
		assert!((shift.gait_phase() - 0.68).abs() < 1e-6);
		let mut changed = QuadrupedV0Rig::for_clip_test();
		for (index, transform) in changed.binding.effective_rest.local.iter_mut().enumerate() {
			if index % 2 == 0 {
				transform.translation *= 1.25;
			}
		}
		changed.binding.refresh_rest(changed.binding.effective_rest.clone());
		let mut direct = changed.clone();
		for progress in [0.7, 0.0, 0.4, 1.0, 0.4] {
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
	fn visible_pose_transition_and_interruption_preserve_captured_pose() -> anyhow::Result<()> {
		let mut visible = QuadrupedV0Rig::for_clip_test();
		QuadrupedRun::default().apply(&mut visible, 0.38);
		let captured = visible.pose.clone();
		let transition = Transition::from_visible(
			QuadrupedRunToGallop::default(),
			captured.clone(),
			ArmatureOffset::IDENTITY,
		);
		transition.apply_quadruped(&mut visible, 0.0, 0.0);
		assert_eq!(visible.pose, captured);
		let mut end = QuadrupedV0Rig::for_clip_test();
		transition.apply_quadruped(&mut end, 1.0, 1.0);
		let mut expected = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunToGallop::default().apply(&mut expected, 1.0);
		assert_pose_near(&end, &expected, 1e-6)?;
		let from = end.pose.clone();
		Transition::from_visible(QuadrupedRun::default(), from.clone(), ArmatureOffset::IDENTITY)
			.apply_quadruped(&mut end, 0.0, 0.0);
		assert_eq!(end.pose, from);
		Ok(())
	}
}
