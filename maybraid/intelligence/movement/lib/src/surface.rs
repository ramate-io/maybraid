//! World query: recommend interaction sequences. Implemented as a [`SystemParam`].

use bevy::ecs::entity::Entity;
use bevy::prelude::*;

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

impl CandidateBudget {
	/// Per-query cap: character preference, never above `max`.
	pub fn clamp_to(self, max: Self) -> Self {
		Self {
			max_candidates: self.max_candidates.min(max.max_candidates),
			max_steps: self.max_steps.min(max.max_steps),
			horizon: self.horizon.min(max.horizon),
		}
	}
}

/// Frame-cost ceiling for every mover. Characters pick their own budget at or below this.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct MovementIntelligenceLimits {
	pub max_budget: CandidateBudget,
}

impl Default for MovementIntelligenceLimits {
	fn default() -> Self {
		Self { max_budget: CandidateBudget { max_candidates: 32, max_steps: 4, horizon: 40.0 } }
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn clamp_to_takes_the_min_of_each_axis() -> anyhow::Result<()> {
		let character = CandidateBudget { max_candidates: 64, max_steps: 8, horizon: 80.0 };
		let limits = CandidateBudget { max_candidates: 32, max_steps: 4, horizon: 40.0 };
		let clamped = character.clamp_to(limits);
		assert_eq!(clamped.max_candidates, 32);
		assert_eq!(clamped.max_steps, 4);
		assert!((clamped.horizon - 40.0).abs() < 1e-4);
		Ok(())
	}
}
