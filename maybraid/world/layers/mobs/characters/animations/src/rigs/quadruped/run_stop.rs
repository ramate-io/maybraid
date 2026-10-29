use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

use crate::animations::QuadrupedRunStop;
use crate::rigs::quadruped::blend::blend_clips;
use crate::{Animation, Effects};

impl Animation<QuadrupedV0Rig> for QuadrupedRunStop {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		let weight = self.weight(progress);
		blend_clips(rig, &self.run, 0.0, &self.idle, 0.0, weight);
	}

	fn effects_for(&self, _rig: &QuadrupedV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::authoring::ArmatureOffset;

	use super::*;
	use crate::animations::{QuadrupedRun, Transition};

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
				.ok_or_else(|| anyhow::anyhow!("missing quadruped bone {name}"))?;
			let left = a.pose.local[id.index()];
			let right = b.pose.local[id.index()];
			assert!(
				left.translation.distance(right.translation) <= tolerance,
				"translation mismatch on {name}"
			);
			assert!(
				left.rotation.dot(right.rotation).abs() >= 1.0 - tolerance,
				"rotation mismatch on {name}"
			);
			assert!(left.scale.distance(right.scale) <= tolerance, "scale mismatch on {name}");
		}
		Ok(())
	}

	fn shin_tip(rig: &QuadrupedV0Rig, name: &str) -> Option<Vec3> {
		let id = rig.binding.definition.id(name)?;
		let origin = rig.binding.definition.translation_in_character(&rig.pose, id);
		let rotation = rig.binding.definition.rotation_in_character(&rig.pose, id);
		Some(origin + rotation * Vec3::Y * 0.5)
	}

	fn foot_positions(rig: &QuadrupedV0Rig) -> Option<[Vec3; 4]> {
		Some([
			shin_tip(rig, SHINS[0])?,
			shin_tip(rig, SHINS[1])?,
			shin_tip(rig, SHINS[2])?,
			shin_tip(rig, SHINS[3])?,
		])
	}

	fn sample(progress: f32) -> QuadrupedV0Rig {
		let mut rig = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStop::default().apply(&mut rig, progress);
		rig
	}

	fn max_foot_delta(a: &[Vec3; 4], b: &[Vec3; 4]) -> f32 {
		a.iter().zip(b).map(|(left, right)| left.distance(*right)).fold(0.0, f32::max)
	}

	#[test]
	fn endpoints_clamp_and_match_full_source_poses() -> anyhow::Result<()> {
		let stop = QuadrupedRunStop::default();
		let mut start = QuadrupedV0Rig::for_clip_test();
		let mut run = QuadrupedV0Rig::for_clip_test();
		stop.apply(&mut start, 0.0);
		stop.run.apply(&mut run, 0.0);
		assert_pose_near(&start, &run, 1e-6)?;

		let mut below = QuadrupedV0Rig::for_clip_test();
		stop.apply(&mut below, -0.5);
		assert_pose_near(&below, &run, 1e-6)?;

		let mut end = QuadrupedV0Rig::for_clip_test();
		let mut idle = QuadrupedV0Rig::for_clip_test();
		stop.apply(&mut end, 1.0);
		stop.idle.apply(&mut idle, 0.0);
		assert_pose_near(&end, &idle, 1e-6)?;

		let mut above = QuadrupedV0Rig::for_clip_test();
		stop.apply(&mut above, 1.5);
		assert_pose_near(&above, &idle, 1e-6)?;
		Ok(())
	}

	#[test]
	fn production_bind_feet_move_smoothly_at_sixty_hz_and_both_ends() -> anyhow::Result<()> {
		let mut previous = foot_positions(&sample(0.0))
			.ok_or_else(|| anyhow::anyhow!("missing start foot positions"))?;
		let first = previous;
		let mut largest_frame_delta = 0.0_f32;
		for frame in 1..=18 {
			let progress = frame as f32
				* crate::animations::quadruped_run_stop::DEFAULT_QUADRUPED_RUN_STOP_SPEED
				/ 60.0;
			let current_rig = sample(progress);
			let current = foot_positions(&current_rig)
				.ok_or_else(|| anyhow::anyhow!("missing foot positions at 60 Hz frame {frame}"))?;
			largest_frame_delta = largest_frame_delta.max(max_foot_delta(&previous, &current));
			previous = current;
		}
		let rest = foot_positions(&sample(1.0))
			.ok_or_else(|| anyhow::anyhow!("missing end foot positions"))?;
		assert!(max_foot_delta(&first, &rest) > 0.02, "run contact must move real feet from idle");
		assert!(largest_frame_delta < 0.16, "60 Hz foot step too large: {largest_frame_delta}");

		// A smooth short settle must converge as the temporal resolution increases.
		let mut finer_previous = first;
		let mut finer_max = 0.0_f32;
		for frame in 1..=36 {
			let current = foot_positions(&sample(
				frame as f32
					* crate::animations::quadruped_run_stop::DEFAULT_QUADRUPED_RUN_STOP_SPEED
					/ 120.0,
			))
			.ok_or_else(|| anyhow::anyhow!("missing feet at 120 Hz frame {frame}"))?;
			finer_max = finer_max.max(max_foot_delta(&finer_previous, &current));
			finer_previous = current;
		}
		assert!(
			finer_max < largest_frame_delta * 0.6,
			"refining time should reduce each foot displacement: {finer_max} vs {largest_frame_delta}"
		);

		let epsilon = 1.0 / 240.0;
		for (edge, left, right) in
			[("start", sample(0.0), sample(epsilon)), ("end", sample(1.0 - epsilon), sample(1.0))]
		{
			let left =
				foot_positions(&left).ok_or_else(|| anyhow::anyhow!("missing left {edge} feet"))?;
			let right = foot_positions(&right)
				.ok_or_else(|| anyhow::anyhow!("missing right {edge} feet"))?;
			assert!(max_foot_delta(&left, &right) < 0.01, "foot discontinuity near {edge}");
		}
		Ok(())
	}

	#[test]
	fn samples_preserve_rest_proportions_and_are_independent_of_order() -> anyhow::Result<()> {
		let stop = QuadrupedRunStop::default();
		let mut reordered = QuadrupedV0Rig::for_clip_test();
		for (index, transform) in reordered.binding.effective_rest.local.iter_mut().enumerate() {
			if index % 2 == 0 {
				transform.translation *= 1.25;
			}
		}
		reordered.binding.refresh_rest(reordered.binding.effective_rest.clone());
		let mut direct = reordered.clone();
		for progress in [0.75, 0.0, 0.35, 1.0, 0.35] {
			stop.apply(&mut reordered, progress);
			assert_eq!(reordered.scratch.depth, 0, "blend must restore scratch depth");
			for transform in &reordered.pose.local {
				assert!(transform.translation.is_finite());
				assert!(transform.rotation.is_finite());
				assert!(transform.scale.is_finite());
			}
		}
		stop.apply(&mut reordered, 0.35);
		stop.apply(&mut direct, 0.35);
		assert_eq!(reordered.scratch.depth, 0);
		assert_eq!(direct.scratch.depth, 0);
		assert_pose_near(&reordered, &direct, 1e-5)?;

		let mut untouched = QuadrupedV0Rig::for_clip_test();
		stop.apply(&mut untouched, 0.5);
		let upper_back = untouched
			.binding
			.definition
			.id("upper_back")
			.ok_or_else(|| anyhow::anyhow!("upper_back missing from quadruped definition"))?;
		assert_eq!(
			untouched.pose.get(upper_back),
			untouched.binding.effective_rest.get(upper_back),
			"unwritten upper back must remain at effective rest"
		);
		let effects = stop.effects_for(&untouched, 0.5);
		assert_eq!(effects, ArmatureOffset::IDENTITY);
		Ok(())
	}

	#[test]
	fn transition_captures_visible_pose_at_zero_and_reaches_clip_at_one() -> anyhow::Result<()> {
		let stop = QuadrupedRunStop::default();
		let mut source = QuadrupedV0Rig::for_clip_test();
		QuadrupedRun::default().apply(&mut source, 0.25);
		let captured = source.pose.clone();
		let offset = ArmatureOffset::from_translation(Vec3::new(0.0, -0.1, 0.0));
		let transition = Transition::from_visible(stop, captured.clone(), offset);

		let mut at_zero = QuadrupedV0Rig::for_clip_test();
		let zero_effects = transition.apply_quadruped(&mut at_zero, 0.6, 0.0);
		assert_eq!(at_zero.pose, captured, "transition start must preserve the visible pose");
		assert_eq!(zero_effects, offset);

		let mut at_one = QuadrupedV0Rig::for_clip_test();
		let one_effects = transition.apply_quadruped(&mut at_one, 0.6, 1.0);
		let mut target = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStop::default().apply(&mut target, 0.6);
		assert_pose_near(&at_one, &target, 1e-6)?;
		assert_eq!(one_effects, ArmatureOffset::IDENTITY);

		let interrupted_pose = at_one.pose.clone();
		let interrupt = Transition::from_visible(
			QuadrupedRun::default(),
			interrupted_pose.clone(),
			ArmatureOffset::IDENTITY,
		);
		let mut interrupted = QuadrupedV0Rig::for_clip_test();
		interrupt.apply_quadruped(&mut interrupted, 0.0, 0.0);
		assert_eq!(
			interrupted.pose, interrupted_pose,
			"interrupt start must keep latest visible pose"
		);
		Ok(())
	}
}
