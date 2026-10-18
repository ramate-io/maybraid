//! Humanoid mapping for [`Bow`](crate::animations::Bow).

use bevy::prelude::Vec3;
use character_rigs::authoring::{ArmAim, ArmatureOffset, HumanoidPose, PoseBuffer};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Bow;
use crate::{Animation, Effects};

fn bone_tail_at(rig: &HumanoidV0Rig, pose: &PoseBuffer, name: &str) -> Vec3 {
	let id = rig.binding.definition.id(name).unwrap_or_else(|| panic!("missing bone {name}"));
	let origin = rig.binding.definition.translation_in_character(pose, id);
	let axis = rig.binding.definition.rotation_in_character(pose, id) * Vec3::Y;
	let len = rig
		.binding
		.effective_rest
		.get(id)
		.map(|transform| transform.translation.y)
		.unwrap_or(0.0);
	origin + axis * len
}

/// Visual-only shift so knee flex does not skate the feet (same armature-offset idea as squat).
fn foot_plant_offset(rig: &HumanoidV0Rig) -> Vec3 {
	let rest = &rig.binding.effective_rest;
	let posed = &rig.pose;
	let left = bone_tail_at(rig, rest, "shin.L") - bone_tail_at(rig, posed, "shin.L");
	let right = bone_tail_at(rig, rest, "shin.R") - bone_tail_at(rig, posed, "shin.R");
	(left + right) * 0.5
}

impl Animation<HumanoidV0Rig> for Bow {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let amount = self.bow_amount(progress);
		if amount < 1e-5 {
			return;
		}

		let mut pose = HumanoidPose::default();
		pose.apply_spine_pitch(self.spine_pitch(progress));
		pose.apply_root(self.root_lean(progress));

		let femur = self.femur_swing(progress);
		let shin = self.shin_flex(progress);
		let hip = self.hip_fold(progress);
		pose.apply_leg(Side::Left, femur, shin);
		pose.apply_leg(Side::Right, femur, shin);
		pose.apply_hip_fold(Side::Left, hip);
		pose.apply_hip_fold(Side::Right, hip);

		let nod = self.neck_nod(progress);
		pose.apply_neck_twisted(0.0, 0.0, nod * 0.65, 0.0, 0.0, nod * 0.35);

		let along = self.humerus_along(progress);
		let elbow = self.elbow_flex(progress);
		for side in [Side::Left, Side::Right] {
			let arm = pose.arm_mut(side);
			arm.elbow_flexion += elbow;
			arm.aim = Some(ArmAim { along, roll: 0.0 });
		}

