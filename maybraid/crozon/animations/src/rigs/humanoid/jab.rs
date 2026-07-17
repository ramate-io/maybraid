//! Humanoid mapping for [`Jab`](crate::animations::Jab).
//!
//! # Punch-roll-first
//!
//! Tee forearm flex bends **up**. [`Jab::punch_roll`] (~π/2 on humerus twist X) rotates
//! that hinge ~90° ventrally so flex bends **front ↔ back**. Tune drop and elbow only
//! after that roll is locked. Humerus DEFAULT swing (Y) stays unused (long-axis spin).
//!
//! # Bind tee pose (hand tips)
//!
//! Right ≈ `(-1.0, 1.7)`, left ≈ `(1.0, 1.7)`.
//!
//! # Current scope
//!
//! Only the **right** arm is posed. The left arm stays at tee until left-side signs are
//! sorted out.
//!
//! # Axis map (DEFAULT: `swing=Y`, `flex=Z`, `twist=X`)
//!
//! | Bone            | Channel | Used for |
//! |-----------------|---------|----------|
//! | humerus         | twist X | Punch roll (~90° from tee) |
//! | humerus         | flex Z  | Arm drop from tee |
//! | shoulder        | swing Y | Tiny aim/height carry |
//! | forearm         | flex    | Guard / chamber / extend |
//! | lumbar→upper    | swing Y | Distributed trunk turn |
//! | pelvis          | swing Y | Hip contribution into the punch |

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

		// Left arm: force tee / bind rest until left-side tuning is ready.
		park_arm_at_tee(rig, Side::Left);

		if self.side == Side::Right {
			apply_right_jab_arm(
				rig,
				self.punch_roll(progress),
				self.arm_drop(progress),
				self.shoulder_carry(progress),
				self.jab_elbow(progress),
			);
		}

		Effects::default()
	}
}

fn park_arm_at_tee<R: HumanoidRig>(rig: &mut R, side: Side) {
	let mut arm = rig.arm_pose(side);
	arm.shoulder = rig.articulate_on_rig(arm.shoulder, 0.0, 0.0);
	arm.humerus = rig.articulate_on_rig_twisted(arm.humerus, 0.0, 0.0, 0.0);
	arm.forearm = rig.articulate_on_rig(arm.forearm, 0.0, 0.0);
	rig.pose_arm(arm);
}

fn lateral_sign(side: Side) -> f32 {
	match side {
		Side::Left => 1.0,
		Side::Right => -1.0,
	}
}

/// Drop on DEFAULT flex Z (run `arm_down` signs) — side hang.
fn humerus_drop(side: Side, drop: f32) -> f32 {
	-drop * lateral_sign(side)
}

/// Right-arm punch roll on DEFAULT twist X. Positive ~π/2 takes tee "flex up" → "flex ventral".
fn right_punch_roll(roll: f32) -> f32 {
	roll
}

fn apply_right_jab_arm<R: HumanoidRig>(
	rig: &mut R,
	punch_roll: f32,
	drop: f32,
	shoulder_carry: f32,
	elbow: f32,
) {
	let side = Side::Right;
	let mut arm = rig.arm_pose(side);

	// Tiny aim only — punch travel is elbow extension in the rolled frame.
	arm.shoulder = rig.articulate_on_rig(arm.shoulder, shoulder_carry, 0.0);

	// Order in compose is flex → twist → swing; we still author roll as the primary DOF.
	arm.humerus = rig.articulate_on_rig_twisted(
		arm.humerus,
		0.0,
		humerus_drop(side, drop),
		right_punch_roll(punch_roll),
	);

	arm.forearm = rig.articulate_on_rig(arm.forearm, 0.0, elbow);
	rig.pose_arm(arm);
}

