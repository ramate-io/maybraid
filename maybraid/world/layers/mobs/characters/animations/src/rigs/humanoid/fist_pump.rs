//! Humanoid mapping for [`FistPump`](crate::animations::FistPump).

use character_rigs::authoring::ArmAim;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{FistPump, Idle};
use crate::Animation;

impl Animation<HumanoidV0Rig> for FistPump {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let idle = Idle::default().sample_pose(0.0);
		let mut target = idle;
		target.apply_root(self.root_lean_back(progress));
		target.apply_leg(self.side, 0.0, self.stance_knee(progress));
		let arm = target.arm_mut(self.side);
		arm.shoulder_forward += self.shoulder_windup(progress);
		arm.elbow_flexion = self.elbow_flex(progress);
		arm.aim = Some(ArmAim { along: self.humerus_along(progress), roll: self.pump_roll() });

		let weight = self.pump_amount(progress);
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

	const PEAK: f32 = 0.43;

	fn idle() -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply_for(&mut rig, 0.0);
		rig
	}

	fn sample(side: Side, progress: f32) -> (HumanoidV0Rig, ArmatureOffset) {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let effect = FistPump::new(side).apply(&mut rig, progress);
		(rig, effect)
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
	fn fist_pump_endpoints_and_clamps_match_idle() -> anyhow::Result<()> {
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
	fn fist_pump_chambers_then_clears_the_head_and_mirrors() -> anyhow::Result<()> {
		let rest = idle();
		let rest_head = head_tip(&rest)?;
		let mut peak_tips = [Vec3::ZERO; 2];
		for side in [Side::Left, Side::Right] {
			let (chamber, _) = sample(side, 0.08);
			let (peak, effect) = sample(side, PEAK);
			assert_eq!(effect, ArmatureOffset::IDENTITY);
			let chamber_tip = palm(&chamber, side)?;
			let tip = palm(&peak, side)?;
			assert!(
				chamber_tip.z < palm(&rest, side)?.z - 0.20,
				"{side:?} fist chambers back {chamber_tip:?}"
			);
			assert!(
				tip.y > rest_head.y + 0.20,
				"{side:?} fist clears head {tip:?} head {rest_head:?}"
			);
			assert!(tip.x * side.sign() > 0.05, "{side:?} fist remains outboard {tip:?}");
			peak_tips[side.index()] = tip;
			let opposite = side.opposite();
			let other = palm(&peak, opposite)?;
			assert!(
				other.y < rest.character_point(&format!("humerus.{}", opposite.suffix())).y - 0.08,
				"opposite hand remains relaxed"
			);
			assert!(
				peak.posed_angle(&format!("forearm.{}", opposite.suffix()))
					- rest.posed_angle(&format!("forearm.{}", opposite.suffix()))
					< 0.05
			);
		}
		assert!((peak_tips[0].x + peak_tips[1].x).abs() < 0.08, "mirrored fists {peak_tips:?}");
		assert!(
			(peak_tips[0].y - peak_tips[1].y).abs() < 0.08,
			"matched fist height {peak_tips:?}"
		);
		Ok(())
	}

	#[test]
	fn fist_pump_recovery_refines_at_240_hz_and_preserves_idle_bones() -> anyhow::Result<()> {
		let rest = idle();
		let (peak, _) = sample(Side::Right, PEAK);
		for bone in ["humerus.L", "forearm.L", "femur.L", "femur.R", "shin.L"] {
			assert!(
				peak.rotation(bone).normalize().dot(rest.rotation(bone).normalize()).abs()
					> 1.0 - 1e-5,
				"unwritten {bone}"
			);
		}
		let mut coarse_previous = rest.clone();
		let mut fine_previous = rest.clone();
		let mut coarse_max = 0.0_f32;
		let mut fine_max = 0.0_f32;
		for index in 1..=60 {
			let (rig, _) = sample(Side::Right, index as f32 / 60.0);
			coarse_max = coarse_max.max(max_rotation_step(&coarse_previous, &rig));
			coarse_previous = rig;
		}
		for index in 1..=240 {
			let (rig, _) = sample(Side::Right, index as f32 / 240.0);
			fine_max = fine_max.max(max_rotation_step(&fine_previous, &rig));
			fine_previous = rig;
		}
		assert!(fine_max * 2.0 < coarse_max, "240 Hz {fine_max} should refine 60 Hz {coarse_max}");
		assert!(
			peak.posed_angle("shin.R") > rest.posed_angle("shin.R") + 0.01,
			"stance knee flexes"
		);
		Ok(())
	}

	#[test]
	fn fist_pump_remains_finite_with_longer_forearm_endpoint() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		rig.forearm_endpoints[Side::Right.index()] *= 1.4;
		FistPump::new(Side::Right).apply_for(&mut rig, PEAK);
		let tip = palm(&rig, Side::Right)?;
		assert!(tip.is_finite());
		for name in ["humerus.R", "forearm.R", "upper_back", "femur.R"] {
			assert!(rig.rotation(name).is_finite(), "finite {name}");
		}
		Ok(())
	}
}
