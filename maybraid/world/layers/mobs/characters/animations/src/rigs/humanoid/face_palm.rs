//! Humanoid mapping for [`FacePalm`](crate::animations::FacePalm).

use bevy::prelude::Vec3;
use character_rigs::articulation::{
	compose_parent_rotation, rotation_along_with_roll, TwoBoneAim, BONE_LENGTH_AXIS,
};
use character_rigs::authoring::{humanoid_bone_axis, ArmAim};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{FacePalm, Idle};
use crate::Animation;

#[derive(Debug, Clone, Copy)]
struct FacePalmReach {
	along: Vec3,
	flex: f32,
	roll: f32,
}

impl FacePalmReach {
	fn solve(rig: &HumanoidV0Rig, side: Side, target: Vec3, correction: Vec3) -> Option<Self> {
		let (shoulder_name, humerus_name, forearm_name) = match side {
			Side::Left => ("shoulder.L", "humerus.L", "forearm.L"),
			Side::Right => ("shoulder.R", "humerus.R", "forearm.R"),
		};
		let definition = &rig.binding.definition;
		let shoulder = definition.id(shoulder_name)?;
		let humerus = definition.id(humerus_name)?;
		let forearm = definition.id(forearm_name)?;
		let humerus_affine = definition.affine_in_character(&rig.pose, humerus)?;
		let forearm_affine = definition.affine_in_character(&rig.pose, forearm)?;
		let humerus_origin = humerus_affine.transform_point3(Vec3::ZERO);
		let forearm_origin = forearm_affine.transform_point3(Vec3::ZERO);
		let upper_length = humerus_origin.distance(forearm_origin);
		let lower_length =
			forearm_affine.transform_vector3(rig.forearm_endpoints[side.index()]).length();
		// Fold the elbow out and forward so the reach stays visible outside the torso.
		let pole = Vec3::X * side.sign() + Vec3::Z * 0.8;
		let aim = TwoBoneAim::reach(
			target - humerus_origin + correction,
			pole,
			upper_length,
			lower_length,
		)?;

		let shoulder_world = definition.rotation_in_character(&rig.pose, shoulder);
		let humerus_local = rig.binding.effective_rest.get(humerus)?;
		let forearm_local = rig.binding.effective_rest.get(forearm)?;
		let aimed = rotation_along_with_roll(
			humerus_local.rotation,
			shoulder_world.inverse() * aim.upper_along,
			0.0,
			BONE_LENGTH_AXIS,
		);
		let humerus_world = shoulder_world * aimed;
		let zero_roll_forearm = compose_parent_rotation(
			forearm_local.rotation,
			humanoid_bone_axis(forearm_name),
			0.0,
			aim.flex,
			0.0,
		);
		let from = zero_roll_forearm * BONE_LENGTH_AXIS;
		let to = humerus_world.inverse() * aim.lower_along;
		let axis = BONE_LENGTH_AXIS.try_normalize()?;
		let from = (from - axis * from.dot(axis)).try_normalize()?;
		let to = (to - axis * to.dot(axis)).try_normalize()?;
		let roll = axis.dot(from.cross(to)).atan2(from.dot(to));
		Some(Self { along: aim.upper_along, flex: aim.flex, roll })
	}
}

fn brow_point(rig: &HumanoidV0Rig) -> Option<Vec3> {
	let upper_neck = rig.binding.definition.id("upper_neck")?;
	let affine = rig.binding.definition.affine_in_character(&rig.pose, upper_neck)?;
	Some(affine.transform_point3(Vec3::Y * 0.25 + Vec3::Z * 0.18))
}

fn forearm_point(rig: &HumanoidV0Rig, side: Side) -> Option<Vec3> {
	let name = match side {
		Side::Left => "forearm.L",
		Side::Right => "forearm.R",
	};
	let forearm = rig.binding.definition.id(name)?;
	let affine = rig.binding.definition.affine_in_character(&rig.pose, forearm)?;
	Some(affine.transform_point3(rig.forearm_endpoints[side.index()]))
}

