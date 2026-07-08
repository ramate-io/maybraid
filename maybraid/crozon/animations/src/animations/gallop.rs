//! Rig-agnostic gallop cycle parameters.
//!
//! Progress is owned by the controller; [`Gallop`] is a pure sampler at a normalized
//! cycle phase in `[0, 1)`. Quadruped rigs convert to [`QuadrupedGallop`](super::QuadrupedGallop)
//! before applying joint articulation.

#[derive(Debug, Clone, PartialEq)]
pub struct Gallop {
	/// Thigh forward/back stride amplitude (radians).
	pub stride: f32,
	/// Vertical spine bob; 1.0 = tuned quadruped default.
	pub bounce: f32,
	/// Spine pitch in the gait; 1.0 = tuned quadruped default.
	pub spine_pitch: f32,
}

impl Default for Gallop {
	fn default() -> Self {
		Self { stride: 1.0, bounce: 1.0, spine_pitch: 1.0 }
	}
}
