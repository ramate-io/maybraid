//! Humanoid mapping for [`StompBeat`](crate::animations::StompBeat).

use bevy::prelude::Quat;
use character_rigs::authoring::ArmAim;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{Idle, StompBeat};
use crate::Animation;

impl Animation<HumanoidV0Rig> for StompBeat {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let idle = Idle::default().sample_pose(0.0);
		let mut target = idle;
		let weight = self.motion_amount(progress);
		if weight <= 0.0 {
			rig.write_pose(&idle);
			return;
		}

		let inv_weight = 1.0 / weight;
		let stomp_side = self.stomping_side();
		let support_side = stomp_side.opposite();
		target.apply_leg(
			stomp_side,
			self.stomping_hip_flexion(progress) * inv_weight,
			self.stomping_knee_flexion(progress) * inv_weight,
		);
		target.apply_leg(support_side, 0.0, self.support_knee_flexion(progress) * inv_weight);
		target.apply_spine_pitch(self.torso_dip(progress) * inv_weight);

		let pump_side = self.pump_side();
		let impact = self.impact_amount(progress) * (1.0 - self.recover_blend(progress));
		let pump_mix = (impact * inv_weight).clamp(0.0, 1.0);
		if pump_mix > 0.0 {
			let humerus = format!("humerus.{}", pump_side.suffix());
			rig.write_pose(&idle);
			let idle_along = rig.character_length(&humerus);
			let aim_rotation =
				Quat::from_rotation_arc(idle_along, self.pump_humerus_along(progress));
			let along =
				(Quat::IDENTITY.slerp(aim_rotation, pump_mix) * idle_along).normalize_or_zero();
			let arm = target.arm_mut(pump_side);
			arm.elbow_flexion += self.pump_elbow(progress) * inv_weight;
			arm.aim = Some(ArmAim { along, roll: self.pump_roll() * pump_side.sign() * pump_mix });
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

	fn idle() -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply_for(&mut rig, 0.0);
		rig
	}

	fn sample(side: Side, progress: f32) -> (HumanoidV0Rig, ArmatureOffset) {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let effect = StompBeat::new(side).apply(&mut rig, progress);
		(rig, effect)
	}

	fn hand(rig: &HumanoidV0Rig, side: Side) -> anyhow::Result<Vec3> {
		let name = format!("forearm.{}", side.suffix());
		let Some(id) = rig.binding.definition.id(&name) else {
			anyhow::bail!("missing {name}");
		};
		let Some(affine) = rig.binding.definition.affine_in_character(&rig.pose, id) else {
			anyhow::bail!("cannot compose {name}");
		};
		Ok(affine.transform_point3(rig.forearm_endpoints[side.index()]))
	}

	fn foot(rig: &HumanoidV0Rig, side: Side) -> anyhow::Result<Vec3> {
		let name = format!("shin.{}", side.suffix());
		let Some(id) = rig.binding.definition.id(&name) else {
			anyhow::bail!("missing {name}");
		};
		let Some(affine) = rig.binding.definition.affine_in_character(&rig.pose, id) else {
			anyhow::bail!("cannot compose {name}");
		};
		Ok(affine.transform_point3(Vec3::Y * rig.segment_lengths.shin))
	}

	fn max_step(a: &HumanoidV0Rig, b: &HumanoidV0Rig) -> f32 {
		a.animation_bone_names()
			.map(|name| a.rotation(name).angle_between(b.rotation(name)))
			.fold(0.0_f32, f32::max)
	}

	#[test]
	fn stomp_lifts_then_slams_and_whips_the_opposite_arm() -> anyhow::Result<()> {
		for side in [Side::Left, Side::Right] {
			let rest = idle();
			let (lift, _) = sample(side, crate::animations::stomp_beat::LIFT_CENTER);
			let (impact, effect) = sample(side, crate::animations::stomp_beat::IMPACT_CENTER);
			assert_eq!(effect, ArmatureOffset::IDENTITY);
			let stomp_bone = format!("shin.{}", side.suffix());
			let rest_knee = rest.character_point(&stomp_bone);
			let lift_point = lift.character_point(&stomp_bone);
			let impact_point = impact.character_point(&stomp_bone);
			let knee_delta = lift_point - rest_knee;
			let foot_delta = foot(&lift, side)? - foot(&rest, side)?;
			let planted_foot_delta = foot(&impact, side)? - foot(&rest, side)?;
			assert!(
				knee_delta.y > 0.04 && knee_delta.z > 0.08,
				"{side:?} knee lifts forward/up: {knee_delta:?}"
			);
			assert!(
				foot_delta.y > 0.02 && foot_delta.z > 0.04,
				"{side:?} full-affine foot follows forward/up: {foot_delta:?}"
			);
			assert!(
				impact_point.y < lift_point.y - 0.06,
				"impact drops below lift {impact_point:?} {lift_point:?}"
			);
			assert!(
				planted_foot_delta.y.abs() < 0.10 && planted_foot_delta.z.abs() < 0.16,
				"impact foot plants near its rest anchor: {planted_foot_delta:?}"
			);
			let pump = hand(&impact, side.opposite())? - hand(&rest, side.opposite())?;
			assert!(pump.y < -0.01 && pump.z > 0.01, "opposite hand moves down/forward {pump:?}");
			assert!(
				impact.posed_angle(&format!("shin.{}", side.opposite().suffix()))
					> rest.posed_angle(&format!("shin.{}", side.opposite().suffix())) + 0.03
			);
		}
		Ok(())
	}

	#[test]
	fn stomp_endpoints_clamp_and_mirror_both_sides() -> anyhow::Result<()> {
		let rest = idle();
		let (right, _) = sample(Side::Right, crate::animations::stomp_beat::IMPACT_CENTER);
		let (left, _) = sample(Side::Left, crate::animations::stomp_beat::IMPACT_CENTER);
		let r_stomp = right.character_point("shin.R");
		let l_stomp = left.character_point("shin.L");
		assert!((r_stomp.x + l_stomp.x).abs() < 0.06, "mirrored shins {r_stomp:?} {l_stomp:?}");
		let r_pump = hand(&right, Side::Left)?;
		let l_pump = hand(&left, Side::Right)?;
		assert!((r_pump.x + l_pump.x).abs() < 0.08, "mirrored arm whip {r_pump:?} {l_pump:?}");
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
	fn stomp_sampling_is_repeatable_and_refines_at_240_hz() -> anyhow::Result<()> {
		let rest = idle();
		let (once, _) = sample(Side::Right, 0.35);
		let (twice, _) = sample(Side::Right, 0.35);
		assert_eq!(once.pose, twice.pose);
		let mut coarse_previous = rest.clone();
		let mut fine_previous = rest.clone();
		let mut coarse = 0.0_f32;
		let mut fine = 0.0_f32;
		for i in 1..=60 {
			let (rig, _) = sample(Side::Right, i as f32 / 60.0);
			coarse = coarse.max(max_step(&coarse_previous, &rig));
			coarse_previous = rig;
		}
		for i in 1..=240 {
			let (rig, _) = sample(Side::Right, i as f32 / 240.0);
			fine = fine.max(max_step(&fine_previous, &rig));
			fine_previous = rig;
		}
		assert!(fine * 2.0 < coarse, "240/60 Hz refinement {fine}/{coarse}");
		Ok(())
	}
}
