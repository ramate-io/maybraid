use bevy::prelude::Vec3;
use character_rigs::authoring::{ArmatureOffset, HumanoidPose, PoseBuffer};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::SquatDescent;
use crate::{Animation, Effects, Progress};

/// Rest-pose toe tip sits slightly forward of the ankle along character −Z.
const REST_TOE_Z_OFFSET: f32 = -0.08;

fn shin_tail_from_pose(rig: &HumanoidV0Rig, pose: &PoseBuffer, side: &str) -> Vec3 {
	let definition = &rig.binding.definition;
	let id = definition.id(side).expect("shin bone");
	let origin = definition.translation_in_character(pose, id);
	let along = definition.rotation_in_character(pose, id) * Vec3::Y;
	origin + along * rig.segment_lengths.shin
}

fn ankle_lift_drop(rig: &HumanoidV0Rig) -> f32 {
	let rest = &rig.binding.effective_rest;
	["shin.L", "shin.R"]
		.iter()
		.map(|side| {
			let posed = shin_tail_from_pose(rig, &rig.pose, side);
			let rest_tail = shin_tail_from_pose(rig, rest, side);
			(posed.y - rest_tail.y).max(0.0)
		})
		.fold(0.0f32, f32::max)
}

fn foot_tail(rig: &HumanoidV0Rig) -> (Vec3, Vec3) {
	let left = shin_tail_from_pose(rig, &rig.pose, "shin.L");
	let right = shin_tail_from_pose(rig, &rig.pose, "shin.R");
	(left, right)
}

fn toe_tail(rig: &HumanoidV0Rig) -> (Vec3, Vec3) {
	let (left, right) = foot_tail(rig);
	let offset = Vec3::new(0.0, 0.0, REST_TOE_Z_OFFSET);
	(left + offset, right + offset)
}

fn planted(point: Vec3, effects: Effects) -> Vec3 {
	point + effects.0.translation
}

impl Animation<HumanoidV0Rig> for SquatDescent {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		let femur_swing = self.femur_swing(progress);
		let shin_flex = self.shin_flex(progress);

