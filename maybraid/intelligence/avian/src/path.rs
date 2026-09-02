//! Collider-probed polyline plus geometric hints.

use movement_intelligence::{
	MovementCandidate, MovementCandidateHints, MovementLocation, MovementStep,
};

/// Occlusion / clearance labels from Fixed-layer rays.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AvianPathHints {
	pub hide: f32,
	pub sightline: f32,
	pub min_clearance: f32,
}

impl AvianPathHints {
	pub fn as_candidate_hints(self) -> MovementCandidateHints {
		MovementCandidateHints {
			hide: self.hide,
			sightline: self.sightline,
			min_clearance: self.min_clearance,
		}
	}
}

/// Native product of the Avian movement surface.
#[derive(Clone, Debug, PartialEq)]
pub struct AvianColliderPath {
	pub points: Vec<MovementLocation>,
	pub cost: f32,
	pub hints: AvianPathHints,
}

impl AvianColliderPath {
	pub fn into_steps(self) -> Vec<MovementStep> {
		self.points.into_iter().map(MovementStep::MoveTo).collect()
	}

	pub fn into_movement_candidate(self) -> MovementCandidate<MovementStep> {
		let cost = self.cost;
		let hints = self.hints.as_candidate_hints();
		MovementCandidate::new(self.into_steps(), cost, hints)
	}
}
