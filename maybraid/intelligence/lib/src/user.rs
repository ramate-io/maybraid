//! Entity-installed movement brain. Higher-order systems write [`Self::objective`] and [`ReplanMovement`].

use bevy::prelude::*;

use crate::ability::MovementAbility;
use crate::candidate::MovementCandidate;
use crate::objective::MovementObjective;
use crate::step::{MovementDrive, MovementStep};

/// Per-user scoring knobs. Budget and standoffs live on [`MovementAbility`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovementIntelligenceSettings {
	pub stuck_timeout: f32,
	pub weight_surface: f32,
	pub weight_hide: f32,
	pub weight_sightline: f32,
	/// Cost of using the mover's full tolerated fall, in surface-cost units.
	pub weight_fall: f32,
}

impl Default for MovementIntelligenceSettings {
	fn default() -> Self {
		Self {
			stuck_timeout: 1.6,
			weight_surface: 1.0,
			weight_hide: 1.0,
			weight_sightline: 1.0,
			weight_fall: 6.0,
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
		let fall = self.settings.weight_fall * candidate.hints.fall_risk;
		surface + fall - hide - sight
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

	pub fn pick_best_candidate(
		&self,
		candidates: impl IntoIterator<Item = MovementCandidate<I>>,
	) -> Option<MovementCandidate<I>> {
		let mut best: Option<(f32, MovementCandidate<I>)> = None;
		for candidate in candidates {
			let score = self.score_candidate(&candidate);
			let take = best.as_ref().is_none_or(|(best_score, _)| score < *best_score);
			if take {
				best = Some((score, candidate));
			}
		}
		best.map(|(_, candidate)| candidate)
	}
}

/// Result of advancing a plan toward the next [`crate::MovementDrive`] target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MovementDriveResult {
	Wish(Vec3),
	Hold,
	/// Planner is not approaching the current step. Drive still writes `wish`;
	/// local unstick belongs to the motor (strafe / jump / backup), not an immediate replan.
	Stuck {
		wish: Vec3,
	},
}

impl<I, A> MovementIntelligence<I, A>
where
	I: MovementDrive + Send + Sync + 'static,
	A: Send + Sync + 'static,
{
	pub fn drive(&mut self, dt: f32, position: Vec3) -> MovementDriveResult {
		loop {
			if self.at_plan_end() {
				return MovementDriveResult::Hold;
			}
			let Some(target) = self.plan.get(self.cursor).and_then(I::drive_target) else {
				self.cursor += 1;
				continue;
			};
			if target.contains(position) {
				self.cursor += 1;
				self.stuck_seconds = 0.0;
				self.last_goal_distance = f32::MAX;
				continue;
			}
			let dist = target.approach_distance(position);
			let wish = target.xz_wish_from(position);
			if dist + 0.08 < self.last_goal_distance {
				self.last_goal_distance = dist;
				self.stuck_seconds = 0.0;
			} else {
				self.stuck_seconds += dt;
				if self.stuck_seconds >= self.settings.stuck_timeout {
					self.stuck_seconds = 0.0;
					return MovementDriveResult::Stuck { wish };
				}
			}
			return MovementDriveResult::Wish(wish);
		}
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
			MovementCandidateHints { hide, sightline, min_clearance: 1.0, fall_risk: 0.0 },
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

	#[test]
	fn score_prefers_a_short_detour_to_a_tolerated_fall() -> anyhow::Result<()> {
		let brain = MovementIntelligence::new(vantage());
		let mut risky = candidate(0.0, 0.0, 5.0);
		risky.hints.fall_risk = 0.5;
		let safe = candidate(0.0, 0.0, 7.0);
		assert!(brain.score_candidate(&safe) < brain.score_candidate(&risky));
		Ok(())
	}

	#[test]
	fn drive_holds_when_plan_is_empty() -> anyhow::Result<()> {
		let mut brain = MovementIntelligence::new(vantage());
		assert_eq!(brain.drive(0.016, Vec3::ZERO), MovementDriveResult::Hold);
		Ok(())
	}

	#[test]
	fn drive_advances_when_inside_waypoint() -> anyhow::Result<()> {
		let mut brain = MovementIntelligence::new(vantage());
		brain.adopt_plan(vec![MovementStep::MoveTo(MovementLocation::new(Vec3::ZERO, 1.0))]);
		assert_eq!(brain.drive(0.016, Vec3::new(0.2, 0.0, 0.1)), MovementDriveResult::Hold);
		assert!(brain.at_plan_end());
		Ok(())
	}

	#[test]
	fn drive_reports_stuck_when_distance_does_not_fall() -> anyhow::Result<()> {
		let mut brain = MovementIntelligence::new(vantage());
		brain.settings.stuck_timeout = 0.08;
		brain.adopt_plan(vec![MovementStep::MoveTo(MovementLocation::new(Vec3::X * 8.0, 0.4))]);
		let mut saw_stuck = false;
		for _ in 0..20 {
			if matches!(brain.drive(0.016, Vec3::ZERO), MovementDriveResult::Stuck { .. }) {
				saw_stuck = true;
			}
		}
		assert!(saw_stuck);
		Ok(())
	}
}
