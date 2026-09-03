//! Test doubles for [`crate::MovementIntelligenceSurface`] without a physics world.

use bevy::prelude::*;

use crate::candidate::{MovementCandidate, MovementCandidateHints};
use crate::location::MovementLocation;
use crate::objective::MovementObjective;
use crate::step::MovementStep;
use crate::surface::{CandidateBudget, MovementIntelligenceSurface};
use crate::MovementSheet;

/// Straight-line `MoveTo` the objective location. No colliders.
pub struct StraightLineSurface;

impl<A: MovementSheet> MovementIntelligenceSurface<MovementStep, A> for StraightLineSurface {
	fn recommend_candidates(
		&mut self,
		from: MovementLocation,
		_exclude: &[Entity],
		ability: &A,
		objective: MovementObjective,
		_budget: CandidateBudget,
	) -> Vec<MovementCandidate<MovementStep>> {
		let goal = objective.location();
		let dest = MovementLocation::new(
			Vec3::new(goal.point.x, from.point.y, goal.point.z),
			goal.radius.max(ability.agent_radius()),
		);
		vec![MovementCandidate::new(
			vec![MovementStep::MoveTo(dest)],
			dest.xz_distance(from.point),
			MovementCandidateHints::default(),
		)]
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ability::MovementAbility;

	#[test]
	fn straight_line_reaches_the_goal() -> anyhow::Result<()> {
		let mut surface = StraightLineSurface;
		let ability = MovementAbility::default();
		let from = MovementLocation::new(Vec3::ZERO, 0.4);
		let objective = MovementObjective::Reach(MovementLocation::new(Vec3::X * 4.0, 0.5));
		let path =
			surface.recommend_path(from, &[], &ability, objective, CandidateBudget::default());
		assert_eq!(path.len(), 1);
		Ok(())
	}
}
