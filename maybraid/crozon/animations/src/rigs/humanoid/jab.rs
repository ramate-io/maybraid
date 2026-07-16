//! Humanoid mapping for [`Jab`](crate::animations::Jab).
//!
//! Semantic amounts from the knobs layer are mapped onto humanoid_v0 bone axes here.
//! Shoulder bones share the same local Y swing *metadata*, but the mirrored bind pose
//! means the same world-space sagittal direction needs opposite local signs: right
//! ventral ≈ `+swing`, left ventral ≈ `-swing` (`forward * -lateral_sign`).
//! Fall's outward arm spread uses `shoulder_flex * lateral_sign`, so chest-fold uses
//! the opposite flex sign.

use crozon_rigs::humanoid::HumanoidRig;
use crozon_rigs::Side;

use crate::animations::Jab;
use crate::rigs::humanoid::apply::{apply_arm, apply_leg, apply_root};
use crate::{Animation, Effects};

impl<R: HumanoidRig> Animation<R> for Jab<R> {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		let jab_side = self.side;
		let guard_side = self.opposite_side();

		apply_leg(rig, jab_side, self.lead_femur_swing(progress), self.stance_shin_flex(progress));
		apply_leg(rig, guard_side, self.rear_femur_swing(progress), self.stance_shin_flex(progress));
		apply_root(rig, self.root_lean(progress));

		apply_arm(
			rig,
			jab_side,
			sagittal_swing(jab_side, self.jab_forward(progress)),
			jab_shoulder_flex(jab_side, self.jab_height(progress), self.jab_lateral(progress)),
			jab_humerus_swing(jab_side, self.extension_amount(progress)),
			jab_humerus_flex(self.jab_height(progress), self.extension_amount(progress)),
			forearm_bend(self.jab_elbow(progress)),
		);

		apply_arm(
			rig,
			guard_side,
			sagittal_swing(guard_side, self.guard_forward(progress)),
			chest_fold_flex(guard_side, self.guard_fold(progress)),
			0.0,
			chest_wrap_humerus(guard_side, self.guard_wrap(progress)),
			forearm_bend(self.guard_elbow(progress)),
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

/// Map semantic ventral reach onto shoulder swing (positive semantic → forward / ventral).
fn sagittal_swing(side: Side, forward: f32) -> f32 {
	forward * -lateral_sign(side)
}

/// Height / lateral aim on shoulder flex. Keep small so the jab stays a straight punch,
/// not a raise.
fn jab_shoulder_flex(side: Side, height: f32, lateral: f32) -> f32 {
	// Fall spreads with `+flex * lateral_sign`; aim tweaks stay in that frame.
	(height * 0.35 + lateral * 0.5) * lateral_sign(side)
}

fn jab_humerus_swing(side: Side, extend: f32) -> f32 {
	// Mild midline tracking at full reach.
	-extend * 0.12 * lateral_sign(side)
}

fn jab_humerus_flex(height: f32, extend: f32) -> f32 {
	0.1 * (1.0 - extend) + height * 0.25
}

/// Fall / spread uses `+flex * lateral_sign` to open the arms; clutch uses the inverse.
fn chest_fold_flex(side: Side, fold: f32) -> f32 {
	-fold * lateral_sign(side)
}

/// Wrap the forearm across the sternum (same side-sign convention as tuck humerus flex).
fn chest_wrap_humerus(side: Side, wrap: f32) -> f32 {
	wrap * lateral_sign(side)
}

/// Positive semantic elbow bend → forearm flex used by run/spring (positive = bent).
fn forearm_bend(elbow: f32) -> f32 {
	elbow
}

#[cfg(test)]
mod tests {
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
	fn jab_shoulder_swing_is_ventral_and_mirrored() -> anyhow::Result<()> {
		let peak = 0.47;
		let right = Jab::<HumanoidV0Rig>::default().with_side(Side::Right);
		let left = Jab::<HumanoidV0Rig>::default().with_side(Side::Left);
		let mut right_rig = HumanoidV0Rig::imported();
		let mut left_rig = HumanoidV0Rig::imported();
		right.apply(&mut right_rig, peak);
		left.apply(&mut left_rig, peak);

		let right_shoulder = right_rig
			.pose()
			.get(&right_rig.arm(Side::Right).shoulder.name)
			.ok_or_else(|| anyhow::anyhow!("missing right shoulder pose"))?;
		let left_shoulder = left_rig
			.pose()
			.get(&left_rig.arm(Side::Left).shoulder.name)
			.ok_or_else(|| anyhow::anyhow!("missing left shoulder pose"))?;

		// Right ventral ≈ +swing; left ventral ≈ -swing (mirrored bind pose).
		assert!(
			right_shoulder.swing > 0.5,
			"expected right ventral swing, got {}",
			right_shoulder.swing
		);
		assert!(
			left_shoulder.swing < -0.5,
			"expected left ventral swing, got {}",
			left_shoulder.swing
		);
		assert!(
			(right_shoulder.swing + left_shoulder.swing).abs() < 0.05,
			"expected mirrored magnitudes, right={}, left={}",
			right_shoulder.swing,
			left_shoulder.swing
		);
		Ok(())
	}

	#[test]
	fn guard_arm_folds_in_not_out() -> anyhow::Result<()> {
		let jab = Jab::<HumanoidV0Rig>::default();
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.0);

		let guard_side = jab.opposite_side();
		let shoulder = rig
			.pose()
			.get(&rig.arm(guard_side).shoulder.name)
			.ok_or_else(|| anyhow::anyhow!("missing guard shoulder pose"))?;
		// Opposite of Fall's outward spread sign for this side.
		let expected_fold_sign = -lateral_sign(guard_side);
		assert!(
			shoulder.flex.signum() == expected_fold_sign || shoulder.flex.abs() < 1e-4,
			"expected chest-fold flex sign {expected_fold_sign}, got {}",
			shoulder.flex
		);
		assert!(shoulder.flex.abs() > 0.4, "expected a real chest fold, got {}", shoulder.flex);
		Ok(())
	}
}
