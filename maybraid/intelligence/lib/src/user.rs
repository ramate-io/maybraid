//! Entity-installed movement brain. Higher-order systems write [`Self::objective`] and [`ReplanMovement`].

use bevy::prelude::*;

use crate::ability::MovementAbility;
use crate::candidate::MovementCandidate;
use crate::objective::MovementObjective;
use crate::step::MovementStep;
use crate::surface::CandidateBudget;

/// Per-user heuristic and probe knobs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovementIntelligenceSettings {
	pub candidate_budget: CandidateBudget,
	pub stuck_timeout: f32,
	pub weight_surface: f32,
	pub weight_hide: f32,
	pub weight_sightline: f32,
}

impl Default for MovementIntelligenceSettings {
	fn default() -> Self {
		Self {
			candidate_budget: CandidateBudget::default(),
			stuck_timeout: 1.6,
			weight_surface: 1.0,
			weight_hide: 1.0,
			weight_sightline: 1.0,
		}
	}
}

/// Capsule using movement intelligence. Inserting it is the FirearmUser-style install.
///
/// This crate does not follow other entities. A higher-order system writes [`Self::objective`]
/// and inserts [`ReplanMovement`] when it wants a new plan.
#[derive(Component, Debug, Clone)]
pub struct MovementIntelligence<I = MovementStep, A = MovementAbility>
where
	I: Send + Sync + 'static,
	A: Send + Sync + 'static,
{
	pub objective: MovementObjective,
	pub settings: MovementIntelligenceSettings,
	pub ability: A,
	pub plan: Vec<I>,
	pub cursor: usize,
	pub stuck_seconds: f32,
	pub last_goal_distance: f32,
}

impl MovementIntelligence {
	pub fn new(objective: MovementObjective) -> Self {
		Self {
			objective,
			settings: MovementIntelligenceSettings::default(),
			ability: MovementAbility::default(),
			plan: Vec::new(),
			cursor: 0,
			stuck_seconds: 0.0,
			last_goal_distance: f32::MAX,
		}
	}
}

impl<I, A> MovementIntelligence<I, A>
where
	I: Send + Sync + 'static,
	A: Send + Sync + 'static,
{
	/// Lower is better. Folds surface cost with objective hide / sightline, scaled by settings.
	pub fn score_candidate(&self, candidate: &MovementCandidate<I>) -> f32 {
		let surface = self.settings.weight_surface * candidate.surface_cost;
		let hide = self.settings.weight_hide * self.objective.hide_weight() * candidate.hints.hide;
		let sight = self.settings.weight_sightline
			* self.objective.sightline_weight()
			* candidate.hints.sightline;
		surface - hide - sight
	}

	pub fn adopt_plan(&mut self, steps: Vec<I>) {
		self.plan = steps;
		self.cursor = 0;
		self.stuck_seconds = 0.0;
		self.last_goal_distance = f32::MAX;
	}

	pub fn at_plan_end(&self) -> bool {
		self.cursor >= self.plan.len()
	}
}

/// Higher-order request to rebuild [`MovementIntelligence::plan`] for the current objective.
#[derive(Component, Debug, Clone, Copy)]
#[component(storage = "SparseSet")]
pub struct ReplanMovement;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::candidate::MovementCandidateHints;
	use crate::location::MovementLocation;
	use crate::step::MovementStep;

	fn vantage() -> MovementObjective {
		MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::ZERO, 1.0),
			hide_weight: 10.0,
			sightline_weight: 12.0,
		}
	}

	fn candidate(hide: f32, sightline: f32, surface_cost: f32) -> MovementCandidate<MovementStep> {
		MovementCandidate::new(
			vec![MovementStep::MoveTo(MovementLocation::new(Vec3::X, 0.5))],
			surface_cost,
			MovementCandidateHints { hide, sightline, min_clearance: 1.0 },
		)
	}

	#[test]
	fn score_prefers_hide_and_sightline_together() -> anyhow::Result<()> {
		let brain = MovementIntelligence::new(vantage());
		let peek = brain.score_candidate(&candidate(1.0, 1.0, 10.0));
		let cover = brain.score_candidate(&candidate(1.0, 0.0, 10.0));
		let open = brain.score_candidate(&candidate(0.0, 1.0, 10.0));
		assert!(peek < cover, "{peek} vs cover {cover}");
		assert!(peek < open, "{peek} vs open {open}");
		Ok(())
	}
}
