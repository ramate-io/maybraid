use bevy::prelude::Vec3;
use character_rigs::authoring::{ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Sit;
use crate::{Animation, Effects};

impl Sit {
	/// Authored semantic pose at unwrapped `progress`. Rest is applied later by the rig.
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		let femur = self.femur_swing();
		let shin = self.shin_flex();
		for side in [Side::Left, Side::Right] {
			pose.apply_leg(side, femur, shin);
			if self.hip_peak.abs() > f32::EPSILON {
				pose.apply_hip_fold(side, self.hip_peak);
			}
			pose.leg_mut(side).hip_abduction = self.hip_spread * side.sign();
			apply_sit_arm(&mut pose, side, self);
		}
		pose.apply_spine_pitch(self.spine_pitch(progress));
		let breath = Sit::breath(progress) * self.breath_amp;
		pose.apply_neck_twisted(0.0, 0.0, 0.35 * breath, 0.0, 0.0, 0.65 * breath);
		pose
	}
}

impl Animation<HumanoidV0Rig> for Sit {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let pose = self.sample_pose(progress);
		rig.write_pose(&pose);
	}

	fn effects_for(&self, rig: &HumanoidV0Rig, progress: f32) -> Effects {
		if self.bones_only {
			return ArmatureOffset::IDENTITY;
		}
		let drop = self.vertical_drop(progress, rig.segment_lengths);
		if drop.abs() > f32::EPSILON {
			ArmatureOffset::from_translation(Vec3::new(0.0, -drop, 0.0))
		} else {
			ArmatureOffset::IDENTITY
		}
	}
}

