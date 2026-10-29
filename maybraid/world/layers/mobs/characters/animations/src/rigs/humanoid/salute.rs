//! Humanoid mapping for [`Salute`](crate::animations::Salute).

use character_rigs::authoring::ArmAim;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{Idle, Salute};
use crate::Animation;

impl Animation<HumanoidV0Rig> for Salute {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let idle = Idle::default().sample_pose(0.0);
		let mut target = idle;
		let neck = self.neck_turn();
		let nod = self.neck_nod();
		target.apply_neck_twisted(neck * 0.65, 0.0, nod * 0.35, neck * 0.35, 0.0, nod * 0.65);
		let arm = target.arm_mut(self.side);
		arm.elbow_flexion = self.salute_elbow();
		arm.aim = Some(ArmAim { along: self.humerus_along(), roll: self.salute_roll() });

		let weight = self.salute_amount(progress);
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
	use character_rigs::Side;

	use super::*;
	use crate::Animation;

	const PEAK: f32 = 0.45;
	const STANDARD_ENDPOINTS: [Vec3; 2] =
		[Vec3::new(-0.00224, 0.375388, -0.00511), Vec3::new(-0.00091, 0.375788, -0.00535)];
	const FULL_ENDPOINTS: [Vec3; 2] =
		[Vec3::new(0.004056, 0.308892, 0.010478), Vec3::new(-0.022457, 0.312406, 0.013176)];

