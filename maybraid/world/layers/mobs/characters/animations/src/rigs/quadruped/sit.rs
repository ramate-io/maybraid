use std::f32::consts::TAU;

use character_rigs::authoring::QuadrupedPose;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;
use character_rigs::Side;

use crate::animations::QuadrupedSit;
use crate::rigs::quadruped::apply::{
	apply_front_leg, apply_hind_leg, apply_neck_axes, apply_spine,
};
use crate::{Animation, Effects};

impl Animation<QuadrupedV0Rig> for QuadrupedSit {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		rig.write_pose(&sample(self, progress));
	}

	fn effects_for(&self, _rig: &QuadrupedV0Rig, _progress: f32) -> Effects {
		// Fold lives on the bones. Gameplay locomotion is not this offset.
		Effects::IDENTITY
	}
}

fn sample(sit: &QuadrupedSit, progress: f32) -> QuadrupedPose {
	let mut pose = QuadrupedPose::default();
	let settle = sit.settle_weight(progress);
	let sway = if sit.sway.abs() > f32::EPSILON {
		(TAU * (progress * QuadrupedSit::SWAY_FREQ)).sin() * sit.sway
	} else {
		0.0
	};

	apply_neck_axes(&mut pose, sway * 0.45, sway * 0.55, sit.neck_alert * settle + sway * 0.25);
	apply_spine(&mut pose, sway * 0.35, sit.spine_gather * settle + sway * 0.4);
	pose.spine_sagittal = sit.spine_upright * settle;

	for side in [Side::Left, Side::Right] {
		apply_front_leg(
			&mut pose,
			side,
			0.0,
			0.0,
			-sit.front_plant * settle,
			sit.front_hinge * settle,
		);
		apply_hind_leg(
			&mut pose,
			side,
			0.0,
			sit.hip_splay * settle * side.sign(),
			sit.hind_stride * settle,
			sit.hind_hinge * settle,
		);
	}
	pose
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::{QuadrupedIdle, Transition};
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

	fn pose_at(sit: &QuadrupedSit, progress: f32) -> (QuadrupedV0Rig, QuadrupedPose) {
		let pose = sample(sit, progress);
		let mut rig = QuadrupedV0Rig::for_clip_test();
		rig.write_pose(&pose);
		(rig, pose)
	}

	#[test]
	fn held_sit_tucks_the_haunches_harder_than_the_forelegs() -> anyhow::Result<()> {
		let sit = QuadrupedSit::held();
		let (rig, pose) = pose_at(&sit, 1.0);

		let hind_thigh = rig.posed_angle("posterior_thigh.L");
		let hind_shin = rig.posed_angle("posterior_shin.L");
		let front_thigh = rig.posed_angle("anterior_thigh.L");
		let front_shin = rig.posed_angle("anterior_shin.L");
		assert!(
			hind_thigh > front_thigh + 0.35,
			"sit should tuck the haunch, hind thigh {hind_thigh} front {front_thigh}"
		);
		assert!(
			hind_shin > front_shin + 0.45,
			"sit should fold the hind shin, hind {hind_shin} front {front_shin}"
		);
		assert!(
			pose.spine_sagittal > 0.15,
			"chest should stay more upright, {}",
			pose.spine_sagittal
		);
		assert!(pose.neck_nod > 0.08, "neck should stay alert, {}", pose.neck_nod);
		assert!(
			(pose.hind[0].proximal_lateral + pose.hind[1].proximal_lateral).abs() < 1e-5,
			"haunch splay is symmetric"
		);

		let effects = sit.apply(&mut QuadrupedV0Rig::for_clip_test(), 1.0);
		if !effects.is_identity() {
			return Err(anyhow::anyhow!("sit must not offset the armature"));
		}
		Ok(())
	}

	#[test]
	fn sit_samples_at_real_frame_intervals() -> anyhow::Result<()> {
		let sit = QuadrupedSit::for_cycle();
		let cache_dt = interval_seconds(DEFAULT_SAMPLE_INTERVAL_US);
		assert!((cache_dt - 0.01).abs() < 1e-7);
		assert!((FRAME_DT - 1.0 / 60.0).abs() < 1e-7);

		let mut max_hind = 0.0_f32;
		let mut stand_hind = f32::MAX;
		let mut t = 0.0;
		while t <= QuadrupedSit::PERIOD + FRAME_DT * 0.5 {
			let (rig, _) = pose_at(&sit, t);
			let hind = rig.posed_angle("posterior_shin.L");
			if t <= QuadrupedSit::REST_END {
				stand_hind = stand_hind.min(hind);
			}
			max_hind = max_hind.max(hind);
			t += FRAME_DT;
		}
		assert!(max_hind > 0.55, "60 Hz cycle must reach a seated hind fold, max {max_hind}");
		assert!(stand_hind < 0.08, "60 Hz cycle should start near stand, stand hind {stand_hind}");

		let mut cache_t = 0.0;
		let mut cache_max = 0.0_f32;
		while cache_t <= QuadrupedSit::PERIOD + cache_dt * 0.5 {
			let (rig, _) = pose_at(&sit, cache_t);
			cache_max = cache_max.max(rig.posed_angle("posterior_shin.L"));
			cache_t += cache_dt;
		}
		assert!(cache_max > 0.55, "10 ms bins must also see the sit fold, max {cache_max}");
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let sit = QuadrupedSit::held();
		let mut once = QuadrupedV0Rig::for_clip_test();
		let mut twice = QuadrupedV0Rig::for_clip_test();
		sit.apply(&mut once, 0.6);
		sit.apply(&mut twice, 0.6);
		sit.apply(&mut twice, 0.6);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn transition_from_idle_enters_and_exits_the_sit() -> anyhow::Result<()> {
		let mut from = QuadrupedV0Rig::for_clip_test();
		QuadrupedIdle::default().apply(&mut from, 0.0);
		let from_pose = from.pose.clone();
		let sit = QuadrupedSit::held();
		let enter = Transition::from_visible(sit, from_pose, ArmatureOffset::IDENTITY);

		let mut mid = QuadrupedV0Rig::for_clip_test();
		enter.apply_quadruped(&mut mid, 1.0, 0.5);
		let mid_hind = mid.posed_angle("posterior_thigh.L");

		let mut full = QuadrupedV0Rig::for_clip_test();
		sit.apply(&mut full, 1.0);
		let full_hind = full.posed_angle("posterior_thigh.L");
		assert!(
			mid_hind > 0.08 && mid_hind < full_hind - 0.05,
			"entry blend should land between idle and sit, mid {mid_hind} full {full_hind}"
		);

		let exit = Transition::from_visible(
			QuadrupedIdle::default(),
			full.pose.clone(),
			ArmatureOffset::IDENTITY,
		);
		let mut out = QuadrupedV0Rig::for_clip_test();
		let effects = exit.apply_quadruped(&mut out, 0.0, 0.0);
		if !effects.is_identity() {
			return Err(anyhow::anyhow!("exit start must keep the captured identity offset"));
		}
		assert!(
			out.posed_angle("posterior_thigh.L") > full_hind - 1e-4,
			"weight 0 must keep the visible sit"
		);
		Ok(())
	}

	#[test]
	fn idle_sit_sways_without_leaving_the_haunch_fold() -> anyhow::Result<()> {
		let sit = QuadrupedSit::for_idle();
		let a = sample(&sit, 0.0);
		let b = sample(&sit, 0.25 / QuadrupedSit::SWAY_FREQ);
		assert!((a.hind[0].hinge - b.hind[0].hinge).abs() < 1e-5);
		assert!((a.neck_turn - b.neck_turn).abs() > 1e-4);
		Ok(())
	}

	#[test]
	fn posed_joint_evidence_on_clip_test_rig() -> anyhow::Result<()> {
		let rest = QuadrupedV0Rig::for_clip_test();
		let (sit, _) = pose_at(&QuadrupedSit::held(), 1.0);
		eprintln!(
			"quadruped sit held@1 posed_angle rest→sit: hind_thigh.L {:.4}→{:.4} hind_shin.L {:.4}→{:.4} front_thigh.L {:.4}→{:.4} front_shin.L {:.4}→{:.4} neck {:.4}→{:.4}",
			rest.posed_angle("posterior_thigh.L"),
			sit.posed_angle("posterior_thigh.L"),
			rest.posed_angle("posterior_shin.L"),
			sit.posed_angle("posterior_shin.L"),
			rest.posed_angle("anterior_thigh.L"),
			sit.posed_angle("anterior_thigh.L"),
			rest.posed_angle("anterior_shin.L"),
			sit.posed_angle("anterior_shin.L"),
			rest.posed_angle("neck"),
			sit.posed_angle("neck"),
		);
		assert!(sit.posed_angle("posterior_shin.L") > 0.7);
		Ok(())
	}

	#[test]
	fn front_chain_stays_more_planted_than_a_run_fold() -> anyhow::Result<()> {
		let rest = QuadrupedV0Rig::for_clip_test();
		let rest_visual = front_shin_visual(&rest);
		let (posed, _) = pose_at(&QuadrupedSit::held(), 1.0);
		let visual = front_shin_visual(&posed);
		assert!(
			(visual - rest_visual).length() < 0.55,
			"front plant should stay modest, {visual:?} vs {rest_visual:?}"
		);
		assert!(visual.x.abs() < 0.12, "front plant stays sagittal, {visual:?}");
		Ok(())
	}
}
