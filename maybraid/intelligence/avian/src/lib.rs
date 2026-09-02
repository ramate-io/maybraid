//! Avian collider-backed [`MovementIntelligenceSurface`].
//!
//! Produces [`AvianColliderPath`] values; interactions convert with [`From`].

mod path;
mod surface;

use bevy::prelude::*;

use movement_intelligence::{MovementCandidate, MovementIntelligenceSurface, MovementStep};

pub use path::{AvianColliderPath, AvianPathHints};
pub use surface::AvianMovementSurface;

impl From<AvianColliderPath> for MovementCandidate<MovementStep> {
	fn from(path: AvianColliderPath) -> Self {
		path.into_movement_candidate()
	}
}

impl From<AvianColliderPath> for Vec<MovementStep> {
	fn from(path: AvianColliderPath) -> Self {
		path.into_steps()
	}
}

impl<I, A> MovementIntelligenceSurface<I, A> for AvianMovementSurface<'_, '_>
where
	MovementCandidate<I>: From<AvianColliderPath>,
	A: movement_intelligence::MovementBody
		+ movement_intelligence::Covering
		+ Send
		+ Sync
		+ 'static,
{
	fn recommend_candidates(
		&mut self,
		from: movement_intelligence::MovementLocation,
		exclude: &[Entity],
		ability: &A,
		objective: movement_intelligence::MovementObjective,
		budget: movement_intelligence::CandidateBudget,
	) -> Vec<MovementCandidate<I>> {
		self.collider_paths(from, exclude, ability, objective, budget)
			.into_iter()
			.map(MovementCandidate::from)
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use movement_intelligence::{MovementCandidateHints, MovementLocation};

	#[test]
	fn avian_collider_path_converts_to_move_to() -> anyhow::Result<()> {
		let path = AvianColliderPath {
			points: vec![MovementLocation::new(Vec3::new(2.0, 0.0, 0.0), 0.5)],
			cost: 2.0,
			hints: AvianPathHints { hide: 1.0, sightline: 0.5, min_clearance: 0.4 },
		};
		let candidate = MovementCandidate::<MovementStep>::from(path);
		assert_eq!(candidate.steps.len(), 1);
		assert_eq!(candidate.surface_cost, 2.0);
		assert_eq!(
			candidate.hints,
			MovementCandidateHints { hide: 1.0, sightline: 0.5, min_clearance: 0.4 }
		);
		Ok(())
	}
}
