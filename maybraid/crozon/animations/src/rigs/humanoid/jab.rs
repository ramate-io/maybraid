//! Humanoid mapping for [`Jab`](crate::animations::Jab).
//!
//! Humerus: [`HumanoidRig::humerus_along_with_roll`] with [`Jab::humerus_along`] + punch
//! roll. Punch travel = humerus +Z whip + elbow uncoil. Cover arm holds the guard along
//! frame with a tucked elbow. Trunk yaw spreads across lumbar → midback → upper_back and
//! both pelves.

use crozon_rigs::humanoid::HumanoidRig;
use crozon_rigs::Side;

use crate::animations::Jab;
use crate::rigs::humanoid::apply::{apply_leg, apply_root};
use crate::{Animation, Effects};

impl<R: HumanoidRig> Animation<R> for Jab<R> {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		let jab_side = self.side;
		let guard_side = self.opposite_side();

		apply_leg(rig, jab_side, self.lead_femur_swing(progress), self.stance_shin_flex(progress));
		apply_leg(rig, guard_side, self.rear_femur_swing(progress), self.stance_shin_flex(progress));
		apply_root(rig, self.root_lean(progress));
		apply_torso_turn(rig, jab_side, self.torso_turn(progress));
		apply_hip_turn(rig, jab_side, self.hip_turn(progress));

		apply_arm(
			rig,
			jab_side,
			self.humerus_along(jab_side, progress),
			humerus_roll(jab_side, self.punch_roll(progress)),
			self.shoulder_carry(progress),
			self.jab_elbow(progress),
		);
		apply_arm(
			rig,
			guard_side,
			self.humerus_along(guard_side, progress),
			humerus_roll(guard_side, self.punch_roll(progress)),
			0.0,
			self.guard_elbow(progress),
		);

		Effects::default()
	}
}

/// Long-axis roll after aim; mirrored so both elbows share the fight plane.
fn humerus_roll(side: Side, punch_roll: f32) -> f32 {
	punch_roll * side.sign()
}

fn apply_arm<R: HumanoidRig>(
	rig: &mut R,
	side: Side,
	along_world: bevy::prelude::Vec3,
	roll: f32,
	shoulder_carry: f32,
	elbow: f32,
) {
	let mut arm = rig.arm_pose(side);
	arm.shoulder = rig.articulate_on_rig(arm.shoulder, shoulder_carry, 0.0);
	arm.humerus = rig.humerus_along_with_roll(side, along_world, roll);
	arm.forearm = rig.articulate_on_rig(arm.forearm, 0.0, elbow);
	rig.pose_arm(arm);
}

fn apply_torso_turn<R: HumanoidRig>(rig: &mut R, jab_side: Side, turn: f32) {
	let yaw = turn * -jab_side.sign();
	let mut spine = rig.spine_pose();
	spine.lumbar = rig.articulate_on_rig(spine.lumbar, yaw * 0.35, 0.0);
	spine.midback = rig.articulate_on_rig(spine.midback, yaw * 0.4, 0.0);
	spine.upper_back = rig.articulate_on_rig(spine.upper_back, yaw * 0.25, 0.0);
	rig.pose_spine(spine);
}

fn apply_hip_turn<R: HumanoidRig>(rig: &mut R, jab_side: Side, hip: f32) {
	let yaw = hip * -jab_side.sign();
	for (side, weight) in [(jab_side, 1.0), (jab_side.opposite(), 0.65)] {
		let mut leg = rig.leg_pose(side);
		leg.pelvis = rig.articulate_on_rig(leg.pelvis, yaw * weight, 0.0);
		rig.pose_leg(leg);
	}
}

#[cfg(test)]
mod tests {
	use std::f32::consts::FRAC_PI_2;

	use bevy::prelude::Vec3;
	use crozon_rigs::articulation::BONE_LENGTH_AXIS;
	use crozon_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::Animation;

	#[test]
	fn jab_extends_punching_forearm() -> anyhow::Result<()> {
		let jab = Jab::<HumanoidV0Rig>::default();
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.47);

