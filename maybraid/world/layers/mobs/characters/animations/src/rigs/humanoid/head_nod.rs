//! Humanoid mapping for [`HeadNod`](crate::animations::HeadNod).

use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{HeadNod, Idle};
use crate::Animation;

impl Animation<HumanoidV0Rig> for HeadNod {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let idle = Idle::default().sample_pose(0.0);
		let mut target = idle;
		let nod = self.nod_twist(progress);
		target.spine.add_stack_forward(self.spine_follow(progress));
		target.apply_neck_twisted(0.0, 0.0, nod * 0.35, 0.0, 0.0, nod * 0.65);

		let weight = self.gesture_amount(progress);
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
	use bevy::prelude::{Transform, Vec3};
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::Animation;

	const FIRST_DIP: f32 = 0.24;
	const SECOND_DIP: f32 = 0.54;

	fn idle() -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply_for(&mut rig, 0.0);
		rig
	}

	fn sample(progress: f32) -> (HumanoidV0Rig, ArmatureOffset) {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let effect = HeadNod.apply(&mut rig, progress);
		(rig, effect)
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

	fn max_rotation_step(a: &HumanoidV0Rig, b: &HumanoidV0Rig) -> f32 {
		a.animation_bone_names()
			.map(|name| a.rotation(name).angle_between(b.rotation(name)))
			.fold(0.0_f32, f32::max)
	}

	#[test]
	fn head_nod_endpoints_and_quiet_valley_match_idle() -> anyhow::Result<()> {
		let rest = idle();
		for progress in [0.0, 0.39, 1.0, -1.0, 2.0] {
			let (rig, effect) = sample(progress);
			assert_eq!(rig.pose, rest.pose, "progress {progress}");
			assert_eq!(effect, ArmatureOffset::IDENTITY);
		}
		Ok(())
	}

	#[test]
	fn head_nod_moves_full_affine_head_endpoint_twice() -> anyhow::Result<()> {
		let rest = idle();
		let rest_tip = head_tip(&rest)?;
		let (first, effect_first) = sample(FIRST_DIP);
		let (second, effect_second) = sample(SECOND_DIP);
		for (label, rig, effect) in
			[("first", first, effect_first), ("second", second, effect_second)]
		{
			let tip = head_tip(&rig)?;
			assert!(tip.y < rest_tip.y - 0.015, "{label} nod lowers head {tip:?} vs {rest_tip:?}");
			assert!(tip.z > rest_tip.z + 0.015, "{label} nod moves head forward {tip:?}");
			assert_eq!(effect, ArmatureOffset::IDENTITY);
		}
		Ok(())
	}

	#[test]
	fn head_nod_keeps_arms_and_legs_at_idle_and_refines_at_240_hz() -> anyhow::Result<()> {
		let rest = idle();
		let (peak, _) = sample(FIRST_DIP);
		for bone in [
			"humerus.L",
			"humerus.R",
			"forearm.L",
			"forearm.R",
			"femur.L",
			"femur.R",
			"shin.L",
			"shin.R",
		] {
			assert!(
				peak.rotation(bone).normalize().dot(rest.rotation(bone).normalize()).abs()
					> 1.0 - 1e-5,
				"{bone} stays at Idle"
			);
		}
		let mut coarse_previous = rest.clone();
		let mut fine_previous = rest.clone();
		let mut coarse_max = 0.0_f32;
		let mut fine_max = 0.0_f32;
		for index in 1..=60 {
			let (posed, _) = sample(index as f32 / 60.0);
			coarse_max = coarse_max.max(max_rotation_step(&coarse_previous, &posed));
			coarse_previous = posed;
		}
		for index in 1..=240 {
			let (posed, _) = sample(index as f32 / 240.0);
			fine_max = fine_max.max(max_rotation_step(&fine_previous, &posed));
			fine_previous = posed;
		}
		assert!(fine_max < coarse_max, "240 Hz {fine_max} should refine 60 Hz {coarse_max}");
		assert!(coarse_max < 0.2, "60 Hz step {coarse_max}");
		Ok(())
	}

	#[test]
	fn head_nod_composes_the_full_affine_for_a_scaled_neck() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let Some(id) = rig.binding.definition.id("upper_neck") else {
			anyhow::bail!("missing upper_neck");
		};
		let Some(rest) = rig.binding.effective_rest.local.get(id.index()).copied() else {
			anyhow::bail!("missing upper_neck rest");
		};
		rig.seed_rest("upper_neck", Transform { scale: rest.scale * 1.25, ..rest });
		HeadNod.apply_for(&mut rig, FIRST_DIP);
		let tip = head_tip(&rig)?;
		assert!(tip.is_finite());
		assert!(rig.rotation("upper_neck").is_finite());
		Ok(())
	}
}
