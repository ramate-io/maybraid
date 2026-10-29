use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

use crate::animations::QuadrupedRunStart;
use crate::rigs::quadruped::blend::blend_clips;
use crate::{Animation, Effects};

impl Animation<QuadrupedV0Rig> for QuadrupedRunStart {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		blend_clips(
			rig,
			&self.idle,
			0.0,
			&self.run,
			Self::run_phase_at_progress(progress),
			self.weight(progress),
		);
	}

	fn effects_for(&self, _rig: &QuadrupedV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use crate::animations::DEFAULT_QUADRUPED_RUN_START_SPEED;
	use bevy::prelude::Vec3;
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

	use super::*;
	use crate::animations::{QuadrupedIdle, QuadrupedRun, Transition};

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

	fn sample(progress: f32) -> QuadrupedV0Rig {
		let mut rig = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStart::default().apply(&mut rig, progress);
		rig
	}

	fn max_delta(a: &[Vec3; 4], b: &[Vec3; 4]) -> f32 {
		a.iter().zip(b).map(|(left, right)| left.distance(*right)).fold(0.0, f32::max)
	}

	#[test]
	fn full_endpoint_poses_clamp_and_match_idle_and_phased_run() -> anyhow::Result<()> {
		let mut start = QuadrupedV0Rig::for_clip_test();
		let mut idle = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStart::default().apply(&mut start, 0.0);
		QuadrupedIdle::default().apply(&mut idle, 0.0);
		assert_pose_near(&start, &idle, 1e-6)?;
		let mut below = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStart::default().apply(&mut below, -0.2);
		assert_pose_near(&below, &idle, 1e-6)?;

		let phase = QuadrupedRunStart::handoff_run_phase();
		let mut end = QuadrupedV0Rig::for_clip_test();
		let mut run = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStart::default().apply(&mut end, 1.0);
		QuadrupedRun::default().apply(&mut run, phase);
		assert_pose_near(&end, &run, 1e-6)?;
		let mut above = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStart::default().apply(&mut above, 1.4);
		assert_pose_near(&above, &run, 1e-6)?;
		assert!(phase > 0.0);
		Ok(())
	}

	#[test]
	fn production_bind_feet_advance_smoothly_and_match_handoff() -> anyhow::Result<()> {
		let mut previous =
			feet(&sample(0.0)).ok_or_else(|| anyhow::anyhow!("missing start feet"))?;
		let start = previous;
		let mut largest_step = 0.0_f32;
		for frame in 1..=18 {
			let progress = frame as f32 * DEFAULT_QUADRUPED_RUN_START_SPEED / 60.0;
			let current = feet(&sample(progress)).ok_or_else(|| anyhow::anyhow!("missing feet"))?;
			largest_step = largest_step.max(max_delta(&previous, &current));
			previous = current;
		}
		let end = feet(&sample(1.0)).ok_or_else(|| anyhow::anyhow!("missing end feet"))?;
		assert!(max_delta(&start, &end) > 0.03, "launch must move production-bind feet");
		assert!(largest_step < 0.12, "60 Hz foot step too large: {largest_step}");
		let mut run = QuadrupedV0Rig::for_clip_test();
		QuadrupedRun::default().apply(&mut run, QuadrupedRunStart::handoff_run_phase());
		assert!(
			max_delta(&end, &feet(&run).ok_or_else(|| anyhow::anyhow!("missing run feet"))?) < 1e-5
		);
		let epsilon = 1.0 / 240.0;
		assert!(
			max_delta(
				&feet(&sample(0.0)).ok_or_else(|| anyhow::anyhow!("missing feet"))?,
				&feet(&sample(epsilon)).ok_or_else(|| anyhow::anyhow!("missing feet"))?
			) < 0.005
		);
		Ok(())
	}

	#[test]
	fn visible_pose_transition_resamples_without_accumulation_or_root_effects() -> anyhow::Result<()>
	{
		let mut visible = QuadrupedV0Rig::for_clip_test();
		QuadrupedIdle::default().apply(&mut visible, 0.25);
		let captured = visible.pose.clone();
		let transition = Transition::from_visible(
			QuadrupedRunStart::default(),
			captured.clone(),
			ArmatureOffset::IDENTITY,
		);
		transition.apply_quadruped(&mut visible, 0.5, 0.0);
		assert_eq!(visible.pose, captured);
		let mut full = QuadrupedV0Rig::for_clip_test();
		transition.apply_quadruped(&mut full, 0.5, 1.0);
		let mut expected = QuadrupedV0Rig::for_clip_test();
		QuadrupedRunStart::default().apply(&mut expected, 0.5);
		assert_pose_near(&full, &expected, 1e-6)?;
		let from = full.pose.clone();
		Transition::from_visible(QuadrupedIdle::default(), from.clone(), ArmatureOffset::IDENTITY)
			.apply_quadruped(&mut full, 0.0, 0.0);
		assert_eq!(full.pose, from, "interruption starts from current visible pose");

		let mut changed = QuadrupedV0Rig::for_clip_test();
		for (index, transform) in changed.binding.effective_rest.local.iter_mut().enumerate() {
			if index % 2 == 0 {
				transform.translation *= 1.2;
			}
		}
		changed.binding.refresh_rest(changed.binding.effective_rest.clone());
		let mut direct = changed.clone();
		let clip = QuadrupedRunStart::default();
		for progress in [0.75, 0.0, 0.4, 1.0, 0.4] {
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
}
