//! Cut-step parameters for middle-out greedy guillotine partitioning.

/// Absolute step window and optional snap for placing cuts along an axis.
///
/// Each accepted cut advances a low or high front outward from the axis mid by a noise
/// sample in `[step_min, step_max]`. This is a **preferred** size window for max-fitting
/// packing; terminal end remainders are not required to lie in the window. Depth (how many
/// placement attempts) lives on [`crate::guillotine::Guillotine`] /
/// [`crate::guillotine::VariableGuillotine`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuillotineConfig {
	/// Inclusive lower bound on the cut step (world units).
	pub step_min: f32,
	/// Inclusive upper bound on the cut step (world units).
	pub step_max: f32,
	/// Optional snap quantum for cut positions (stable BVH / hashing).
	pub snap_quantum: Option<f32>,
}

impl Default for GuillotineConfig {
	fn default() -> Self {
		Self {
			step_min: 1.0,
			step_max: 8.0,
			snap_quantum: None,
		}
	}
}

impl GuillotineConfig {
	pub const fn new(step_min: f32, step_max: f32) -> Self {
		Self {
			step_min,
			step_max,
			snap_quantum: None,
		}
	}

	pub const fn with_snap_quantum(mut self, quantum: f32) -> Self {
		self.snap_quantum = Some(quantum);
		self
	}

	pub const fn with_step_min(mut self, step_min: f32) -> Self {
		self.step_min = step_min;
		self
	}

	pub const fn with_step_max(mut self, step_max: f32) -> Self {
		self.step_max = step_max;
		self
	}
}