		rig.write_pose(&pose);
	}

	fn effects_for(&self, rig: &HumanoidV0Rig, progress: f32) -> Effects {
		if self.bow_amount(progress) < 1e-5 {
			return Effects::IDENTITY;
		}
		let offset = foot_plant_offset(rig);
		if offset.length_squared() < 1e-10 {
			Effects::IDENTITY
		} else {
			ArmatureOffset::from_translation(offset)
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use character_rigs::Side;

	use super::*;
	use crate::animations::{Idle, Transition};
	use crate::Animation;

	const HOLD: f32 = 0.45;
	const PLANT_TOL: f32 = 0.02;

	fn peak() -> f32 {
		HOLD
	}

	fn bone_tail(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		bone_tail_at(rig, &rig.pose, name)
	}

	fn head_point(rig: &HumanoidV0Rig) -> Vec3 {
		bone_tail(rig, "upper_neck")
	}

	fn visible_foot(rig: &HumanoidV0Rig, side: Side, effects: ArmatureOffset) -> Vec3 {
		let bone = format!("shin.{}", side.suffix());
		bone_tail(rig, &bone) + effects.0.translation
	}

	fn sample_bow(progress: f32) -> (HumanoidV0Rig, ArmatureOffset) {
		let bow = Bow;
		let mut rig = HumanoidV0Rig::for_clip_test();
		let effects = bow.apply(&mut rig, progress);
		(rig, effects)
	}

	#[test]
	fn bow_repeatability() -> anyhow::Result<()> {
		let bow = Bow;
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		bow.apply(&mut a, peak());
		bow.apply(&mut b, peak());
		for bone in ["upper_back", "root", "femur.L", "femur.R", "forearm.L", "forearm.R"] {
			let ra = a.rotation(bone);
			let rb = b.rotation(bone);
			assert!(ra.dot(rb).abs() > 1.0 - 1e-5, "{bone} must match");
		}
		Ok(())
	}

	#[test]
	fn bow_entry_and_exit_are_continuous_at_rest() -> anyhow::Result<()> {
		let bow = Bow;
		let mut entry = HumanoidV0Rig::for_clip_test();
		let mut exit = HumanoidV0Rig::for_clip_test();
		let rest_pose = entry.pose.clone();
		bow.apply_for(&mut entry, 0.0);
		bow.apply_for(&mut exit, 1.0);
		assert_eq!(entry.pose, rest_pose, "entry should not write bones");
		assert_eq!(exit.pose, rest_pose, "exit should not write bones");
		Ok(())
	}

	#[test]
	fn bow_interruption_mid_clip_is_stable() -> anyhow::Result<()> {
		let bow = Bow;
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		bow.apply_for(&mut once, 0.38);
		bow.apply_for(&mut twice, 0.38);
		bow.apply_for(&mut twice, 0.38);
		for bone in ["upper_back", "root", "forearm.L", "forearm.R"] {
			let delta = once.rotation(bone).angle_between(twice.rotation(bone));
			assert!(delta < 1e-3, "{bone} drifted {delta}");
		}
		Ok(())
	}

	#[test]
	fn bow_lowers_head_at_peak() -> anyhow::Result<()> {
		let bow = Bow;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		bow.apply(&mut posed, peak());

		let rest_head = head_point(&rest);
		let posed_head = head_point(&posed);
		assert!(
			posed_head.y < rest_head.y - 0.08,
			"head dips with torso, {posed_head:?} vs {rest_head:?}"
		);
		assert!(
			posed_head.z > 0.05,
			"head should pitch forward in character space, {posed_head:?}"
		);
		Ok(())
	}

	#[test]
	fn bow_mirrors_arm_pose() -> anyhow::Result<()> {
		let bow = Bow;
		let mut rig = HumanoidV0Rig::for_clip_test();
		bow.apply(&mut rig, peak());

		for side in [Side::Left, Side::Right] {
			let bone = format!("forearm.{}", side.suffix());
			let tip = rig.character_point(&bone);
			assert!(tip.x.signum() == side.sign(), "{side:?} arm stays on its side, {tip:?}");
		}

		let left = rig.character_point("forearm.L");
		let right = rig.character_point("forearm.R");
		assert!((left.y - right.y).abs() < 0.05, "matched forearm height {left:?} {right:?}");
		assert!((left.x + right.x).abs() < 0.08, "mirrored spread {left:?} {right:?}");
		Ok(())
	}

	#[test]
	fn bow_pelvis_flex_is_bilateral_at_peak() -> anyhow::Result<()> {
		let bow = Bow;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		bow.apply(&mut posed, peak());

		let left = posed.posed_angle("pelvis.L");
		let right = posed.posed_angle("pelvis.R");
		assert!(left > rest.posed_angle("pelvis.L") + 0.04, "hip fold reads on left");
		assert!((left - right).abs() < 0.03, "mirrored pelvis flex {left} {right}");
		Ok(())
	}

	#[test]
	fn bow_posed_evidence_at_key_progress() -> anyhow::Result<()> {
		let rest = HumanoidV0Rig::for_clip_test();
		let rest_head = head_point(&rest);
		let rest_upper = bone_tail(&rest, "upper_back");
		let rest_foot_l = bone_tail(&rest, "shin.L");
		let rest_foot_r = bone_tail(&rest, "shin.R");

		for progress in [0.0, 0.2, HOLD, 1.0] {
			let (rig, effects) = sample_bow(progress);
			let head = head_point(&rig);
			let upper = bone_tail(&rig, "upper_back");
			let hand_l = bone_tail(&rig, "forearm.L");
			let hand_r = bone_tail(&rig, "forearm.R");
			let elbow_l = rig.character_point("forearm.L");
			let elbow_r = rig.character_point("forearm.R");

			if progress > 0.0 && progress < 1.0 {
				assert!(head.y < rest_head.y - 0.03, "progress {progress} head y {head:?}");
				assert!(head.z > rest_head.z + 0.03, "progress {progress} head z {head:?}");
				assert!(upper.z > rest_upper.z + 0.03, "progress {progress} torso z {upper:?}");
			}

			assert!((hand_l.y - hand_r.y).abs() < 0.05, "progress {progress}");
			assert!((hand_l.z - hand_r.z).abs() < 0.05, "progress {progress}");
			assert!((hand_l.x + hand_r.x).abs() < 0.08, "progress {progress}");

			if progress > 0.0 && progress < 1.0 {
				assert!(hand_l.y < 0.35, "hands hang from T-pose, {hand_l:?} {hand_r:?}");
				assert!(hand_l.x > 0.05 && hand_r.x < -0.05, "hands stay on their sides");
				assert!(elbow_l.x > 0.0 && elbow_r.x < 0.0, "elbows stay lateral");
			}

			if progress > 0.0 && progress < 1.0 {
				for (side, rest_foot) in [(Side::Left, rest_foot_l), (Side::Right, rest_foot_r)] {
					let planted = visible_foot(&rig, side, effects);
					let delta = planted - rest_foot;
					assert!(
						delta.x.abs() < PLANT_TOL
							&& delta.y.abs() < PLANT_TOL
							&& delta.z.abs() < PLANT_TOL,
						"progress {progress} {side:?} foot drift {delta:?}"
					);
				}
			} else {
				assert!(effects.is_identity(), "rest endpoints have no armature shift");
			}
		}
		Ok(())
	}

	#[test]
	fn transition_from_visible_mid_bow_blends_toward_idle() -> anyhow::Result<()> {
		let bow = Bow;
		let mut rig = HumanoidV0Rig::for_clip_test();
		bow.apply_for(&mut rig, HOLD);
		let from_pose = rig.pose.clone();
		let from_offset = bow.effects_for(&rig, HOLD);
		let peak_angle = rig.posed_angle("upper_back");
		let transition = Transition::from_visible(Idle::default(), from_pose, from_offset);
		transition.apply(&mut rig, 0.0, 0.5);
		assert!(
			rig.posed_angle("upper_back") < peak_angle * 0.85,
			"mid-bow interrupt should ease torso back toward idle"
		);
		Ok(())
	}

	#[test]
	fn bow_loop_boundary_matches_rest() -> anyhow::Result<()> {
		let bow = Bow;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut at_one = HumanoidV0Rig::for_clip_test();
		let mut at_zero = HumanoidV0Rig::for_clip_test();
		bow.apply(&mut at_zero, 0.0);
		bow.apply(&mut at_one, 1.0);
		let rest_head = head_point(&rest);
		let zero_head = head_point(&at_zero);
		let one_head = head_point(&at_one);
		assert!((zero_head - rest_head).length() < 1e-3);
		assert!((one_head - rest_head).length() < 1e-3);
		Ok(())
	}
}
