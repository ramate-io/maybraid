//! Quadruped gallop tuning parameters.
//!
//! [`QuadrupedGallop`] holds art-level quadruped knobs. Rig-agnostic callers use [`Gallop`](super::Gallop)
//! and convert via [`QuadrupedGallop::from_gallop`].

use std::f32::consts::PI;
use std::marker::PhantomData;

use super::Gallop;

#[derive(Debug, Clone, PartialEq)]
pub struct QuadrupedGallop<Rig> {
	pub shoulder_swing: f32,
	pub shoulder_lift: f32,
	pub hip_swing: f32,
	pub hip_lift: f32,
	pub stride: f32,
	pub knee_neutral: f32,
	pub knee_contracted: f32,
	pub knee_extended: f32,
	pub spine_swing: f32,
	pub neck_swing: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> Default for QuadrupedGallop<Rig> {
	fn default() -> Self {
		Self {
			shoulder_swing: 0.12,
			shoulder_lift: 0.05,
			hip_swing: 0.14,
			hip_lift: 0.06,
			stride: 1.0,
			knee_neutral: PI * 0.5,
			knee_contracted: 2.0,
			knee_extended: 0.3,
			spine_swing: 0.08,
			neck_swing: 0.04,
			_rig: PhantomData,
		}
	}
}

impl<Rig> QuadrupedGallop<Rig> {
	/// Scale the tuned quadruped template from rig-agnostic [`Gallop`] knobs.
	pub fn from_gallop(gallop: &Gallop) -> Self {
		let template = Self::default();
		let reference = Gallop::default();
		let stride_scale = gallop.stride / reference.stride;

		let knee_neutral_delta = template.knee_neutral - template.knee_extended;
		let knee_contracted_delta = template.knee_contracted - template.knee_neutral;

		Self {
			stride: template.stride * stride_scale,
			shoulder_lift: template.shoulder_lift * gallop.bounce,
			hip_lift: template.hip_lift * gallop.bounce,
			shoulder_swing: template.shoulder_swing * gallop.spine_pitch,
			hip_swing: template.hip_swing * gallop.spine_pitch,
			spine_swing: template.spine_swing * gallop.spine_pitch,
			neck_swing: template.neck_swing * gallop.spine_pitch,
			knee_extended: template.knee_extended,
			knee_neutral: template.knee_extended + knee_neutral_delta * stride_scale,
			knee_contracted: template.knee_extended
				+ knee_neutral_delta * stride_scale
				+ knee_contracted_delta * stride_scale,
			_rig: PhantomData,
		}
	}
}
