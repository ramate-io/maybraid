//! Humanoid mapping for [`Bow`](crate::animations::Bow).

use character_rigs::authoring::ArmAim;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{Bow, Idle};
use crate::Animation;

impl Animation<HumanoidV0Rig> for Bow {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let idle = Idle::default().sample_pose(0.0);
		let mut target = idle;
		let pitch = self.spine_pitch(progress);
		// Keep the pelvis/root stable so the bow folds at the waist with planted feet.
		target.spine.forward_bend[1] += pitch * 0.58;
		target.spine.forward_bend[2] += pitch * 0.27;
		target.spine.forward_bend[3] += pitch * 0.15;
		for side in [Side::Left, Side::Right] {
			let arm = target.arm_mut(side);
			arm.elbow_flexion += self.elbow_flex(progress);
			arm.aim = Some(ArmAim { along: self.humerus_along(side), roll: 0.0 });
		}
		let nod = self.neck_nod(progress);
		target.apply_neck_twisted(0.0, 0.0, nod * 0.65, 0.0, 0.0, nod * 0.35);

		let weight = self.bow_amount(progress);
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

	fn sample(progress: f32) -> (HumanoidV0Rig, ArmatureOffset) {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let effects = Bow.apply(&mut rig, progress);
		(rig, effects)
	}

	fn idle() -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply_for(&mut rig, 0.0);
		rig
	}

	fn head_tip(rig: &HumanoidV0Rig) -> anyhow::Result<Vec3> {
		let Some(neck) = rig.binding.definition.id("upper_neck") else {
			anyhow::bail!("missing upper_neck");
		};
		let Some(affine) = rig.binding.definition.affine_in_character(&rig.pose, neck) else {
			anyhow::bail!("cannot compose upper_neck");
		};
		Ok(affine.transform_point3(Vec3::Y * 0.25))
	}

	fn hand_tip(rig: &HumanoidV0Rig, side: Side) -> anyhow::Result<Vec3> {
		let bone = format!("forearm.{}", side.suffix());
		let Some(id) = rig.binding.definition.id(&bone) else {
			anyhow::bail!("missing {bone}");
		};
		let Some(affine) = rig.binding.definition.affine_in_character(&rig.pose, id) else {
			anyhow::bail!("cannot compose {bone}");
		};
		Ok(affine.transform_point3(rig.forearm_endpoints[side.index()]))
	}

	fn ankle_anchor(rig: &HumanoidV0Rig, side: Side) -> anyhow::Result<Vec3> {
		let bone = format!("shin.{}", side.suffix());
		let Some(id) = rig.binding.definition.id(&bone) else {
			anyhow::bail!("missing {bone}");
		};
		let Some(affine) = rig.binding.definition.affine_in_character(&rig.pose, id) else {
			anyhow::bail!("cannot compose {bone}");
		};
		Ok(affine.transform_point3(Vec3::Y * rig.segment_lengths.shin))
	}

	fn max_rotation_step(a: &HumanoidV0Rig, b: &HumanoidV0Rig) -> f32 {
		a.animation_bone_names()
			.map(|name| a.rotation(name).angle_between(b.rotation(name)))
			.fold(0.0_f32, f32::max)
	}

	#[test]
	fn bow_endpoints_clamp_to_idle_with_identity_effects() -> anyhow::Result<()> {
		let rest = idle();
		for progress in [0.0, 1.0, -2.0, 3.0] {
			let (rig, effects) = sample(progress);
			assert_eq!(rig.pose, rest.pose, "progress {progress} equals Idle phase zero");
			assert_eq!(effects, ArmatureOffset::IDENTITY);
		}
		Ok(())
	}

	#[test]
	fn bow_lowers_head_and_hands_symmetrically_at_peak() -> anyhow::Result<()> {
		let rest = idle();
		let (bow, effects) = sample(PEAK);
		let rest_head = head_tip(&rest)?;
		let bow_head = head_tip(&bow)?;
		assert!(bow_head.y < rest_head.y - 0.08, "head dips {bow_head:?} vs {rest_head:?}");
		assert!(bow_head.z > rest_head.z + 0.04, "head follows forward {bow_head:?}");
		assert_eq!(effects, ArmatureOffset::IDENTITY);
		let left = hand_tip(&bow, Side::Left)?;
		let right = hand_tip(&bow, Side::Right)?;
		assert!(left.y < rest.character_point("shoulder.L").y, "left hand hangs {left:?}");
		assert!(right.y < rest.character_point("shoulder.R").y, "right hand hangs {right:?}");
		assert!((left.y - right.y).abs() < 0.08, "hand height mirrors {left:?} {right:?}");
		assert!((left.x + right.x).abs() < 0.10, "hand sides mirror {left:?} {right:?}");
		for side in [Side::Left, Side::Right] {
			let drift = ankle_anchor(&bow, side)?.distance(ankle_anchor(&rest, side)?);
			assert!(drift < 1e-4, "{side:?} ankle stays planted through waist bow: {drift}");
		}
		Ok(())
	}

	#[test]
	fn bow_samples_converge_from_sixty_to_two_hundred_forty_hz() -> anyhow::Result<()> {
		let rest = idle();
		let mut coarse_previous = rest.clone();
		let mut fine_previous = rest.clone();
		let mut coarse_max = 0.0_f32;
		let mut fine_max = 0.0_f32;
		for index in 1..=60 {
			let time = index as f32 / 60.0;
			let (coarse, _) = sample(time);
			coarse_max = coarse_max.max(max_rotation_step(&coarse_previous, &coarse));
			coarse_previous = coarse;
		}
		for index in 1..=240 {
			let time = index as f32 / 240.0;
			let (fine, _) = sample(time);
			fine_max = fine_max.max(max_rotation_step(&fine_previous, &fine));
			fine_previous = fine;
		}
		assert!(fine_max < coarse_max, "240 Hz {fine_max} should refine 60 Hz {coarse_max}");
		assert!(coarse_max < 0.2, "60 Hz pose step {coarse_max}");
		Ok(())
	}

	#[test]
	fn bow_changes_only_its_authored_body_and_arm_bones() -> anyhow::Result<()> {
		let rest = idle();
		let (posed, _) = sample(PEAK);
		for bone in
			["humerus.L", "humerus.R", "forearm.L", "forearm.R", "lumbar", "midback", "upper_back"]
		{
			assert!(posed.posed_angle(bone) > 1e-3, "{bone} participates in the bow");
		}
		for bone in ["upper_belly", "lat.L", "lat.R", "chest.L", "chest.R"] {
			assert!(
				posed.rotation(bone).normalize().dot(rest.rotation(bone).normalize()).abs()
					> 1.0 - 1e-5,
				"{bone} stays at Idle baseline"
			);
		}
		Ok(())
	}

	#[test]
	fn bow_uses_the_calibrated_forearm_endpoint_at_changed_arm_length() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		rig.forearm_endpoints[Side::Left.index()] *= 1.3;
		Bow.apply_for(&mut rig, PEAK);
		let tip = hand_tip(&rig, Side::Left)?;
		assert!(tip.is_finite());
		for name in ["humerus.L", "forearm.L", "upper_back"] {
			assert!(rig.rotation(name).is_finite(), "finite {name}");
		}
		Ok(())
	}
}
