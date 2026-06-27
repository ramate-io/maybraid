//! Walk cycle tuning parameters.
//!
//! Progress is owned by the controller; [`Walk`] is a pure sampler at a normalized
//! cycle phase in `[0, 1)`.

use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct Walk<Rig> {
	/// Humerus flex bias that keeps forearms slightly forward of the torso.
	pub arm_down: f32,
	/// Baseline elbow bend while the arm hangs at the side.
	pub elbow_bend: f32,
	/// Extra elbow bend on the back-swing of the arm.
	pub elbow_pump: f32,
	/// Oscillating elbow contribution over the stride.
	pub elbow_cycle: f32,
	/// Shoulder swing amplitude opposite the matching leg.
	pub shoulder_swing: f32,
	/// Vertical shoulder bounce over the stride (kept small; lift comes from hips).
	pub shoulder_lift: f32,
	/// Pelvis sagittal swing in phase with the femur (radians).
	pub hip_swing: f32,
	/// Vertical pelvis bounce over the stride (primary source of body lift).
	pub hip_lift: f32,
	/// Femur forward/back swing amplitude.
	pub stride: f32,
	/// Humerus swing relative to shoulder swing.
	pub humerus_swing_scale: f32,
	/// Constant forward root lean while walking (radians).
	pub torso_lean: f32,
	/// Shin flex at full leg extension (radians).
	pub knee_extended: f32,
	/// Extra shin flex during swing phase for toe clearance (radians).
	pub knee_clearance: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> Default for Walk<Rig> {
	fn default() -> Self {
		Self {
			arm_down: 0.35,
			elbow_bend: 0.35,
			elbow_pump: 0.12,
			elbow_cycle: 0.05,
			shoulder_swing: 0.08,
			shoulder_lift: 0.0,
			hip_swing: 0.1,
			hip_lift: 0.15,
			stride: 0.46,
			humerus_swing_scale: 0.5,
			torso_lean: 0.08,
			knee_extended: 0.35,
			knee_clearance: 0.2,
			_rig: PhantomData,
		}
	}
}
