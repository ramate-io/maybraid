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
const GUARD_ELBOW: f32 = 1.45;
const CHAMBER_ELBOW: f32 = 1.65;
const EXTEND_ELBOW: f32 = 0.08;

/// Sagittal reach from a high guard toward the target (`1.0` = full jab).
const GUARD_FORWARD: f32 = 0.4;
const CHAMBER_FORWARD: f32 = 0.12;
const EXTEND_FORWARD: f32 = 1.15;

/// How tightly the guard arm folds in from the T-pose toward the chest.
const GUARD_FOLD: f32 = 0.85;
const GUARD_WRAP: f32 = 0.7;

const ROOT_LEAN: f32 = 0.14;
const LEAD_FEMUR: f32 = 0.2;
const REAR_FEMUR: f32 = -0.14;
const STANCE_SHIN: f32 = 0.18;

/// Boxing jab: chamber, snap to a relative target, then recover to guard.
///
/// Progress is cyclic. Knobs expose *semantic* fight-pose amounts (forward reach,
/// elbow bend, chest fold). Humanoid (and later) rig impls map those onto concrete
/// swing/flex axes — including any sign corrections for bind-pose quirks.
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

	/// Vertical aim offset relative to the default chest-height target.
	pub fn height_bias(&self) -> f32 {
		(self.target.y - DEFAULT_JAB_TARGET.y) * 0.85
	}

	/// Lateral aim from the target's X component (toward/across centerline).
	pub fn lateral_aim(&self) -> f32 {
		self.target.x * 0.65
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

	pub fn root_lean(&self, progress: f32) -> f32 {
		self.extension_amount(progress) * ROOT_LEAN * self.reach_scale()
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

	/// Sagittal forward amount for the jab arm (`0` = tucked back, larger = more ventral).
	pub fn jab_forward(&self, progress: f32) -> f32 {
		let chamber = self.chamber_amount(progress);
		let extend = self.extension_amount(progress);
		GUARD_FORWARD * (1.0 - chamber.max(extend))
			+ CHAMBER_FORWARD * chamber
			+ EXTEND_FORWARD * extend * self.reach_scale()
	}

	/// Elbow bend on the jab arm (larger = more flexed).
	pub fn jab_elbow(&self, progress: f32) -> f32 {
		let chamber = self.chamber_amount(progress);
		let extend = self.extension_amount(progress);
		GUARD_ELBOW * (1.0 - extend) * (1.0 - chamber)
			+ CHAMBER_ELBOW * chamber
			+ EXTEND_ELBOW * extend
	}

	/// How much the jab arm drops/raises from the default chest line.
	pub fn jab_height(&self, progress: f32) -> f32 {
		self.height_bias() * self.extension_amount(progress)
	}

	/// Lateral aim blend for the jab arm (positive = toward +X in target space).
	pub fn jab_lateral(&self, progress: f32) -> f32 {
		self.lateral_aim() * self.extension_amount(progress)
	}

	/// Elbow bend on the covering / guard arm.
	pub fn guard_elbow(&self, progress: f32) -> f32 {
		GUARD_ELBOW + self.extension_amount(progress) * 0.12
	}

	/// How tightly the guard arm is clutched in from the T-pose toward the chest.
	pub fn guard_fold(&self, progress: f32) -> f32 {
		GUARD_FOLD + self.extension_amount(progress) * 0.1
	}

	/// How far the guard forearm wraps across the sternum.
	pub fn guard_wrap(&self, progress: f32) -> f32 {
		GUARD_WRAP + self.extension_amount(progress) * 0.08
	}

	/// Modest forward hold so the guard fist stays in front of the face, not out to the side.
	pub fn guard_forward(&self, progress: f32) -> f32 {
		GUARD_FORWARD + self.extension_amount(progress) * 0.06
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
		assert!(jab.jab_forward(peak) > jab.jab_forward(0.0));
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
	fn chamber_pulls_jab_arm_back_from_guard() -> anyhow::Result<()> {
		let jab = Jab::<()>::default();
		let mid_chamber = BACKSWING_END * 0.5;
		assert!(jab.jab_forward(mid_chamber) < jab.jab_forward(0.0));
		Ok(())
	}
}
