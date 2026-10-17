use std::f32::consts::TAU;

use character_rigs::authoring::QuadrupedPose;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;
use character_rigs::Side;

use crate::animations::QuadrupedLieDown;
use crate::rigs::quadruped::apply::{
	apply_front_leg, apply_hind_leg, apply_neck_axes, apply_spine,
};
use crate::{Animation, Effects};

impl Animation<QuadrupedV0Rig> for QuadrupedLieDown {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		rig.write_pose(&sample(self, progress));
	}

	fn effects_for(&self, _rig: &QuadrupedV0Rig, _progress: f32) -> Effects {
		// Fold lives on the bones. Gameplay locomotion is not this offset.
		Effects::IDENTITY
	}
}

fn sample(lie: &QuadrupedLieDown, progress: f32) -> QuadrupedPose {
	let mut pose = QuadrupedPose::default();
	let settle = lie.settle_weight(progress);
	let curl = lie.curl.clamp(0.0, 1.0) * settle;
	let breath = if lie.breath.abs() > f32::EPSILON {
		(TAU * (progress * QuadrupedLieDown::BREATH_FREQ)).sin()
			* lie.breath
			* lie.breath_weight(progress)
	} else {
		0.0
	};

	apply_neck_axes(
		&mut pose,
		-curl * 0.28 + breath * 0.15,
		curl * 0.18,
		-lie.neck_rest * settle + breath * 0.35,
	);
	apply_spine(&mut pose, curl * 0.12, lie.spine_settle * 0.18 * settle + curl * 0.55);
	pose.spine_sagittal = lie.spine_settle * settle + breath;

	for side in [Side::Left, Side::Right] {
		let curl_side = match side {
			Side::Left => 1.0 - curl * 0.35,
			Side::Right => 1.0 + curl * 0.20,
		};
		apply_front_leg(
			&mut pose,
			side,
			0.0,
			curl * 0.08 * side.sign(),
			lie.front_reach * settle,
			lie.front_hinge * settle,
		);
		apply_hind_leg(
			&mut pose,
			side,
			0.0,
			lie.hip_splay * settle * side.sign() * curl_side,
			lie.hind_tuck * settle * curl_side,
			lie.hind_hinge * settle,
		);
	}
	pose
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::{QuadrupedIdle, QuadrupedSit, Transition};
	use crate::interval_seconds;
	use crate::DEFAULT_SAMPLE_INTERVAL_US;
	use character_rigs::authoring::{ArmatureOffset, QUADRUPED_GLB_ANTERIOR_MID_BACK};
	use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

	const FRAME_DT: f32 = 1.0 / 60.0;

	fn front_shin_visual(rig: &QuadrupedV0Rig) -> bevy::prelude::Vec3 {
		use bevy::prelude::Vec3;
		QUADRUPED_GLB_ANTERIOR_MID_BACK
			* rig.rotation("shoulder.L")
			* rig.rotation("anterior_thigh.L")
			* rig.rotation("anterior_shin.L")
			* Vec3::Y
	}

	fn pose_at(lie: &QuadrupedLieDown, progress: f32) -> (QuadrupedV0Rig, QuadrupedPose) {
		let pose = sample(lie, progress);
		let mut rig = QuadrupedV0Rig::for_clip_test();
		rig.write_pose(&pose);
		(rig, pose)
	}

	#[test]
	fn sphinx_reaches_the_forelegs_and_settles_the_spine() -> anyhow::Result<()> {
		let lie = QuadrupedLieDown::held();
		let (rig, pose) = pose_at(&lie, 1.0);
		let rest = QuadrupedV0Rig::for_clip_test();

		let front_shin = rig.posed_angle("anterior_shin.L");
		let hind_shin = rig.posed_angle("posterior_shin.L");
		let sit = {
			let mut sit_rig = QuadrupedV0Rig::for_clip_test();
			QuadrupedSit::held().apply(&mut sit_rig, 1.0);
			sit_rig.posed_angle("anterior_shin.L")
		};
		assert!(
			front_shin > sit + 0.25,
			"sphinx should fold the elbows more than a sit, lie {front_shin} sit {sit}"
		);
		assert!(hind_shin > 0.45, "haunches should settle, hind shin {hind_shin}");
		assert!(pose.spine_sagittal > 0.3, "spine should flatten, {}", pose.spine_sagittal);
		assert!(pose.neck_nod < 0.0, "resting neck should nod down, {}", pose.neck_nod);
		assert!(pose.neck_turn.abs() < 0.05, "sphinx should not yaw, {}", pose.neck_turn);

		let visual = front_shin_visual(&rig);
		let rest_visual = front_shin_visual(&rest);
		assert!(
			visual.z < rest_visual.z - 0.08,
			"front hinge should fold back, {visual:?} vs {rest_visual:?}"
		);
		assert!(visual.x.abs() < 0.16, "sphinx reach stays sagittal, {visual:?}");

		let effects = lie.apply(&mut QuadrupedV0Rig::for_clip_test(), 1.0);
		if !effects.is_identity() {
			return Err(anyhow::anyhow!("lie must not offset the armature"));
		}
		Ok(())
	}

	#[test]
	fn posed_joint_evidence_on_clip_test_rig() -> anyhow::Result<()> {
		let rest = QuadrupedV0Rig::for_clip_test();
		let (sphinx, _) = pose_at(&QuadrupedLieDown::held(), 1.0);
		let (curled, _) = pose_at(&QuadrupedLieDown::curled(), 1.0);
		eprintln!(
			"quadruped lie held@1 posed_angle rest→sphinx→curl: front_shin.L {:.4}→{:.4}→{:.4} hind_shin.L {:.4}→{:.4}→{:.4} neck {:.4}→{:.4}→{:.4} lumbar {:.4}→{:.4}→{:.4}",
			rest.posed_angle("anterior_shin.L"),
			sphinx.posed_angle("anterior_shin.L"),
			curled.posed_angle("anterior_shin.L"),
			rest.posed_angle("posterior_shin.L"),
			sphinx.posed_angle("posterior_shin.L"),
			curled.posed_angle("posterior_shin.L"),
			rest.posed_angle("neck"),
			sphinx.posed_angle("neck"),
			curled.posed_angle("neck"),
			rest.posed_angle("lumbar"),
			sphinx.posed_angle("lumbar"),
			curled.posed_angle("lumbar"),
		);
		assert!(sphinx.posed_angle("anterior_shin.L") > 0.6);
		Ok(())
	}

	#[test]
	fn curled_rest_yaws_toward_the_haunch() -> anyhow::Result<()> {
		let sphinx = sample(&QuadrupedLieDown::held(), 1.0);
		let curled = sample(&QuadrupedLieDown::curled(), 1.0);
		assert!(curled.spine_lateral > sphinx.spine_lateral + 0.1);
		assert!(curled.neck_turn < sphinx.neck_turn - 0.05);
		assert!(curled.hind[0].proximal_lateral.abs() < curled.hind[1].proximal_lateral.abs());
		Ok(())
	}

	#[test]
	fn rest_breathing_is_visible_across_real_frames() -> anyhow::Result<()> {
		let lie = QuadrupedLieDown::for_rest();
		let mut sagittals = Vec::new();
		let mut t = 0.0;
		while t <= 1.2 + FRAME_DT * 0.5 {
			let (_, pose) = pose_at(&lie, t);
			sagittals.push(pose.spine_sagittal);
			t += FRAME_DT;
		}
		let min = sagittals.iter().copied().fold(f32::MAX, f32::min);
		let max = sagittals.iter().copied().fold(f32::MIN, f32::max);
		assert!(max - min > 0.03, "breath should move the spine at 60 Hz, min {min} max {max}");
		Ok(())
	}

	#[test]
	fn cycle_samples_lie_down_and_get_up_at_frame_intervals() -> anyhow::Result<()> {
		let lie = QuadrupedLieDown::for_cycle();
		let cache_dt = interval_seconds(DEFAULT_SAMPLE_INTERVAL_US);
		assert!((cache_dt - 0.01).abs() < 1e-7);

		let mut start_front = 0.0;
		let mut peak_front: f32 = 0.0;
		let mut end_front = 0.0;
		let mut t = 0.0;
		while t <= QuadrupedLieDown::PERIOD + FRAME_DT * 0.5 {
			let (rig, _) = pose_at(&lie, t);
			let front = rig.posed_angle("anterior_shin.L");
			if t <= FRAME_DT {
				start_front = front;
			}
			if t >= QuadrupedLieDown::DOWN_END && t <= QuadrupedLieDown::HOLD_END {
				peak_front = peak_front.max(front);
			}
			if t + FRAME_DT > QuadrupedLieDown::PERIOD {
				end_front = front;
			}
			t += FRAME_DT;
		}
		assert!(start_front < 0.08, "cycle should start standing, {start_front}");
		assert!(peak_front > 0.5, "cycle hold should lie down, {peak_front}");
		assert!(end_front < 0.08, "cycle should finish standing, {end_front}");
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let lie = QuadrupedLieDown::for_rest();
		let mut once = QuadrupedV0Rig::for_clip_test();
		let mut twice = QuadrupedV0Rig::for_clip_test();
		lie.apply(&mut once, 0.37);
		lie.apply(&mut twice, 0.37);
		lie.apply(&mut twice, 0.37);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn transition_from_visible_covers_lie_down_and_get_up() -> anyhow::Result<()> {
		let mut from = QuadrupedV0Rig::for_clip_test();
		QuadrupedIdle::default().apply(&mut from, 0.0);
		let enter = Transition::from_visible(
			QuadrupedLieDown::held(),
			from.pose.clone(),
			ArmatureOffset::IDENTITY,
		);

		let mut mid = QuadrupedV0Rig::for_clip_test();
		enter.apply_quadruped(&mut mid, 1.0, 0.5);
		let mid_front = mid.posed_angle("anterior_shin.L");

		let mut full = QuadrupedV0Rig::for_clip_test();
		QuadrupedLieDown::held().apply(&mut full, 1.0);
		let full_front = full.posed_angle("anterior_shin.L");
		assert!(
			mid_front > 0.08 && mid_front < full_front - 0.05,
			"lie-down blend should be between idle and rest, mid {mid_front} full {full_front}"
		);

		let get_up = Transition::from_visible(
			QuadrupedIdle::default(),
			full.pose.clone(),
			ArmatureOffset::IDENTITY,
		);
		let mut out = QuadrupedV0Rig::for_clip_test();
		get_up.apply_quadruped(&mut out, 0.0, 1.0);
		assert!(out.posed_angle("anterior_shin.L") < 0.12, "get-up at weight 1 should match idle");
		Ok(())
	}
}
