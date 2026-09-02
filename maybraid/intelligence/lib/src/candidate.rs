//! Surface-labeled plan candidates. The character fold selects; the surface does not.

use crate::objective::MovementObjective;

/// Geometric / structural annotations from a surface probe.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MovementCandidateHints {
	/// 0..=1 occlusion of the body from the objective location.
	pub hide: f32,
	/// 0..=1 clearance of an eye-height ray toward the objective location.
	pub sightline: f32,
	pub min_clearance: f32,
}

impl MovementCandidateHints {
	/// Lower is better. Path length is omitted so standpoints can be ranked before walk probes.
	pub fn covering_score(self, objective: MovementObjective) -> f32 {
		-(objective.hide_weight() * self.hide + objective.sightline_weight() * self.sightline)
	}
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::location::MovementLocation;
	use bevy::prelude::*;

	#[test]
	fn covering_score_ranks_peek_ahead_of_open() -> anyhow::Result<()> {
		let objective = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::ZERO, 1.0),
			hide_weight: 10.0,
			sightline_weight: 12.0,
		};
		let peek = MovementCandidateHints { hide: 1.0, sightline: 1.0, min_clearance: 1.0 };
		let open = MovementCandidateHints { hide: 0.0, sightline: 1.0, min_clearance: 1.0 };
		assert!(peek.covering_score(objective) < open.covering_score(objective));
		Ok(())
	}
}
