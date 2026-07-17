use std::f32::consts::FRAC_PI_2;
use std::marker::PhantomData;

use bevy::prelude::Vec3;
use crozon_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

/// Default jab aim relative to the body COM: chest height, straight ahead.
pub const DEFAULT_JAB_TARGET: Vec3 = Vec3::new(0.0, 0.35, 0.7);
/// Nominal chamber depth (`1.0` matches the tuned preparatory motion).
pub const DEFAULT_BACKSWING: f32 = 1.0;

const BACKSWING_END: f32 = 0.18;
const EXTEND_END: f32 = 0.42;
const HOLD_END: f32 = 0.52;

/// Semantic pose magnitudes (radians / blends). Rig impls map these onto bone axes.
///
/// # Punch-roll-first model
///
/// Tee: humerus roll leaves forearm flex bending **up**.
/// Punch: ~90° ventral roll so the same flex bends **front ↔ back**. That roll is the
/// primary constant; drop and elbow are tuned against it. Punch travel is mostly elbow
/// extension. Trunk/hips add weight; shoulder carry stays tiny for aim only.
///
/// Bind tee tips (world-ish): right ≈ `(-1.0, 1.7)`, left ≈ `(1.0, 1.7)`.
const GUARD_ELBOW: f32 = 1.5;
const CHAMBER_ELBOW: f32 = 1.65;
const EXTEND_ELBOW: f32 = 0.05;

/// ~90° from tee — reorients the elbow hinge into the sagittal (front/back) plane.
const PUNCH_ROLL: f32 = FRAC_PI_2;
/// Side hang from the tee once punch roll is established.
const ARM_DROP: f32 = 0.55;
/// Tiny shoulder aim/height; not the punch driver.
const SHOULDER_CARRY: f32 = 0.12;

/// Total trunk turn into the punch (rig spreads across lumbar / mid / upper / hips).
const TORSO_TURN: f32 = 0.55;
const ROOT_LEAN: f32 = 0.05;
const LEAD_FEMUR: f32 = 0.16;
const REAR_FEMUR: f32 = -0.1;
const STANCE_SHIN: f32 = 0.14;
/// Pelvis contribution toward the jab side (fraction of [`TORSO_TURN`]).
const HIP_TURN: f32 = 0.35;

/// Boxing jab: chamber, snap to a relative target, then recover to guard.
///
/// Progress is cyclic. Knobs expose *semantic* fight-pose amounts. Humanoid (and later)
/// rig impls map those onto concrete axes — including bind-pose sign corrections.
#[derive(Debug, Clone)]
pub struct Jab<Rig> {
	/// The side of the body that is throwing the jab.
	pub side: Side,
	/// The extent of the backswing of the jab (all preparatory motions).
	pub backswing: f32,
	/// The target of the jab relative to the body's center of mass (root on most humanoid rigs).
	pub target: Vec3,
	_rig: PhantomData<Rig>,
}

impl<Rig> Default for Jab<Rig> {
	fn default() -> Self {
		Self {
			side: Side::Right,
			backswing: DEFAULT_BACKSWING,
			target: DEFAULT_JAB_TARGET,
			_rig: PhantomData,
		}
	}
}

impl<Rig> Jab<Rig> {
	pub fn new(side: Side, backswing: f32, target: Vec3) -> Self {
		Self { side, backswing, target, _rig: PhantomData }
	}

	pub fn with_side(mut self, side: Side) -> Self {
		self.side = side;
		self
	}

	pub fn with_backswing(mut self, backswing: f32) -> Self {
		self.backswing = backswing;
		self
	}

	pub fn with_target(mut self, target: Vec3) -> Self {
		self.target = target;
		self
	}

	pub fn opposite_side(&self) -> Side {
		match self.side {
			Side::Left => Side::Right,
			Side::Right => Side::Left,
		}
	}

	/// Reach scale from the forward component of [`Self::target`].
	pub fn reach_scale(&self) -> f32 {
		let reference = DEFAULT_JAB_TARGET.z.max(f32::EPSILON);
		(self.target.z / reference).clamp(0.4, 1.5)
	}