		pose.apply_leg(Side::Left, femur_swing, shin_flex);
		pose.apply_leg(Side::Right, femur_swing, shin_flex);
		let hip = self.hip_fold(progress);
		if hip.abs() > f32::EPSILON {
			pose.apply_hip_fold(Side::Left, hip);
			pose.apply_hip_fold(Side::Right, hip);
		}
		pose.apply_spine_pitch(self.root_swing(progress));
		rig.write_pose(&pose);
	}

	fn effects_for(&self, rig: &HumanoidV0Rig, progress: f32) -> Effects {
		let progress = Progress(progress).clamp();
		if progress <= 0.0 || progress >= 1.0 {
			return Effects::IDENTITY;
		}
		let drop = ankle_lift_drop(rig);
		if drop > f32::EPSILON {
			ArmatureOffset::from_translation(Vec3::new(0.0, -drop, 0.0))
		} else {
			Effects::IDENTITY
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;
	use crate::animations::Squat;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn assert_planted_near(
		point: Vec3,
		effects: Effects,
		target: Vec3,
		label: &str,
	) -> anyhow::Result<()> {
		const VERT: f32 = 0.01;
		const HORIZ: f32 = 0.02;
		let world = planted(point, effects);
		let dy = world.y - target.y;
		let dx = (world.x - target.x).abs();
		anyhow::ensure!(
			dy.abs() <= VERT && dy >= -VERT,
			"{label} vertical drift {dy:.4} m (limit {VERT})"
		);
		anyhow::ensure!(dx <= HORIZ, "{label} lateral drift {dx:.4} m (limit {HORIZ})");
		Ok(())
	}

	#[test]
	fn feet_stay_planted_through_descent() -> anyhow::Result<()> {
		const SAMPLES: [f32; 7] = [0.0, 0.25, 0.4, 0.55, 0.7, 0.85, 1.0];
		let descent = SquatDescent::default();

		let mut stand = HumanoidV0Rig::for_clip_test();
		let stand_effects = descent.apply(&mut stand, 0.0);
		let (stand_fl, stand_fr) = foot_tail(&stand);
		let (stand_tl, stand_tr) = toe_tail(&stand);
		let stand_root = planted(stand.character_point("root"), stand_effects);

		let mut held_ref = HumanoidV0Rig::for_clip_test();
		let held_effects = Squat::held().apply(&mut held_ref, 1.0);
		let (held_fl, held_fr) = foot_tail(&held_ref);
		let (held_tl, held_tr) = toe_tail(&held_ref);
		let held_root = planted(held_ref.character_point("root"), held_effects);

		for progress in SAMPLES {
			let mut rig = HumanoidV0Rig::for_clip_test();
			let effects = descent.apply(&mut rig, progress);
			let (fl, fr) = foot_tail(&rig);
			let (tl, tr) = toe_tail(&rig);
			let root = planted(rig.character_point("root"), effects);
			if progress >= 1.0 - 1e-5 {
				assert!(effects.is_identity(), "held endpoint has no armature move");
				assert_planted_near(fl, Effects::IDENTITY, held_fl, "foot.L@1")?;
				assert_planted_near(fr, Effects::IDENTITY, held_fr, "foot.R@1")?;
				assert_planted_near(tl, Effects::IDENTITY, held_tl, "toe.L@1")?;
				assert_planted_near(tr, Effects::IDENTITY, held_tr, "toe.R@1")?;
				assert!((root.y - held_root.y).abs() < 1e-4, "pelvis height at full depth");
			} else if progress <= 1e-5 {
				assert!(effects.is_identity(), "upright start has no armature move");
				assert_planted_near(fl, Effects::IDENTITY, stand_fl, "foot.L@0")?;
				assert_planted_near(fr, Effects::IDENTITY, stand_fr, "foot.R@0")?;
				assert_planted_near(tl, Effects::IDENTITY, stand_tl, "toe.L@0")?;
				assert_planted_near(tr, Effects::IDENTITY, stand_tr, "toe.R@0")?;
				assert!((root.y - stand_root.y).abs() < 1e-4, "pelvis height at start");
			} else {
				assert!(!effects.is_identity(), "mid-descent lowers the armature");
				assert_planted_near(fl, effects, stand_fl, &format!("foot.L@{progress:.2}"))?;
				assert_planted_near(fr, effects, stand_fr, &format!("foot.R@{progress:.2}"))?;
				assert_planted_near(tl, effects, stand_tl, &format!("toe.L@{progress:.2}"))?;
				assert_planted_near(tr, effects, stand_tr, &format!("toe.R@{progress:.2}"))?;
				let _ = root;
			}
		}
		Ok(())
	}

	#[test]
	fn knee_flex_is_monotonic_without_overshoot() -> anyhow::Result<()> {
		let descent = SquatDescent::default();
		let mut held = HumanoidV0Rig::for_clip_test();
		Squat::held().apply(&mut held, 1.0);
		let held_knee = held.posed_angle("shin.L");

		let mut last = 0.0f32;
		for step in 0..=20 {
			let progress = step as f32 / 20.0;
			let mut rig = HumanoidV0Rig::for_clip_test();
			descent.apply(&mut rig, progress);
			let knee = rig.posed_angle("shin.L");
			anyhow::ensure!(knee + 1e-4 >= last, "knee reversed at {progress}: {knee} < {last}");
			anyhow::ensure!(
				knee <= held_knee + 1e-4,
				"knee overshoot at {progress}: {knee} > {held_knee}"
			);
			last = knee;
		}
		Ok(())
	}

	#[test]
	fn start_matches_upright() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatDescent::default().apply(&mut rig, 0.0);
		let rest = HumanoidV0Rig::for_clip_test();
		for name in ["femur.L", "shin.L", "root"] {
			let posed = tip(&rig, name);
			let rest_tip = tip(&rest, name);
			assert!((posed - rest_tip).length() < 1e-3, "{name} should stay at rest");
		}
		Ok(())
	}

	#[test]
	fn end_matches_held_squat() -> anyhow::Result<()> {
		let mut descent_rig = HumanoidV0Rig::for_clip_test();
		let mut held_rig = HumanoidV0Rig::for_clip_test();
		SquatDescent::default().apply(&mut descent_rig, 1.0);
		Squat::held().apply(&mut held_rig, 1.0);

		for name in descent_rig.animation_bone_names() {
			let a = descent_rig.rotation(name);
			let b = held_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn mid_descent_posed_knee_angle_is_documented() -> anyhow::Result<()> {
		let progress = 0.55;
		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatDescent::default().apply(&mut rig, progress);
		let knee_angle = rig.posed_angle("shin.L");
		let hip_angle = rig.posed_angle("femur.L");
		let knee_z = rig.character_point("shin.L").z;
		// Posed-joint anchors for stylization review (for_clip_test rest).
		assert!((knee_angle - 1.297).abs() < 0.02, "shin.L angle, got {knee_angle}");
		assert!((hip_angle - 0.776).abs() < 0.02, "femur.L angle, got {hip_angle}");
		assert!((knee_z - 0.142).abs() < 0.02, "shin.L character Z, got {knee_z}");
		Ok(())
	}

	#[test]
	fn knee_flex_leads_hip_flex_at_mid_descent() -> anyhow::Result<()> {
		let progress = 0.55;
		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatDescent::default().apply(&mut rig, progress);
		let knee = rig.posed_angle("shin.L");
		let hip = rig.posed_angle("femur.L");
		let depth = SquatDescent::default().depth(progress);
		let knee_share = (depth / crate::animations::squat_descent::KNEE_LEAD_DEPTH).min(1.0);
		assert!(
			knee > hip * (knee_share / depth) * 0.95,
			"knee should lead hips mid-descent (knee={knee}, hip={hip}, depth={depth})"
		);
		let knee_tip = rig.character_point("shin.L");
		let rest = HumanoidV0Rig::for_clip_test();
		let rest_knee = rest.character_point("shin.L");
		assert!(
			(knee_tip.z - rest_knee.z).abs() > 0.08,
			"knee joint should sit further forward than rest at mid-descent, got {knee_tip:?}"
		);
		Ok(())
	}

	#[test]
	fn both_femurs_fold_sagittally_at_mid_descent() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatDescent::default().apply(&mut rig, 0.5);
		let rest = HumanoidV0Rig::for_clip_test();
		let posed = rig.character_length("femur.L");
		let rest_dir = rest.character_length("femur.L");
		assert!(posed.z.abs() > rest_dir.z.abs() + 0.1, "squat folds in Z, {posed:?}");
		assert!(posed.x.abs() < 0.08, "squat is not a side swing, {posed:?}");
		assert!(
			(rig.posed_angle("femur.L") - rig.posed_angle("femur.R")).abs() < 1e-4,
			"both legs share flexion"
		);
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		let descent = SquatDescent::default();
		descent.apply(&mut once, 0.4);
		descent.apply(&mut twice, 0.4);
		descent.apply(&mut twice, 0.4);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn transition_from_idle_blends_into_mid_descent() -> anyhow::Result<()> {
		use crate::animations::{Idle, Squat, Transition};
		use character_rigs::authoring::ArmatureOffset;

		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut rig, 0.0);
		let from_pose = rig.pose.clone();
		let transition =
			Transition::from_visible(SquatDescent::default(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.5, 0.5);
		assert!(
			rig.posed_angle("femur.L") > 0.05
				&& rig.posed_angle("femur.L") < Squat::held().femur_peak.abs(),
			"blend should land between idle and full squat"
		);
		Ok(())
	}

	#[test]
	fn unwritten_bones_stay_at_rest() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let rest = HumanoidV0Rig::for_clip_test();
		SquatDescent::default().apply(&mut rig, 0.5);
		for name in ["humerus.L", "humerus.R", "forearm.L", "forearm.R"] {
			let posed = rig.rotation(name);
			let rest_rot = rest.rotation(name);
			assert!(posed.dot(rest_rot).abs() > 1.0 - 1e-5, "{name} should stay at rest");
		}
		Ok(())
	}
}
