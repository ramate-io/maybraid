use std::marker::PhantomData;

use crozon_rigs::RigPose;

/// Remaps linear transition progress into blend weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendCurve {
	Linear,
	#[default]
	SmoothStep,
	EaseIn,
	EaseOut,
	EaseInOut,
}

impl BlendCurve {
	pub fn sample(self, t: f32) -> f32 {
		let t = t.clamp(0.0, 1.0);

		match self {
			Self::Linear => t,
			Self::SmoothStep => t * t * (3.0 - 2.0 * t),
			Self::EaseIn => t * t,
			Self::EaseOut => {
				let inv = 1.0 - t;
				1.0 - inv * inv
			}
			Self::EaseInOut => {
				if t < 0.5 {
					2.0 * t * t
				} else {
					1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
				}
			}
		}
	}
}

/// Transition into an animation from a captured source pose.
///
/// Unlike [`Mix`](super::Mix) or [`Smooth`](super::Smooth), the source pose is fixed at
/// construction time rather than re-sampled each frame.
#[derive(Debug, Clone)]
pub struct Transition<A, R> {
	/// Animation being transitioned into.
	pub animation: A,
	/// Pose captured when the transition began.
	pub from_pose: RigPose,
	/// Linear transition progress before curve remapping (`0.0..=1.0`).
	pub progress: f32,
	/// Curve used to remap [`Self::progress`] into blend weight.
	pub curve: BlendCurve,
	_rig: PhantomData<R>,
}

impl<A, R> Transition<A, R> {
	/// Creates a transition from an explicit captured pose.
	pub fn from_pose(animation: A, from_pose: RigPose, progress: f32) -> Self {
		Self {
			animation,
			from_pose,
			progress: progress.clamp(0.0, 1.0),
			curve: BlendCurve::default(),
			_rig: PhantomData,
		}
	}

	pub fn with_curve(mut self, curve: BlendCurve) -> Self {
		self.curve = curve;
		self
	}

	pub fn with_progress(mut self, progress: f32) -> Self {
		self.progress = progress.clamp(0.0, 1.0);
		self
	}

	pub fn weight(&self) -> f32 {
		self.curve.sample(self.progress)
	}

	pub fn is_complete(&self) -> bool {
		self.progress >= 1.0
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn blend_curve_endpoints() -> anyhow::Result<()> {
		for curve in [
			BlendCurve::Linear,
			BlendCurve::SmoothStep,
			BlendCurve::EaseIn,
			BlendCurve::EaseOut,
			BlendCurve::EaseInOut,
		] {
			assert!(curve.sample(0.0).abs() < 1e-5);
			assert!((curve.sample(1.0) - 1.0).abs() < 1e-5);
		}
		Ok(())
	}

	#[test]
	fn transition_clamps_progress() {
		let transition = Transition::<(), ()>::from_pose((), RigPose::new(), 1.5);
		assert_eq!(transition.progress, 1.0);
	}
}
