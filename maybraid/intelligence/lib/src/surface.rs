//! World query: recommend interaction sequences. Implemented as a [`SystemParam`].

use bevy::ecs::entity::Entity;

use crate::candidate::MovementCandidate;
use crate::location::MovementLocation;
use crate::objective::MovementObjective;

/// How much work a surface may do for one query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateBudget {
	pub max_candidates: usize,
	pub max_steps: usize,
	pub horizon: f32,
}

impl Default for CandidateBudget {
	fn default() -> Self {
		Self { max_candidates: 16, max_steps: 2, horizon: 18.0 }
	}
}

/// World-informed movement proposals. Backends are typically [`bevy::ecs::system::SystemParam`].
pub trait MovementIntelligenceSurface<I, A> {
	fn recommend_candidates(
		&mut self,
		from: MovementLocation,
		exclude: &[Entity],
		ability: &A,
		objective: MovementObjective,
		budget: CandidateBudget,
	) -> Vec<MovementCandidate<I>>;

	/// Cheapest candidate by [`MovementCandidate::surface_cost`]. Character scoring may ignore this.
	fn recommend_path(
		&mut self,
		from: MovementLocation,
		exclude: &[Entity],
		ability: &A,
		objective: MovementObjective,
		budget: CandidateBudget,
	) -> Vec<I>
	where
		I: Clone,
	{
		self.recommend_candidates(from, exclude, ability, objective, budget)
			.into_iter()
			.min_by(|a, b| a.surface_cost.total_cmp(&b.surface_cost))
			.map(|candidate| candidate.steps)
			.unwrap_or_default()
	}
}