fn apply_sit_arm(pose: &mut HumanoidPose, side: Side, sit: &Sit) {
	let hang = -side.sign() * sit.arm_hang;
	let inward = -side.sign() * 0.12;
	pose.apply_arm(
		side,
		sit.shoulder_forward,
		inward,
		sit.shoulder_forward * 0.45,
		hang,
		-sit.elbow_hang,
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::{Idle, Squat, Transition};

	const FRAME_DT: f32 = 1.0 / 60.0;
	const SETTLE_SECS: f32 = 0.6;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn visual_point(rig: &HumanoidV0Rig, effects: Effects, name: &str) -> Vec3 {
		effects.0.rotation * rig.character_point(name) + effects.0.translation
	}

	#[test]
	fn ground_sit_folds_both_legs_sagittally() -> anyhow::Result<()> {
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		let effects = Sit::ground().apply(&mut posed, 0.0);
		let rest_femur = rest.character_length("femur.L");
		let posed_femur = posed.character_length("femur.L");
		if posed_femur.z.abs() <= rest_femur.z.abs() + 0.15 {
			anyhow::bail!("ground sit should fold the thigh in Z, {posed_femur:?}");
		}
		if posed_femur.x.abs() >= 0.20 {
			anyhow::bail!("ground sit thigh should stay sagittal, {posed_femur:?}");
		}
		if (posed.posed_angle("femur.L") - posed.posed_angle("femur.R")).abs() >= 1e-4 {
			anyhow::bail!("both femurs should share the sit fold");
		}
		if posed.posed_angle("femur.L") <= 0.8 {
			anyhow::bail!("hips should fold past a shallow crouch");
		}
		if posed.posed_angle("shin.L") <= 0.8 {
			anyhow::bail!("knees should fold");
		}
		if effects.0.translation.y >= -0.2 {
			anyhow::bail!("ground sit should drop the armature, {effects:?}");
		}
		Ok(())
	}

	#[test]
	fn ground_sit_drops_knees_and_spreads_them() -> anyhow::Result<()> {
		let rest = HumanoidV0Rig::for_clip_test();
		let mut ground = HumanoidV0Rig::for_clip_test();
		let mut seat = HumanoidV0Rig::for_clip_test();
		let ground_fx = Sit::ground().apply(&mut ground, 0.0);
		let seat_fx = Sit::seat().apply(&mut seat, 0.0);
		let rest_knee = rest.character_point("shin.L");
		let ground_knee = visual_point(&ground, ground_fx, "shin.L");
		if ground_knee.y >= rest_knee.y - 0.15 {
			anyhow::bail!("ground sit knee should drop, {ground_knee:?} vs rest {rest_knee:?}");
		}
		if ground_knee.z.abs() <= rest_knee.z.abs() + 0.08 {
			anyhow::bail!("ground sit knee should travel forward, {ground_knee:?}");
		}
		let ground_span =
			(ground.character_point("shin.L").x - ground.character_point("shin.R").x).abs();
		let seat_span = (seat.character_point("shin.L").x - seat.character_point("shin.R").x).abs();
		if ground_span <= seat_span + 0.02 {
			anyhow::bail!("ground sit should spread the knees more, {ground_span} vs {seat_span}");
		}
		if ground_fx.0.translation.y >= seat_fx.0.translation.y - 0.04 {
			anyhow::bail!("ground sit should sit lower than a seat");
		}
		Ok(())
	}

	#[test]
	fn seat_sit_is_more_upright_than_held_squat() -> anyhow::Result<()> {
		let mut sit = HumanoidV0Rig::for_clip_test();
		let mut squat = HumanoidV0Rig::for_clip_test();
		Sit::seat().apply(&mut sit, 0.0);
		Squat::held().apply(&mut squat, 1.0);
		let sit_root = tip(&sit, "root");
		let squat_root = tip(&squat, "root");
		if sit_root.z >= squat_root.z - 0.02 {
			anyhow::bail!(
				"seat torso should pitch less than squat, {sit_root:?} vs {squat_root:?}"
			);
		}
		if sit_root.x.abs() >= 1e-3 {
			anyhow::bail!("seat spine must stay sagittal, {sit_root:?}");
		}
		if sit.posed_angle("humerus.L") <= 0.2 {
			anyhow::bail!("seat arms should leave the T-pose");
		}
		Ok(())
	}

	#[test]
	fn sit_drop_does_not_translate_bones() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let sit = Sit::ground();
		let effects = sit.apply(&mut rig, 0.0);
		assert!(effects.0.translation.y < -0.2);
		let femur = rig.binding.definition.id("femur.L").expect("femur");
		assert_eq!(
			rig.pose.get(femur).expect("pose").translation,
			rig.binding.effective_rest.get(femur).expect("rest").translation
		);
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		let sit = Sit::ground();
		sit.apply(&mut once, 0.4);
		sit.apply(&mut twice, 0.4);
		sit.apply(&mut twice, 0.4);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			if a.dot(b).abs() <= 1.0 - 1e-5 {
				anyhow::bail!("re-sample must not drift on {name}");
			}
		}
		Ok(())
	}

	#[test]
	fn settle_in_from_idle_uses_from_visible_at_frame_steps() -> anyhow::Result<()> {
		let mut source = HumanoidV0Rig::for_clip_test();
		let from_offset = Idle::default().apply(&mut source, 0.0);
		let transition = Transition::from_visible(Sit::ground(), source.pose.clone(), from_offset);
		let mut prev = 0.0;
		let mut t = 0.0;
		while t <= SETTLE_SECS + 1e-5 {
			let mut rig = HumanoidV0Rig::for_clip_test();
			let u = (t / SETTLE_SECS).clamp(0.0, 1.0);
			let effects = transition.apply(&mut rig, 0.0, u);
			let angle = rig.posed_angle("femur.L");
			if t > FRAME_DT && angle + 1e-4 < prev {
				anyhow::bail!("settle femur should not reverse, t={t} {angle} < {prev}");
			}
			prev = angle;
			t += FRAME_DT;
			if t >= SETTLE_SECS {
				if effects.0.translation.y >= -0.15 {
					anyhow::bail!("settled sit should keep the visual drop, {effects:?}");
				}
				if angle <= 0.7 {
					anyhow::bail!("settled sit should reach the seated fold, {angle}");
				}
			}
		}
		Ok(())
	}

	#[test]
	fn get_up_from_visible_sit_unwinds_at_frame_steps() -> anyhow::Result<()> {
		let mut source = HumanoidV0Rig::for_clip_test();
		let from_offset = Sit::ground().apply(&mut source, 0.0);
		let transition =
			Transition::from_visible(Idle::default(), source.pose.clone(), from_offset);
		let mut prev = source.posed_angle("femur.L");
		let mut t = 0.0;
		while t <= SETTLE_SECS + 1e-5 {
			let mut rig = HumanoidV0Rig::for_clip_test();
			let u = (t / SETTLE_SECS).clamp(0.0, 1.0);
			transition.apply(&mut rig, 0.0, u);
			let angle = rig.posed_angle("femur.L");
			if t > FRAME_DT && angle > prev + 1e-4 {
				anyhow::bail!("get-up femur should unwind, t={t} {angle} > {prev}");
			}
			prev = angle;
			t += FRAME_DT;
		}
		let mut stood = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut stood, 0.0, 1.0);
		if stood.posed_angle("femur.L") >= 0.2 {
			anyhow::bail!("get-up should leave the sit fold, {}", stood.posed_angle("femur.L"));
		}
		Ok(())
	}

	#[test]
	fn breath_loop_is_small_and_non_accumulating_on_frame_steps() -> anyhow::Result<()> {
		let sit = Sit::ground();
		let mut first = HumanoidV0Rig::for_clip_test();
		let mut later = HumanoidV0Rig::for_clip_test();
		sit.apply(&mut first, 0.0);
		let mut min_z = tip(&first, "upper_back").z;
		let mut max_z = min_z;
		let mut t = FRAME_DT;
		while t <= 1.0 / crate::animations::sit::SIT_BREATH_FREQ + 1e-5 {
			let mut rig = HumanoidV0Rig::for_clip_test();
			sit.apply(&mut rig, t);
			let z = tip(&rig, "upper_back").z;
			min_z = min_z.min(z);
			max_z = max_z.max(z);
			t += FRAME_DT;
		}
		if max_z - min_z <= 0.002 {
			anyhow::bail!("breath should move the upper back, span {}", max_z - min_z);
		}
		if max_z - min_z >= 0.08 {
			anyhow::bail!("breath should stay a idle, span {}", max_z - min_z);
		}
		sit.apply(&mut later, 0.0);
		sit.apply(&mut later, 0.0);
		if first.rotation("upper_back").dot(later.rotation("upper_back")).abs() <= 1.0 - 1e-5 {
			anyhow::bail!("re-sampling the same breath frame must not drift");
		}
		Ok(())
	}
}
