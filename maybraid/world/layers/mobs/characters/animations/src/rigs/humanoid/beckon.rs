//! Humanoid mapping for [`Beckon`](crate::animations::Beckon).

use character_rigs::authoring::ArmAim;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{Beckon, Idle};
use crate::Animation;

impl Animation<HumanoidV0Rig> for Beckon {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let idle = Idle::default().sample_pose(0.0);
		let mut target = idle;
		target.spine.add_turn(self.torso_turn());
		let neck = self.neck_turn();
		target.apply_neck_twisted(neck * 0.65, 0.0, 0.0, neck * 0.35, 0.0, 0.0);
		let arm = target.arm_mut(self.side);
		arm.shoulder_lift += self.shoulder_lift(progress);
		arm.elbow_flexion = self.elbow_flex(progress);
		arm.aim = Some(ArmAim { along: self.humerus_along(progress), roll: self.beckon_roll() });

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
	use bevy::prelude::Vec3;
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use character_rigs::Side;

	use super::*;
	use crate::Animation;

	const PEAK: f32 = 0.48;

	fn idle() -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply_for(&mut rig, 0.0);
		rig
	}

	fn sample(side: Side, progress: f32) -> (HumanoidV0Rig, ArmatureOffset) {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let effect = Beckon::new(side).apply(&mut rig, progress);
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

	fn max_step(a: &HumanoidV0Rig, b: &HumanoidV0Rig) -> f32 {
		a.animation_bone_names()
			.map(|name| a.rotation(name).angle_between(b.rotation(name)))
			.fold(0.0_f32, f32::max)
	}

	#[test]
	fn beckon_reaches_forward_and_curl_pulls_hand_back_on_both_sides() -> anyhow::Result<()> {
		let rest = idle();
		for side in [Side::Left, Side::Right] {
			let (reach, effect) = sample(side, PEAK);
			let reach_tip = palm(&reach, side)?;
			let rest_tip = palm(&rest, side)?;
			assert_eq!(effect, ArmatureOffset::IDENTITY);
			assert!(reach_tip.z > rest_tip.z + 0.30, "{side:?} hand reaches forward {reach_tip:?}");
			assert!(reach_tip.x * side.sign() > 0.05, "hand remains on own side {reach_tip:?}");
			let early_curl = crate::animations::beckon::EXTEND_END
				+ (crate::animations::beckon::BECKON_END - crate::animations::beckon::EXTEND_END)
					* 0.35;
			let late_curl = crate::animations::beckon::EXTEND_END
				+ (crate::animations::beckon::BECKON_END - crate::animations::beckon::EXTEND_END)
					* 0.65;
			let (flat, _) = sample(side, early_curl);
			let (curl, _) = sample(side, late_curl);
			let delta = palm(&curl, side)? - palm(&flat, side)?;
			assert!(delta.length() > 0.02, "curl pulse changes the hand path {delta:?}");
			let other = side.opposite();
			assert!(
				palm(&reach, other)?.y
					< rest.character_point(&format!("humerus.{}", other.suffix())).y - 0.05
			);
			assert!(
				(reach.posed_angle(&format!("forearm.{}", other.suffix()))
					- rest.posed_angle(&format!("forearm.{}", other.suffix())))
				.abs() < 0.05
			);
		}
		Ok(())
	}

	#[test]
	fn beckon_endpoints_converge_at_sixty_and_240_hz() -> anyhow::Result<()> {
		let rest = idle();
		for side in [Side::Left, Side::Right] {
			let mut coarse_previous = rest.clone();
			let mut fine_previous = rest.clone();
			let mut coarse = 0.0_f32;
			let mut fine = 0.0_f32;
			for i in 1..=60 {
				let (rig, effect) = sample(side, i as f32 / 60.0);
				assert_eq!(effect, ArmatureOffset::IDENTITY);
				coarse = coarse.max(max_step(&coarse_previous, &rig));
				coarse_previous = rig;
			}
			for i in 1..=240 {
				let (rig, _) = sample(side, i as f32 / 240.0);
				fine = fine.max(max_step(&fine_previous, &rig));
				fine_previous = rig;
			}
			assert!(fine * 2.0 < coarse, "{side:?} 240 Hz refinement {fine} / {coarse}");
		}
		Ok(())
	}

	#[test]
	fn beckon_rests_and_uses_calibrated_endpoints_after_proportion_change() -> anyhow::Result<()> {
		let rest = idle();
		let (start, _) = sample(Side::Right, 0.0);
		let (end, _) = sample(Side::Right, 1.0);
		assert_eq!(start.pose, rest.pose);
		assert_eq!(end.pose, rest.pose);
		let mut long = HumanoidV0Rig::for_clip_test();
		long.forearm_endpoints[Side::Right.index()] *= 1.3;
		Beckon::default().apply_for(&mut long, PEAK);
		assert!(palm(&long, Side::Right)?.is_finite());
		Ok(())
	}
}
