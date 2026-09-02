//! Surface-labeled plan candidates. The character fold selects; the surface does not.

/// Geometric / structural annotations from a surface probe.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MovementCandidateHints {
	/// 0..=1 occlusion of the body from the objective location.
	pub hide: f32,
	/// 0..=1 clearance of an eye-height ray toward the objective location.
	pub sightline: f32,
	pub min_clearance: f32,
}

/// One recommended interaction sequence plus surface labels.
#[derive(Clone, Debug, PartialEq)]
pub struct MovementCandidate<I> {
	pub steps: Vec<I>,
	/// Geometric cost only (length, clearance). Character scoring folds this in.
	pub surface_cost: f32,
	pub hints: MovementCandidateHints,
}

impl<I> MovementCandidate<I> {
	pub fn new(steps: Vec<I>, surface_cost: f32, hints: MovementCandidateHints) -> Self {
		Self { steps, surface_cost, hints }
	}
}
