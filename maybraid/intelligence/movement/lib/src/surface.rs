//! World query: recommend interaction sequences. Implemented as a [`SystemParam`].

use bevy::ecs::entity::Entity;
use bevy::prelude::*;
use intelligence_lod::IntelligenceBand;

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
	/// Nearby covering work (vantage / flee) keeps the full fan.
	pub const NEAR_M: f32 = 12.0;
	/// Beyond this, covering queries snap instead of fanning.
	pub const FAR_M: f32 = 28.0;

	/// Per-query cap: character preference, never above `max`.
	pub fn clamp_to(self, max: Self) -> Self {
		Self {
			max_candidates: self.max_candidates.min(max.max_candidates),
			max_steps: self.max_steps.min(max.max_steps),
			horizon: self.horizon.min(max.horizon),
		}
	}

	/// Objective- and distance-scale a clamped budget.
	///
	/// `Reach` is a routing hop / POI snap: one candidate, no detours.
	/// `VantageOn` / `FleeFrom` keep the fan nearby and collapse when far.
	pub fn lod_for(self, from: Vec3, objective: MovementObjective) -> Self {
		let goal = objective.location().point;
		let dist = Vec2::new(from.x - goal.x, from.z - goal.z).length();
		match objective {
			MovementObjective::Reach(_) => {
				Self { max_candidates: 1, max_steps: 1, horizon: self.horizon }
			}
			MovementObjective::EdgeOf(_) => Self {
				max_candidates: self.max_candidates.min(4),
				max_steps: 1,
				horizon: self.horizon,
			},
			MovementObjective::FleeFrom(_) | MovementObjective::VantageOn { .. } => {
				if dist >= Self::FAR_M {
					Self { max_candidates: 1, max_steps: 1, horizon: self.horizon }
				} else if dist >= Self::NEAR_M {
					Self {
						max_candidates: self.max_candidates.min(3),
						max_steps: self.max_steps.min(2),
						horizon: self.horizon,
					}
				} else {
					self
				}
			}
		}
	}

	/// Viewer-axis clamp stacked on [`Self::lod_for`]. Near keeps the query.
	pub fn clamp_viewer(self, band: IntelligenceBand) -> Self {
		match band {
			IntelligenceBand::Near => self,
			IntelligenceBand::Mid => Self {
				max_candidates: self.max_candidates.min(3),
				max_steps: self.max_steps.min(2),
				horizon: self.horizon,
			},
			IntelligenceBand::Far => {
				Self { max_candidates: 1, max_steps: 1, horizon: self.horizon }
			}
		}
	}
}

/// Remaining walk attempts for one `Update`. Surfaces call [`Self::take`] per
/// `probe_walk`; leftover [`crate::ReplanMovement`] stays if a query starves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalkProbeBudget {
	remaining: usize,
	starved: bool,
}

impl WalkProbeBudget {
	pub fn new(remaining: usize) -> Self {
		Self { remaining, starved: false }
	}

	pub fn unlimited() -> Self {
		Self { remaining: usize::MAX, starved: false }
	}

	pub fn remaining(self) -> usize {
		self.remaining
	}

	pub fn is_exhausted(self) -> bool {
		self.remaining == 0
	}

	pub fn is_starved(self) -> bool {
		self.starved
	}

	/// Spend one walk probe. Returns `false` and marks starved when empty.
	pub fn take(&mut self) -> bool {
		if self.remaining == 0 {
			self.starved = true;
			return false;
		}
		if self.remaining != usize::MAX {
			self.remaining -= 1;
		}
		true
	}
}

/// Frame-cost ceiling for every mover. Characters pick their own budget at or below this.
///
/// [`Self::max_replans_per_frame`] caps how many markers start. [`Self::max_walk_probes_per_frame`]
/// caps Avian walk attempts so one `VantageOn` cannot burn the drain. Leftover
/// [`crate::ReplanMovement`] stays for later frames. Do not timer the replan set.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct MovementIntelligenceLimits {
	pub max_budget: CandidateBudget,
	pub max_replans_per_frame: usize,
	pub max_walk_probes_per_frame: usize,
}

