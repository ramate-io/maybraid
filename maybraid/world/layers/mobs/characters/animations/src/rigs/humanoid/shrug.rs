//! Humanoid mapping for [`Shrug`](crate::animations::Shrug).

use character_rigs::authoring::ArmAim;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{Idle, Shrug};
use crate::Animation;

impl Animation<HumanoidV0Rig> for Shrug {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let idle = Idle::default().sample_pose(0.0);
		let mut target = idle;
		for side in [Side::Left, Side::Right] {
			let arm = target.arm_mut(side);
			arm.shoulder_lift += self.shoulder_lift(side);
			arm.elbow_flexion = self.elbow_bend();
			arm.aim = Some(ArmAim { along: self.humerus_along(side), roll: 0.0 });
		}
		let tilt = self.neck_side_tilt();
		target.apply_neck_twisted(0.0, tilt * 0.65, 0.0, 0.0, tilt * 0.35, 0.0);

		let weight = self.shrug_amount(progress);
		if weight <= 0.0 {
			rig.write_pose(&idle);
			return;
		}
		let depth = rig.scratch.depth;
		rig.write_pose(&idle);
		rig.scratch.capture_from(depth, &rig.pose);
		rig.write_pose(&target);
		rig.scratch.capture_to(depth, &rig.pose);
		rig.scratch.blend_saved(depth, weight, &mut rig.pose);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use character_rigs::Side;

	use super::*;
	use crate::Animation;

	const PEAK: f32 = 0.45;

	fn idle() -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply_for(&mut rig, 0.0);
		rig
	}

	fn sample(progress: f32) -> (HumanoidV0Rig, ArmatureOffset) {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let effect = Shrug.apply(&mut rig, progress);
		(rig, effect)
	}

	fn forearm_tip(rig: &HumanoidV0Rig, side: Side) -> anyhow::Result<Vec3> {
		let name = format!("forearm.{}", side.suffix());
		let Some(bone) = rig.binding.definition.id(&name) else {
			anyhow::bail!("missing {name}");
		};
		let Some(affine) = rig.binding.definition.affine_in_character(&rig.pose, bone) else {
			anyhow::bail!("cannot compose {name}");
		};
		Ok(affine.transform_point3(rig.forearm_endpoints[side.index()]))
	}

	fn max_rotation_step(a: &HumanoidV0Rig, b: &HumanoidV0Rig) -> f32 {
		a.animation_bone_names()
			.map(|name| a.rotation(name).angle_between(b.rotation(name)))
			.fold(0.0_f32, f32::max)
	}

	#[test]
	fn shrug_endpoints_and_clamps_match_idle_with_identity_effects() -> anyhow::Result<()> {
		let rest = idle();
		for progress in [0.0, 1.0, -1.0, 2.0] {
			let (rig, effect) = sample(progress);
			assert_eq!(rig.pose, rest.pose, "progress {progress}");
			assert_eq!(effect, ArmatureOffset::IDENTITY);
		}
		Ok(())
	}

	#[test]
	fn shrug_lifts_both_bind_shoulders_and_mirrors_forearms() -> anyhow::Result<()> {
		let rest = idle();
		let (posed, effect) = sample(PEAK);
		assert_eq!(effect, ArmatureOffset::IDENTITY);
		let left_elbow = posed.character_point("forearm.L");
		let right_elbow = posed.character_point("forearm.R");
		let left_shoulder = posed.character_point("shoulder.L");
		let right_shoulder = posed.character_point("shoulder.R");
		assert!(
			left_elbow.y < left_shoulder.y - 0.02,
			"left elbow stays below shoulder {left_elbow:?}"
		);
		assert!(
			right_elbow.y < right_shoulder.y - 0.02,
			"right elbow stays below shoulder {right_elbow:?}"
		);
		let left = forearm_tip(&posed, Side::Left)?;
		let right = forearm_tip(&posed, Side::Right)?;
		assert!(left.y > left_elbow.y + 0.04, "left forearm bends up {left:?} / {left_elbow:?}");
		assert!(
			right.y > right_elbow.y + 0.04,
			"right forearm bends up {right:?} / {right_elbow:?}"
		);
		assert!(left.z > 0.05 && right.z > 0.05, "forearms come forward {left:?} {right:?}");
		assert!(left.y < left_shoulder.y + 0.08, "left hand stays near chest {left:?}");
		assert!(right.y < right_shoulder.y + 0.08, "right hand stays near chest {right:?}");
		assert!(left.x * Side::Left.sign() > 0.05, "left tip stays outboard {left:?}");
		assert!(right.x * Side::Right.sign() > 0.05, "right tip stays outboard {right:?}");
		assert!((left.x + right.x).abs() < 0.08, "mirrored tips {left:?} {right:?}");
		assert!((left.y - right.y).abs() < 0.08, "matched tip height {left:?} {right:?}");
		assert!(posed.posed_angle("forearm.L") > rest.posed_angle("forearm.L") + 0.2);
		assert!(posed.posed_angle("forearm.R") > rest.posed_angle("forearm.R") + 0.2);
		Ok(())
	}

	#[test]
	fn shrug_60_hz_samples_refine_at_240_hz() -> anyhow::Result<()> {
		let rest = idle();
		let mut coarse_previous = rest.clone();
		let mut fine_previous = rest.clone();
		let mut coarse_max = 0.0_f32;
		let mut fine_max = 0.0_f32;
		for index in 1..=60 {
			let (sample, _) = sample(index as f32 / 60.0);
			coarse_max = coarse_max.max(max_rotation_step(&coarse_previous, &sample));
			coarse_previous = sample;
		}
		for index in 1..=240 {
			let (sample, _) = sample(index as f32 / 240.0);
			fine_max = fine_max.max(max_rotation_step(&fine_previous, &sample));
			fine_previous = sample;
		}
		assert!(fine_max < coarse_max, "240 Hz {fine_max} should refine 60 Hz {coarse_max}");
		assert!(fine_max * 2.0 < coarse_max, "240 Hz refinement {fine_max} / {coarse_max}");
		Ok(())
	}

	#[test]
	fn shrug_leaves_legs_and_root_at_idle_baseline() -> anyhow::Result<()> {
		let rest = idle();
		let (posed, _) = sample(PEAK);
		for bone in ["femur.L", "femur.R", "shin.L", "shin.R", "pelvis.L", "pelvis.R", "root"] {
			assert!(
				posed.rotation(bone).normalize().dot(rest.rotation(bone).normalize()).abs()
					> 1.0 - 1e-5,
				"unwritten {bone} stays at Idle"
			);
		}
		Ok(())
	}

	#[test]
	fn shrug_uses_calibrated_hand_endpoints_after_proportion_change() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		rig.forearm_endpoints[Side::Right.index()] *= 1.25;
		Shrug.apply_for(&mut rig, PEAK);
		let tip = forearm_tip(&rig, Side::Right)?;
		assert!(tip.is_finite());
		for name in ["shoulder.R", "humerus.R", "forearm.R"] {
			assert!(rig.rotation(name).is_finite(), "finite {name}");
		}
		Ok(())
	}
}
