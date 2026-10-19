//! Quadruped clips share the indexed-rig blend in [`crate::rigs::mix`].

pub(crate) use crate::rigs::mix::blend_clips;

#[cfg(test)]
mod tests {
	use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

	use super::blend_clips;
	use crate::animations::{Gallop, QuadrupedIdle, QuadrupedRun, QuadrupedRunToGallop};
	use crate::Animation;

	/// Copy of the pre-shared quadruped `blend_poses` body from
	/// [#1313](https://github.com/ramate-io/maybraid/pull/1313) / [#1317](https://github.com/ramate-io/maybraid/pull/1317).
	fn blend_poses_legacy<A, B>(
		rig: &mut QuadrupedV0Rig,
		from: &A,
		to: &B,
		from_progress: f32,
		to_progress: f32,
		weight: f32,
	) where
		A: Animation<QuadrupedV0Rig>,
		B: Animation<QuadrupedV0Rig>,
	{
		let weight = weight.clamp(0.0, 1.0);
		if weight <= 0.0 {
			from.apply_for(rig, from_progress);
			return;
		}
		if weight >= 1.0 {
			to.apply_for(rig, to_progress);
			return;
		}

		let depth = rig.scratch.depth;
		rig.scratch.depth = depth + 1;
		from.apply_for(rig, from_progress);
		rig.scratch.capture_from(depth, &rig.pose);
		to.apply_for(rig, to_progress);
		rig.scratch.capture_to(depth, &rig.pose);
		rig.scratch.blend_saved(depth, weight, &mut rig.pose);
		rig.scratch.depth = depth;
	}

	fn assert_pose_near(a: &QuadrupedV0Rig, b: &QuadrupedV0Rig, label: &str) {
		assert_eq!(a.pose.len(), b.pose.len(), "{label}: pose length");
		for (index, (left, right)) in a.pose.local.iter().zip(b.pose.local.iter()).enumerate() {
			assert!(
				(left.translation - right.translation).length() < 1e-5,
				"{label}: translation {index}"
			);
			assert!(
				left.rotation.dot(right.rotation).abs() > 1.0 - 1e-5,
				"{label}: rotation {index}"
			);
			assert!((left.scale - right.scale).length() < 1e-5, "{label}: scale {index}");
		}
	}

	#[test]
	fn shared_blend_matches_legacy_quadruped_body() -> anyhow::Result<()> {
		let run = QuadrupedRun::default();
		let gallop = Gallop::default();
		let cases = [
			(0.0, 0.25, 0.75, 0.0),
			(0.0, 0.25, 0.75, 1.0),
			(0.25, 0.5, 0.75, 0.5),
			(0.1, 0.9, 0.35, 0.35),
		];
		for (from_p, to_p, _, weight) in cases {
			let mut legacy = QuadrupedV0Rig::for_clip_test();
			blend_poses_legacy(&mut legacy, &run, &gallop, from_p, to_p, weight);
			let mut shared = QuadrupedV0Rig::for_clip_test();
			blend_clips(&mut shared, &run, from_p, &gallop, to_p, weight);
			assert_pose_near(&legacy, &shared, &format!("weight={weight}"));
		}
		Ok(())
	}

	#[test]
	fn endpoint_fast_paths_match_direct_clip_apply() -> anyhow::Result<()> {
		let run = QuadrupedRun::default();
		let gallop = Gallop::default();

		let mut from_only = QuadrupedV0Rig::for_clip_test();
		run.apply_for(&mut from_only, 0.3);
		let mut weight_zero = QuadrupedV0Rig::for_clip_test();
		blend_clips(&mut weight_zero, &run, 0.3, &gallop, 0.8, 0.0);
		assert_pose_near(&from_only, &weight_zero, "weight 0");

		let mut to_only = QuadrupedV0Rig::for_clip_test();
		gallop.apply_for(&mut to_only, 0.8);
		let mut weight_one = QuadrupedV0Rig::for_clip_test();
		blend_clips(&mut weight_one, &run, 0.3, &gallop, 0.8, 1.0);
		assert_pose_near(&to_only, &weight_one, "weight 1");
		Ok(())
	}

	#[test]
	fn nested_shift_blend_matches_legacy_scratch_depth() -> anyhow::Result<()> {
		let shift = QuadrupedRunToGallop::default();
		let idle = QuadrupedIdle::default();
		let mut legacy = QuadrupedV0Rig::for_clip_test();
		blend_poses_legacy(&mut legacy, &shift, &idle, 0.5, 0.0, 0.5);
		let mut shared = QuadrupedV0Rig::for_clip_test();
		blend_clips(&mut shared, &shift, 0.5, &idle, 0.0, 0.5);
		assert_pose_near(&legacy, &shared, "nested weight 0.5");
		Ok(())
	}
}