impl Default for MovementIntelligenceLimits {
	fn default() -> Self {
		Self {
			max_budget: CandidateBudget { max_candidates: 32, max_steps: 4, horizon: 40.0 },
			max_replans_per_frame: 8,
			max_walk_probes_per_frame: 16,
		}
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

	/// Same as [`Self::recommend_candidates`], spending [`WalkProbeBudget`].
	///
	/// Default ignores the probe cap. Avian counts one take per walk attempt.
	fn recommend_candidates_budgeted(
		&mut self,
		from: MovementLocation,
		exclude: &[Entity],
		ability: &A,
		objective: MovementObjective,
		budget: CandidateBudget,
		probes: &mut WalkProbeBudget,
	) -> Vec<MovementCandidate<I>> {
		let _ = probes;
		self.recommend_candidates(from, exclude, ability, objective, budget)
	}

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

	#[test]
	fn default_limits_drain_replans_instead_of_taking_everyone() -> anyhow::Result<()> {
		let limits = MovementIntelligenceLimits::default();
		anyhow::ensure!(limits.max_replans_per_frame > 0);
		anyhow::ensure!(limits.max_replans_per_frame < usize::MAX);
		anyhow::ensure!(limits.max_walk_probes_per_frame >= limits.max_replans_per_frame);
		Ok(())
	}

	#[test]
	fn reach_lod_is_one_snap() -> anyhow::Result<()> {
		let full = CandidateBudget { max_candidates: 8, max_steps: 3, horizon: 28.0 };
		let reach = full.lod_for(
			Vec3::ZERO,
			MovementObjective::Reach(MovementLocation::new(Vec3::X * 20.0, 1.0)),
		);
		anyhow::ensure!(reach.max_candidates == 1);
		anyhow::ensure!(reach.max_steps == 1);
		Ok(())
	}

	#[test]
	fn covering_lod_collapses_when_far() -> anyhow::Result<()> {
		let full = CandidateBudget { max_candidates: 8, max_steps: 3, horizon: 40.0 };
		let far = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::X * 40.0, 1.0),
			hide_weight: 1.0,
			sightline_weight: 1.0,
		};
		let near = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::X * 4.0, 1.0),
			hide_weight: 1.0,
			sightline_weight: 1.0,
		};
		let far_budget = full.lod_for(Vec3::ZERO, far);
		let near_budget = full.lod_for(Vec3::ZERO, near);
		anyhow::ensure!(far_budget.max_candidates == 1);
		anyhow::ensure!(far_budget.max_steps == 1);
		anyhow::ensure!(near_budget.max_candidates == 8);
		anyhow::ensure!(near_budget.max_steps == 3);
		Ok(())
	}

	#[test]
	fn clamp_viewer_shrinks_mid_and_far_on_top_of_covering_lod() -> anyhow::Result<()> {
		use intelligence_lod::IntelligenceBand;

		let full = CandidateBudget { max_candidates: 8, max_steps: 3, horizon: 40.0 };
		let covering = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::X * 4.0, 1.0),
			hide_weight: 1.0,
			sightline_weight: 1.0,
		};
		let near_covering = full.lod_for(Vec3::ZERO, covering);
		anyhow::ensure!(near_covering.max_candidates == 8);
		let mid = near_covering.clamp_viewer(IntelligenceBand::Mid);
		anyhow::ensure!(mid.max_candidates == 3);
		anyhow::ensure!(mid.max_steps == 2);
		let far = near_covering.clamp_viewer(IntelligenceBand::Far);
		anyhow::ensure!(far.max_candidates == 1);
		anyhow::ensure!(far.max_steps == 1);
		anyhow::ensure!(near_covering.clamp_viewer(IntelligenceBand::Near) == near_covering);
		Ok(())
	}

	#[test]
	fn walk_probe_budget_starves_instead_of_wrapping() -> anyhow::Result<()> {
		let mut probes = WalkProbeBudget::new(1);
		anyhow::ensure!(probes.take());
		anyhow::ensure!(!probes.take());
		anyhow::ensure!(probes.is_starved());
		Ok(())
	}
}
