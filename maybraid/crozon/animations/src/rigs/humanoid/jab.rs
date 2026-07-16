//! Humanoid mapping for [`Jab`](crate::animations::Jab).
//!
//! # Bind tee pose (hand tips)
//!
//! Right ≈ `(-1.0, 1.7)`, left ≈ `(1.0, 1.7)`.
//!
//! # Axis assumptions (DEFAULT humerus: `swing=Y`, `flex=Z`, `twist=X`)
//!
//! | Channel | Local axis | Used for |
//! |---------|------------|----------|
//! | flex    | Z          | Drop to a side hang from the tee |
//! | twist   | X          | Elbow-plane cock + forward tip. π/2 aligns the hinge with Y; π/2+π/4
//! |         |            | cocks it ~45° between X and Y. Extra X tips the shaft. |
//! | swing   | Y          | **Ventral** carry so the arm sits in front of the torso (not inside it) |
//!
//! Light midline uses tuck's ±X flex, composed after DEFAULT. Magnitudes stay modest so
//! the left arm does not flatten into a horizontal bind.

use bevy::prelude::Vec3;
use crozon_rigs::humanoid::HumanoidRig;
use crozon_rigs::{RiggedAxis, Side};

use crate::animations::Jab;
use crate::rigs::humanoid::apply::{apply_leg, apply_root};
use crate::{Animation, Effects};

/// Tuck midline frame: flex on ±X only.
fn humerus_midline_axis(side: Side) -> RiggedAxis {
	match side {
		Side::Left => RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::X, twist_axis: Vec3::Z },
		Side::Right => {
			RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::NEG_X, twist_axis: Vec3::Z }
		}
	}
}

impl<R: HumanoidRig> Animation<R> for Jab<R> {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		let jab_side = self.side;
		let guard_side = self.opposite_side();

		apply_leg(rig, jab_side, self.lead_femur_swing(progress), self.stance_shin_flex(progress));
		apply_leg(rig, guard_side, self.rear_femur_swing(progress), self.stance_shin_flex(progress));
		apply_root(rig, self.root_lean(progress));
		apply_torso_roll(rig, jab_side, self.torso_turn(progress));

		apply_jab_arm(
			rig,
			jab_side,
			self.jab_humerus_drop(progress),
			self.humerus_ventral(progress),
			self.humerus_elbow_roll(progress),
			self.humerus_forward(progress),
			self.humerus_midline(progress),
			self.jab_elbow(progress),
		);
		apply_jab_arm(
			rig,
			guard_side,
			self.guard_humerus_drop(progress),
			self.humerus_ventral(progress),
			self.humerus_elbow_roll(progress),
			self.humerus_forward(progress),
			self.humerus_midline(progress),
			self.guard_elbow(progress),
		);

		Effects::default()
	}
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

/// Ventral on DEFAULT swing Y. +lateral_sign read dorsal earlier; invert for ventral.
fn humerus_ventral(side: Side, amount: f32) -> f32 {
	-amount * lateral_sign(side)
}

/// Elbow cock + forward tip on DEFAULT twist X.
/// Sign is inverted vs [`lateral_sign`]: +lateral on the right read as behind the back.
fn humerus_x_twist(side: Side, elbow_roll: f32, forward: f32) -> f32 {
	-(elbow_roll + forward) * lateral_sign(side)
}

/// Midline on tuck ±X flex.
fn humerus_midline(side: Side, amount: f32) -> f32 {
	amount * lateral_sign(side)
}

fn apply_jab_arm<R: HumanoidRig>(
	rig: &mut R,
	side: Side,
	drop: f32,
	ventral: f32,
	elbow_roll: f32,
	forward: f32,
	midline: f32,
	elbow: f32,
) {
	let mut arm = rig.arm_pose(side);

	// Shoulders stay at rest.
	arm.shoulder = rig.articulate_on_rig(arm.shoulder, 0.0, 0.0);

	// DEFAULT: ventral (Y) + side hang (Z) + X cock/forward.
	arm.humerus = rig.articulate_on_rig_twisted(
		arm.humerus,
		humerus_ventral(side, ventral),
		humerus_drop(side, drop),
		humerus_x_twist(side, elbow_roll, forward),
	);
	// Light midline on tuck ±X.
	arm.humerus = arm.humerus.articulate(
		humerus_midline_axis(side),
		0.0,
		humerus_midline(side, midline),
		0.0,
	);

	arm.forearm = rig.articulate_on_rig(arm.forearm, 0.0, elbow);
	rig.pose_arm(arm);
}