/// Distribute trunk yaw across lumbar → midback → upper_back (swing Y).
fn apply_torso_turn<R: HumanoidRig>(rig: &mut R, jab_side: Side, turn: f32) {
	let roll = turn * -lateral_sign(jab_side);
	let mut spine = rig.spine_pose();
	spine.lumbar = rig.articulate_on_rig(spine.lumbar, roll * 0.35, 0.0);
	spine.midback = rig.articulate_on_rig(spine.midback, roll * 0.4, 0.0);
	spine.upper_back = rig.articulate_on_rig(spine.upper_back, roll * 0.25, 0.0);
	rig.pose_spine(spine);
}

/// Hip contribution: both pelves yaw with the trunk (jab-side slightly more).
fn apply_hip_turn<R: HumanoidRig>(rig: &mut R, jab_side: Side, hip: f32) {
	let yaw = hip * -lateral_sign(jab_side);
	for (side, weight) in [(jab_side, 1.0), (match jab_side {
		Side::Left => Side::Right,
		Side::Right => Side::Left,
	}, 0.65)]
	{
		let mut leg = rig.leg_pose(side);
		leg.pelvis = rig.articulate_on_rig(leg.pelvis, yaw * weight, 0.0);
		rig.pose_leg(leg);
	}
}

#[cfg(test)]
mod tests {
	use std::f32::consts::FRAC_PI_2;

	use crozon_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::Animation;

	#[test]
	fn jab_extends_punching_forearm() -> anyhow::Result<()> {
		let jab = Jab::<HumanoidV0Rig>::default();
		let mut rig = HumanoidV0Rig::imported();
		let peak = 0.47;
		jab.apply(&mut rig, peak);

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
	fn left_arm_stays_at_tee() -> anyhow::Result<()> {
		let jab = Jab::<HumanoidV0Rig>::default();
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.47);

		let shoulder = rig
			.pose()
			.get(&rig.arm(Side::Left).shoulder.name)
			.ok_or_else(|| anyhow::anyhow!("left shoulder"))?;
		let humerus = rig
			.pose()
			.get(&rig.arm(Side::Left).humerus.name)
			.ok_or_else(|| anyhow::anyhow!("left humerus"))?;
		let forearm = rig
			.pose()
			.get(&rig.arm(Side::Left).forearm.name)
			.ok_or_else(|| anyhow::anyhow!("left forearm"))?;
		assert!(shoulder.swing.abs() < 1e-4 && shoulder.flex.abs() < 1e-4);
		assert!(humerus.swing.abs() < 1e-4 && humerus.flex.abs() < 1e-4 && humerus.twist.abs() < 1e-4);
		assert!(forearm.flex.abs() < 1e-4);
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
			(humerus.twist - FRAC_PI_2).abs() < 1e-3,
			"expected punch roll on twist, got {}",
			humerus.twist
		);
		assert!(humerus.swing.abs() < 1e-4, "humerus Y unused, got {}", humerus.swing);
		assert!(
			jab.shoulder_carry(0.0) < 0.2,
			"shoulder carry should stay tiny, got {}",
			jab.shoulder_carry(0.0)
		);
		Ok(())
	}

	#[test]
	fn humerus_drops_to_side_on_z() -> anyhow::Result<()> {
		let rig = HumanoidV0Rig::imported();
		let mut arm = rig.arm_pose(Side::Right);
		arm.humerus =
			rig.articulate_on_rig_twisted(arm.humerus, 0.0, humerus_drop(Side::Right, 0.55), 0.0);
		assert_eq!(arm.humerus.flex.signum(), -lateral_sign(Side::Right));
		assert!(arm.humerus.flex.abs() > 0.4);
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
		assert!(lumbar.swing.abs() > 0.05, "lumbar swing={}", lumbar.swing);
		assert!(midback.swing.abs() > 0.05, "midback swing={}", midback.swing);
		assert!(upper.swing.abs() > 0.03, "upper_back swing={}", upper.swing);
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
		assert!(
			jab_pelvis.swing.abs() > 0.02,
			"expected pelvis yaw, got swing={}",
			jab_pelvis.swing
		);
		Ok(())
	}
}