	fn idle() -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply_for(&mut rig, 0.0);
		rig
	}

	fn sample(side: Side, progress: f32) -> (HumanoidV0Rig, ArmatureOffset) {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let effect = Salute::new(side).apply(&mut rig, progress);
		(rig, effect)
	}

	fn braidman_rig(endpoints: [Vec3; 2]) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		rig.forearm_endpoints = endpoints;
		let mut lumbar = Transform::from_translation(Vec3::Y * 0.25);
		lumbar.scale = Vec3::splat(0.8);
		rig.seed_rest("lumbar", lumbar);
		for name in ["humerus.L", "humerus.R"] {
			let mut humerus = Transform::from_translation(Vec3::Y * 0.35);
			humerus.scale = Vec3::new(0.8, 1.0, 0.8);
			rig.seed_rest(name, humerus);
		}
		rig
	}

	fn palm(rig: &HumanoidV0Rig, side: Side) -> anyhow::Result<Vec3> {
		let name = format!("forearm.{}", side.suffix());
		let Some(id) = rig.binding.definition.id(&name) else {
			anyhow::bail!("missing {name}");
		};
		let Some(affine) = rig.binding.definition.affine_in_character(&rig.pose, id) else {
			anyhow::bail!("cannot compose {name}");
		};
		Ok(affine.transform_point3(rig.forearm_endpoints[side.index()]))
	}

	fn head_tip(rig: &HumanoidV0Rig) -> anyhow::Result<Vec3> {
		let Some(id) = rig.binding.definition.id("upper_neck") else {
			anyhow::bail!("missing upper_neck");
		};
		let Some(affine) = rig.binding.definition.affine_in_character(&rig.pose, id) else {
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
	fn salute_endpoints_clamps_and_effects_match_idle() -> anyhow::Result<()> {
		let rest = idle();
		for side in [Side::Left, Side::Right] {
			for progress in [0.0, 1.0, -1.0, 2.0] {
				let (rig, effect) = sample(side, progress);
				assert_eq!(rig.pose, rest.pose);
				assert_eq!(effect, ArmatureOffset::IDENTITY);
			}
		}
		Ok(())
	}

	#[test]
	fn salute_both_sides_raise_the_hand_inboard_below_the_brow() -> anyhow::Result<()> {
		let rest = idle();
		let head = head_tip(&rest)?;
		let mut tips = [Vec3::ZERO; 2];
		for side in [Side::Left, Side::Right] {
			let (rig, _) = sample(side, PEAK);
			let tip = palm(&rig, side)?;
			let rest_tip = palm(&rest, side)?;
			assert!(tip.y > rest_tip.y + 0.20, "{side:?} hand rises {tip:?}");
			assert!(tip.x.abs() < rest_tip.x.abs() - 0.04, "{side:?} hand moves inboard {tip:?}");
			assert_eq!(tip.x.signum(), rest_tip.x.signum(), "no midline crossing");
			assert!(tip.y < head.y + 0.25, "hand reaches brow height {tip:?}, head {head:?}");
			tips[side.index()] = tip;
			let opposite = side.opposite();
			let opposite_tip = palm(&rig, opposite)?;
			assert!(opposite_tip.y < head.y - 0.15, "opposite arm stays relaxed");
		}
		assert!((tips[0].x + tips[1].x).abs() < 0.08, "mirrored salute tips {tips:?}");
		assert!((tips[0].y - tips[1].y).abs() < 0.08, "matched salute heights {tips:?}");
		Ok(())
	}

	#[test]
	fn salute_uses_measured_standard_and_full_braidman_endpoints() -> anyhow::Result<()> {
		for endpoints in [STANDARD_ENDPOINTS, FULL_ENDPOINTS] {
			for side in [Side::Left, Side::Right] {
				let mut rest = braidman_rig(endpoints);
				Idle::default().apply_for(&mut rest, 0.0);
				let rest_hand = palm(&rest, side)?;
				let head = head_tip(&rest)?;
				let mut rig = braidman_rig(endpoints);
				let effect = Salute::new(side).apply(&mut rig, PEAK);
				let hand = palm(&rig, side)?;
				assert_eq!(effect, ArmatureOffset::IDENTITY);
				assert!(hand.y > rest_hand.y + 0.15, "{side:?} calibrated hand rises {hand:?}");
				assert!(
					hand.x.abs() < rest_hand.x.abs() - 0.025,
					"{side:?} hand moves inboard {hand:?}"
				);
				assert_eq!(hand.x.signum(), rest_hand.x.signum(), "hand stays on its side");
				assert!(hand.y < head.y + 0.25, "hand remains below the brow {hand:?} / {head:?}");
			}
		}
		Ok(())
	}

	#[test]
	fn salute_recovery_refines_from_sixty_to_240_hz() -> anyhow::Result<()> {
		let rest = idle();
		for side in [Side::Left, Side::Right] {
			let mut coarse_previous = rest.clone();
			let mut fine_previous = rest.clone();
			let mut coarse_max = 0.0_f32;
			let mut fine_max = 0.0_f32;
			for index in 1..=60 {
				let (rig, _) = sample(side, index as f32 / 60.0);
				coarse_max = coarse_max.max(max_rotation_step(&coarse_previous, &rig));
				coarse_previous = rig;
			}
			for index in 1..=240 {
				let (rig, _) = sample(side, index as f32 / 240.0);
				fine_max = fine_max.max(max_rotation_step(&fine_previous, &rig));
				fine_previous = rig;
			}
			assert!(
				fine_max * 2.0 < coarse_max,
				"240 Hz {fine_max} should refine 60 Hz {coarse_max}"
			);
		}
		Ok(())
	}

	#[test]
	fn salute_leaves_legs_and_opposite_arm_at_idle() -> anyhow::Result<()> {
		let rest = idle();
		let (posed, _) = sample(Side::Right, PEAK);
		for bone in ["humerus.L", "forearm.L", "femur.L", "femur.R", "shin.L", "shin.R", "root"] {
			if bone.starts_with("humerus") || bone.starts_with("forearm") {
				continue;
			}
			assert!(
				posed.rotation(bone).normalize().dot(rest.rotation(bone).normalize()).abs()
					> 1.0 - 1e-5,
				"{bone} stays at Idle"
			);
		}
		assert!(posed.posed_angle("forearm.L") < rest.posed_angle("forearm.L") + 0.05);
		Ok(())
	}
}