/// Trunk roll into the jab: midback / upper-back **swing (Y)**.
fn apply_torso_roll<R: HumanoidRig>(rig: &mut R, jab_side: Side, turn: f32) {
	let roll = turn * -lateral_sign(jab_side);
	let mut spine = rig.spine_pose();
	spine.midback = rig.articulate_on_rig(spine.midback, roll, 0.0);
	spine.upper_back = rig.articulate_on_rig(spine.upper_back, roll * 0.8, 0.0);
	rig.pose_spine(spine);
}

#[cfg(test)]
mod tests {
	use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

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
			.get(&rig.arm(jab.side).forearm.name)
			.ok_or_else(|| anyhow::anyhow!("missing jab forearm pose"))?;
		let guard = rig
			.pose()
			.get(&rig.arm(jab.opposite_side()).forearm.name)
			.ok_or_else(|| anyhow::anyhow!("missing guard forearm pose"))?;
		assert!(forearm.flex.abs() < guard.flex.abs());
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
	fn shoulders_stay_at_rest() -> anyhow::Result<()> {
		let jab = Jab::<HumanoidV0Rig>::default();
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.47);

		for side in [jab.side, jab.opposite_side()] {
			let shoulder = rig
				.pose()
				.get(&rig.arm(side).shoulder.name)
				.ok_or_else(|| anyhow::anyhow!("missing shoulder pose"))?;
			assert!(
				shoulder.swing.abs() < 1e-4
					&& shoulder.flex.abs() < 1e-4
					&& shoulder.twist.abs() < 1e-4,
				"expected resting shoulder, got swing={} flex={} twist={}",
				shoulder.swing,
				shoulder.flex,
				shoulder.twist
			);
		}
		Ok(())
	}

	#[test]
	fn right_humerus_cocks_past_y_aligned_roll() -> anyhow::Result<()> {
		let expected = FRAC_PI_2 + FRAC_PI_4;
		let jab = Jab::<HumanoidV0Rig>::default().with_side(Side::Right);
		assert!((jab.humerus_elbow_roll(0.0) - expected).abs() < 1e-4);
		assert!(jab.humerus_forward(0.0) > 0.3);

		let twist = humerus_x_twist(Side::Right, expected, jab.humerus_forward(0.0));
		// Right: flipped so X cock/forward goes in front, not behind the back.
		assert!(twist > 0.0);
		assert!(twist.abs() > expected);
		Ok(())
	}

	#[test]
	fn right_humerus_swings_ventral() -> anyhow::Result<()> {
		// Right lateral_sign is -1; ventral uses -lateral_sign → positive Y swing.
		assert!(humerus_ventral(Side::Right, 1.0) > 0.0);
		assert!(humerus_ventral(Side::Left, 1.0) < 0.0);
		Ok(())
	}

	#[test]
	fn humerus_drops_to_side_on_z() -> anyhow::Result<()> {
		let jab = Jab::<HumanoidV0Rig>::default();
		let rig = HumanoidV0Rig::imported();
		let mut arm = rig.arm_pose(jab.side);
		arm.humerus =
			rig.articulate_on_rig_twisted(arm.humerus, 0.0, humerus_drop(jab.side, 0.85), 0.0);
		assert_eq!(arm.humerus.flex.signum(), -lateral_sign(jab.side));
		assert!(arm.humerus.flex.abs() > 0.6);
		Ok(())
	}

	#[test]
	fn midline_stays_modest() -> anyhow::Result<()> {
		let jab = Jab::<HumanoidV0Rig>::default();
		assert!(jab.humerus_midline(0.0) < 0.2);
		assert!(jab.humerus_ventral(0.0) < 0.6);
		Ok(())
	}

	#[test]
	fn torso_rolls_on_mid_and_upper_back_swing() -> anyhow::Result<()> {
		let jab = Jab::<HumanoidV0Rig>::default();
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.47);

		let midback = rig
			.pose()
			.get(&rig.spine().midback.name)
			.ok_or_else(|| anyhow::anyhow!("missing midback pose"))?;
		let upper = rig
			.pose()
			.get(&rig.spine().upper_back.name)
			.ok_or_else(|| anyhow::anyhow!("missing upper_back pose"))?;
		assert!(
			midback.swing.abs() > 0.2,
			"expected midback Y roll, got swing={}",
			midback.swing
		);
		assert!(
			upper.swing.abs() > 0.15,
			"expected upper_back Y roll, got swing={}",
			upper.swing
		);
		Ok(())
	}
}