		let forearm = rig
			.pose()
			.get(&rig.arm(Side::Right).forearm.name)
			.ok_or_else(|| anyhow::anyhow!("missing jab forearm pose"))?;
		assert!(forearm.flex.abs() < 0.2);
		Ok(())
	}

	#[test]
	fn jab_applies_lead_stance() -> anyhow::Result<()> {
		let jab = Jab::<HumanoidV0Rig>::default();
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.47);

		let lead = rig
			.pose()
			.get(&rig.leg(jab.side).femur.name)
			.ok_or_else(|| anyhow::anyhow!("missing lead femur pose"))?;
		let rear = rig
			.pose()
			.get(&rig.leg(jab.opposite_side()).femur.name)
			.ok_or_else(|| anyhow::anyhow!("missing rear femur pose"))?;
		assert!(lead.swing > 0.0);
		assert!(rear.swing < 0.0);
		Ok(())
	}

	#[test]
	fn cover_arm_keeps_tucked_elbow() -> anyhow::Result<()> {
		let jab = Jab::<HumanoidV0Rig>::default().with_side(Side::Right);
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.47);

		let forearm = rig
			.pose()
			.get(&rig.arm(Side::Left).forearm.name)
			.ok_or_else(|| anyhow::anyhow!("left forearm"))?;
		assert!(forearm.flex > 1.0, "cover elbow should stay tucked, got {}", forearm.flex);
		Ok(())
	}

	#[test]
	fn punch_roll_locks_humerus_twist_near_ninety() -> anyhow::Result<()> {
		let jab = Jab::<HumanoidV0Rig>::default().with_side(Side::Right);
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.0);

		let humerus = rig
			.pose()
			.get(&rig.arm(Side::Right).humerus.name)
			.ok_or_else(|| anyhow::anyhow!("right humerus"))?;
		assert!(
			(humerus.twist + FRAC_PI_2).abs() < 1e-3,
			"expected ventral punch roll on twist, got {}",
			humerus.twist
		);
		assert!(humerus.swing.abs() < 1e-4);
		assert!(jab.shoulder_carry(0.0) < 0.2);
		Ok(())
	}

	#[test]
	fn humerus_along_with_roll_aims_length_in_world() -> anyhow::Result<()> {
		let rig = HumanoidV0Rig::imported();
		let along = Vec3::new(-0.35, -0.75, -0.55).normalize();
		let humerus = rig.humerus_along_with_roll(Side::Right, along, FRAC_PI_2);
		let parent = rig.parent_world_rotation(&humerus.name);
		let aimed_world = (parent * humerus.transform.rotation * BONE_LENGTH_AXIS).normalize();
		assert!(
			aimed_world.dot(along) > 0.99,
			"expected world aim {along:?}, got {aimed_world:?}"
		);
		Ok(())
	}

	#[test]
	fn higher_target_aims_humerus_less_down() -> anyhow::Result<()> {
		let sternum = Jab::<HumanoidV0Rig>::default().with_side(Side::Right);
		let chin = Jab::<HumanoidV0Rig>::default()
			.with_side(Side::Right)
			.with_target(Vec3::new(0.0, 0.55, 0.7));
		assert!(chin.humerus_along(Side::Right, 0.0).y > sternum.humerus_along(Side::Right, 0.0).y);
		Ok(())
	}

	#[test]
	fn torso_turn_spreads_across_spine() -> anyhow::Result<()> {
		let jab = Jab::<HumanoidV0Rig>::default();
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.47);

		let lumbar = rig
			.pose()
			.get(&rig.spine().lumbar.name)
			.ok_or_else(|| anyhow::anyhow!("missing lumbar pose"))?;
		let midback = rig
			.pose()
			.get(&rig.spine().midback.name)
			.ok_or_else(|| anyhow::anyhow!("missing midback pose"))?;
		let upper = rig
			.pose()
			.get(&rig.spine().upper_back.name)
			.ok_or_else(|| anyhow::anyhow!("missing upper_back pose"))?;
		assert!(lumbar.swing.abs() > 0.05);
		assert!(midback.swing.abs() > 0.05);
		assert!(upper.swing.abs() > 0.03);
		Ok(())
	}

	#[test]
	fn hip_turn_drives_pelvis_swing() -> anyhow::Result<()> {
		let jab = Jab::<HumanoidV0Rig>::default().with_side(Side::Right);
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.47);

		let jab_pelvis = rig
			.pose()
			.get(&rig.leg(Side::Right).pelvis.name)
			.ok_or_else(|| anyhow::anyhow!("jab pelvis"))?;
		assert!(jab_pelvis.swing.abs() > 0.02);
		Ok(())
	}
}
