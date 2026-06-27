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
	/// Femur medial flex opposing hip swing-out (radians at full hip excursion).
	pub femur_medial_counter: f32,
	/// Humerus swing relative to shoulder swing.
	pub humerus_swing_scale: f32,
	/// Constant forward root lean while walking (radians).
	pub torso_lean: f32,
	/// Shin flex on the stance leg (radians).
	pub knee_stance_bend: f32,
	/// Peak shin flex during swing for toe clearance (radians).
	pub knee_swing_bend: f32,
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
			femur_medial_counter: 0.2,
			humerus_swing_scale: 0.5,
			torso_lean: 0.08,
			knee_stance_bend: 0.1,
			knee_swing_bend: 0.2,
			_rig: PhantomData,
		}
	}
}
