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
	pub hind_bound_pitch: f32,
	pub front_bound_pitch: f32,
	pub neck_follow: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> Default for QuadrupedGallop<Rig> {
	fn default() -> Self {
		Self {
			shoulder_swing: 0.14,
			shoulder_lift: 0.07,
			hip_swing: 0.16,
			hip_lift: 0.08,
			stride: 1.1,
			knee_neutral: PI * 0.5,
			knee_contracted: 2.25,
			knee_extended: 0.25,
			hind_bound_pitch: 0.12,
			front_bound_pitch: 0.10,
			neck_follow: 0.5,
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
			hind_bound_pitch: template.hind_bound_pitch * gallop.bound_pitch,
			front_bound_pitch: template.front_bound_pitch * gallop.bound_pitch,
			shoulder_swing: template.shoulder_swing,
			hip_swing: template.hip_swing,
			knee_extended: template.knee_extended,
			knee_neutral: template.knee_extended + knee_neutral_delta * stride_scale,
			knee_contracted: template.knee_extended
				+ knee_neutral_delta * stride_scale
				+ knee_contracted_delta * stride_scale,
			neck_follow: template.neck_follow,
			_rig: PhantomData,
		}
	}
}