impl Animation<HumanoidV0Rig> for FacePalm {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let idle = Idle::default().sample_pose(0.0);
		let mut target = idle;
		target.apply_spine_pitch(self.spine_lean_back());
		let tilt = self.neck_tilt_back();
		target.apply_neck_twisted(0.0, 0.0, tilt * 0.35, 0.0, 0.0, tilt * 0.65);
		let arm = target.arm_mut(self.side);
		arm.shoulder_lift += self.shoulder_lift();
		arm.elbow_flexion = self.elbow_flex();
		arm.aim = Some(ArmAim { along: self.humerus_along(), roll: self.palm_roll() });

		let weight = self.strike_amount(progress);
		if weight <= 0.0 {
			rig.write_pose(&idle);
			return;
		}
		let depth = rig.scratch.depth;
		rig.write_pose(&idle);
		rig.scratch.capture_from(depth, &rig.pose);
		rig.write_pose(&target);
		if let Some(brow) = brow_point(rig) {
			let mut correction = Vec3::ZERO;
			for _ in 0..8 {
				if let Some(reach) = FacePalmReach::solve(rig, self.side, brow, correction) {
					let arm = target.arm_mut(self.side);
					arm.elbow_flexion = reach.flex;
					arm.aim = Some(ArmAim { along: reach.along, roll: reach.roll });
					rig.write_pose(&target);
					if let Some(actual) = forearm_point(rig, self.side) {
						correction += brow - actual;
					}
				}
			}
		}
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

	const PEAK: f32 = 0.48;
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
		let effect = FacePalm::new(side).apply(&mut rig, progress);
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