	/// Chamber envelope: peaks during the preparatory phase, then clears for the snap.
	pub fn chamber_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).cycle();
		let raw = if t < BACKSWING_END {
			smoothstep(t / BACKSWING_END)
		} else if t < EXTEND_END {
			1.0 - smoothstep((t - BACKSWING_END) / (EXTEND_END - BACKSWING_END))
		} else {
			0.0
		};
		raw * self.backswing.clamp(0.0, 2.0)
	}

	/// Extension envelope: snaps out after the chamber, holds briefly, then recovers.
	pub fn extension_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).cycle();
		if t < BACKSWING_END {
			0.0
		} else if t < EXTEND_END {
			let u = (t - BACKSWING_END) / (EXTEND_END - BACKSWING_END);
			// Ease-out snap.
			1.0 - (1.0 - u).powi(2)
		} else if t < HOLD_END {
			1.0
		} else {
			1.0 - smoothstep((t - HOLD_END) / (1.0 - HOLD_END))
		}
	}

	/// Punch-frame humerus roll (~90° from tee). Held for the whole clip.
	pub fn punch_roll(&self, progress: f32) -> f32 {
		let _ = progress;
		PUNCH_ROLL
	}

	/// Arm drop from the tee once punch roll is set (eases slightly at full reach).
	pub fn arm_drop(&self, progress: f32) -> f32 {
		let extend = self.extension_amount(progress);
		ARM_DROP * (1.0 - 0.15 * extend)
	}

	/// Tiny shoulder aim/height — not the punch driver.
	pub fn shoulder_carry(&self, progress: f32) -> f32 {
		let extend = self.extension_amount(progress);
		SHOULDER_CARRY * (0.85 + 0.15 * extend)
	}

	/// Elbow bend on the jab arm (larger = more flexed). Primary punch travel.
	pub fn jab_elbow(&self, progress: f32) -> f32 {
		let chamber = self.chamber_amount(progress);
		let extend = self.extension_amount(progress);
		GUARD_ELBOW * (1.0 - extend) * (1.0 - chamber)
			+ CHAMBER_ELBOW * chamber
			+ EXTEND_ELBOW * extend
	}

	/// Slight sagittal lean — kept small; the punch turn lives in [`Self::torso_turn`].
	pub fn root_lean(&self, progress: f32) -> f32 {
		self.extension_amount(progress) * ROOT_LEAN * self.reach_scale()
	}

	/// Trunk turn into the jab side (peaks with extension; rig distributes across spine/hips).
	pub fn torso_turn(&self, progress: f32) -> f32 {
		let chamber = self.chamber_amount(progress);
		let extend = self.extension_amount(progress);
		// Slight wind-up opposite the punch, then turn into it.
		TORSO_TURN * (extend - 0.35 * chamber) * self.reach_scale()
	}

	/// Pelvis yaw contribution (semantic amount; rig maps onto pelvis bones).
	pub fn hip_turn(&self, progress: f32) -> f32 {
		self.torso_turn(progress) * HIP_TURN
	}

	pub fn lead_femur_swing(&self, progress: f32) -> f32 {
		let stance = 0.55 + 0.45 * self.extension_amount(progress);
		LEAD_FEMUR * stance
	}

	pub fn rear_femur_swing(&self, progress: f32) -> f32 {
		let stance = 0.55 + 0.45 * self.extension_amount(progress);
		REAR_FEMUR * stance
	}

	pub fn stance_shin_flex(&self, progress: f32) -> f32 {
		STANCE_SHIN * (0.7 + 0.3 * self.extension_amount(progress))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn jab_chambers_before_extension() -> anyhow::Result<()> {
		let jab = Jab::<()>::default();
		let mid_chamber = BACKSWING_END * 0.5;
		assert!(jab.chamber_amount(mid_chamber) > 0.4);
		assert!(jab.extension_amount(mid_chamber) < 0.05);
		Ok(())
	}

	#[test]
	fn jab_reaches_near_full_extension() -> anyhow::Result<()> {
		let jab = Jab::<()>::default();
		let peak = (EXTEND_END + HOLD_END) * 0.5;
		assert!(jab.extension_amount(peak) > 0.95);
		assert!(jab.chamber_amount(peak) < 0.05);
		assert!(jab.jab_elbow(peak) < GUARD_ELBOW * 0.25);
		Ok(())
	}

	#[test]
	fn jab_is_elbow_and_torso_driven() -> anyhow::Result<()> {
		let jab = Jab::<()>::default();
		let guard = 0.0;
		let peak = (EXTEND_END + HOLD_END) * 0.5;
		let elbow_delta = (jab.jab_elbow(guard) - jab.jab_elbow(peak)).abs();
		assert!(elbow_delta > 1.0);
		assert!(jab.torso_turn(peak).abs() > jab.torso_turn(guard).abs());
		Ok(())
	}

	#[test]
	fn punch_roll_is_held_near_ninety_degrees() -> anyhow::Result<()> {
		let jab = Jab::<()>::default();
		assert!((jab.punch_roll(0.0) - FRAC_PI_2).abs() < 1e-4);
		assert!((jab.punch_roll(0.47) - jab.punch_roll(0.0)).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn jab_recovers_to_guard_by_cycle_end() -> anyhow::Result<()> {
		let jab = Jab::<()>::default();
		assert!(jab.extension_amount(0.99) < 0.08);
		assert!((jab.jab_elbow(0.99) - GUARD_ELBOW).abs() < 0.2);
		Ok(())
	}

	#[test]
	fn backswing_scales_chamber_depth() -> anyhow::Result<()> {
		let deep = Jab::<()>::default().with_backswing(1.5);
		let shallow = Jab::<()>::default().with_backswing(0.25);
		let t = BACKSWING_END * 0.5;
		assert!(deep.chamber_amount(t) > shallow.chamber_amount(t));
		Ok(())
	}

	#[test]
	fn farther_target_increases_reach_scale() -> anyhow::Result<()> {
		let near = Jab::<()>::default().with_target(Vec3::new(0.0, 0.35, 0.4));
		let far = Jab::<()>::default().with_target(Vec3::new(0.0, 0.35, 1.0));
		assert!(far.reach_scale() > near.reach_scale());
		Ok(())
	}

	#[test]
	fn chamber_deepens_elbow_bend() -> anyhow::Result<()> {
		let jab = Jab::<()>::default();
		let mid_chamber = BACKSWING_END * 0.5;
		assert!(jab.jab_elbow(mid_chamber) > jab.jab_elbow(0.0));
		Ok(())
	}

	#[test]
	fn torso_turns_into_the_extension() -> anyhow::Result<()> {
		let jab = Jab::<()>::default();
		let peak = (EXTEND_END + HOLD_END) * 0.5;
		assert!(jab.torso_turn(peak) > jab.torso_turn(0.0));
		assert!(jab.hip_turn(peak).abs() > 0.0);
		Ok(())
	}
}