	fn palm_point(rig: &HumanoidV0Rig, side: Side) -> anyhow::Result<Vec3> {
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

	fn brow_target(rig: &HumanoidV0Rig) -> anyhow::Result<Vec3> {
		let Some(id) = rig.binding.definition.id("upper_neck") else {
			anyhow::bail!("missing upper_neck");
		};
		let Some(affine) = rig.binding.definition.affine_in_character(&rig.pose, id) else {
			anyhow::bail!("cannot compose upper_neck");
		};
		Ok(affine.transform_point3(Vec3::Y * 0.25 + Vec3::Z * 0.18))
	}

	fn joint(rig: &HumanoidV0Rig, name: &str) -> anyhow::Result<Vec3> {
		let Some(id) = rig.binding.definition.id(name) else {
			anyhow::bail!("missing {name}");
		};
		let Some(affine) = rig.binding.definition.affine_in_character(&rig.pose, id) else {
			anyhow::bail!("cannot compose {name}");
		};
		Ok(affine.transform_point3(Vec3::ZERO))
	}

	fn assert_clearance(
		rig: &HumanoidV0Rig,
		side: Side,
		hand: Vec3,
		head: Vec3,
	) -> anyhow::Result<()> {
		let shoulder = joint(rig, &format!("shoulder.{}", side.suffix()))?;
		let elbow = joint(rig, &format!("forearm.{}", side.suffix()))?;
		let chest = joint(rig, "upper_back")?;
		let midpoint = (elbow + hand) * 0.5;
		assert!(
			elbow.x * side.sign() > shoulder.x * side.sign() + 0.025,
			"{side:?} elbow remains outboard: {elbow:?} / {shoulder:?}"
		);
		assert!(elbow.y < head.y - 0.05, "{side:?} elbow stays below head: {elbow:?} / {head:?}");
		assert!(elbow.z > chest.z + 0.08, "{side:?} elbow clears torso: {elbow:?} / {chest:?}");
		assert!(
			midpoint.z > chest.z + 0.10,
			"{side:?} forearm stays in front of torso: {midpoint:?} / {chest:?}"
		);
		assert!(hand.z > head.z + 0.10, "{side:?} palm clears forehead: {hand:?} / {head:?}");
		Ok(())
	}

	fn max_step(a: &HumanoidV0Rig, b: &HumanoidV0Rig) -> f32 {
		a.animation_bone_names()
			.map(|name| a.rotation(name).angle_between(b.rotation(name)))
			.fold(0.0_f32, f32::max)
	}

	#[test]
	fn face_palm_centers_the_calibrated_hand_near_the_brow_on_both_sides() -> anyhow::Result<()> {
		let rest = idle();
		let mut tips = [Vec3::ZERO; 2];
		for side in [Side::Left, Side::Right] {
			let (rig, effect) = sample(side, PEAK);
			let hand = palm_point(&rig, side)?;
			let brow = brow_target(&rig)?;
			let head = head_tip(&rig)?;
			assert_eq!(effect, ArmatureOffset::IDENTITY);
			assert!(
				(hand - brow).length() < 0.025,
				"{side:?} hand contacts brow {hand:?} vs {brow:?}"
			);
			assert!(
				(hand.y - head.y).abs() < 0.06,
				"hand reaches brow height {hand:?}, head {head:?}"
			);
			assert!(hand.z > head.z + 0.10, "palm stays in front of brow {hand:?} / {head:?}");
			assert_clearance(&rig, side, hand, head)?;
			tips[side.index()] = hand;
			let other = side.opposite();
			for bone in
				[format!("humerus.{}", other.suffix()), format!("forearm.{}", other.suffix())]
			{
				assert!(
					rig.rotation(&bone).normalize().dot(rest.rotation(&bone).normalize()).abs()
						> 1.0 - 1e-5,
					"opposite arm local pose stays Idle: {bone}"
				);
			}
		}
		assert!((tips[0].x + tips[1].x).abs() < 0.08, "mirrored contact {tips:?}");
		assert!((tips[0].y - tips[1].y).abs() < 0.08, "matched contact height {tips:?}");
		Ok(())
	}

	#[test]
	fn face_palm_contacts_the_measured_standard_and_full_braidman_binds() -> anyhow::Result<()> {
		for endpoints in [STANDARD_ENDPOINTS, FULL_ENDPOINTS] {
			for side in [Side::Left, Side::Right] {
				let mut rig = braidman_rig(endpoints);
				FacePalm::new(side).apply_for(&mut rig, PEAK);
				let hand = palm_point(&rig, side)?;
				let brow = brow_target(&rig)?;
				let head = head_tip(&rig)?;
				assert!(
					(hand - brow).length() < 0.025,
					"{side:?} calibrated palm contacts brow: {hand:?} vs {brow:?}"
				);
				assert!(
					(hand.y - head.y).abs() < 0.06,
					"hand is at brow height {hand:?} / {head:?}"
				);
				assert!(
					hand.z > head.z + 0.12,
					"palm remains in front of brow {hand:?} / {head:?}"
				);
				assert_clearance(&rig, side, hand, head)?;
			}
		}
		Ok(())
	}

	#[test]
	fn face_palm_endpoints_and_60_to_240_hz_recovery_are_smooth() -> anyhow::Result<()> {
		let rest = idle();
		for side in [Side::Left, Side::Right] {
			for progress in [0.0, 1.0, -1.0, 2.0] {
				let (rig, effect) = sample(side, progress);
				assert_eq!(rig.pose, rest.pose);
				assert_eq!(effect, ArmatureOffset::IDENTITY);
			}
			let mut coarse_previous = rest.clone();
			let mut fine_previous = rest.clone();
			let mut coarse = 0.0_f32;
			let mut fine = 0.0_f32;
			for i in 1..=60 {
				let (rig, _) = sample(side, i as f32 / 60.0);
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
	fn face_palm_leaves_legs_at_idle_and_respects_endpoint_scale() -> anyhow::Result<()> {
		let rest = idle();
		let (posed, _) = sample(Side::Right, PEAK);
		for bone in ["humerus.L", "forearm.L", "femur.L", "femur.R", "shin.L", "shin.R"] {
			assert!(
				posed.rotation(bone).normalize().dot(rest.rotation(bone).normalize()).abs()
					> 1.0 - 1e-5,
				"unwritten {bone}"
			);
		}
		let mut long = HumanoidV0Rig::for_clip_test();
		long.forearm_endpoints[Side::Right.index()] *= 1.35;
		FacePalm::default().apply_for(&mut long, PEAK);
		assert!(palm_point(&long, Side::Right)?.is_finite());
		Ok(())
	}
}
