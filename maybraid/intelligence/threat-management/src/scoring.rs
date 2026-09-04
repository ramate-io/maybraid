use bevy::prelude::*;
use threat_intelligence::ThreatKnowledge;

use crate::ThreatTactic;

/// Signed coefficients on the shared health and proximity axes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ThreatManagementElement {
	pub by_health: f32,
	pub by_distance: f32,
}

impl ThreatManagementElement {
	pub const ZERO: Self = Self { by_health: 0.0, by_distance: 0.0 };

	pub const fn new(by_health: f32, by_distance: f32) -> Self {
		Self { by_health, by_distance }
	}

	pub fn score(self, health: f32, proximity: f32) -> f32 {
		self.by_health * health + self.by_distance * proximity
	}
}

/// `1 / (1 + nearest_xz / horizon)`. Empty knowledge is `0`.
pub fn proximity(nearest_xz: Option<f32>, horizon: f32) -> f32 {
	let Some(distance) = nearest_xz.filter(|distance| distance.is_finite()) else {
		return 0.0;
	};
	if !horizon.is_finite() || horizon <= 0.0 {
		return if distance <= 0.0 { 1.0 } else { 0.0 };
	}
	1.0 / (1.0 + distance / horizon)
}

pub fn nearest_known_xz(knowledge: &ThreatKnowledge, from: Vec3) -> Option<f32> {
	knowledge
		.iter()
		.map(|known| {
			Vec2::new(known.last_known_position.x - from.x, known.last_known_position.z - from.z)
				.length()
		})
		.min_by(|a, b| a.total_cmp(b))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TacticScores {
	pub ignore: f32,
	pub evade: f32,
	pub combat: f32,
}

impl TacticScores {
	pub fn of(self, tactic: ThreatTactic) -> f32 {
		match tactic {
			ThreatTactic::Ignore => self.ignore,
			ThreatTactic::Evade => self.evade,
			ThreatTactic::Combat => self.combat,
		}
	}
}

pub fn score_tactics(
	ignore: ThreatManagementElement,
	evade: ThreatManagementElement,
	combat: ThreatManagementElement,
	health: f32,
	proximity: f32,
) -> TacticScores {
	TacticScores {
		ignore: ignore.score(health, proximity),
		evade: evade.score(health, proximity),
		combat: combat.score(health, proximity),
	}
}

/// `score_new / score_old >= new / old`, evaluated with cross multiplication.
pub fn meets_commitment(challenger: f32, current: f32, commitment: (f32, f32)) -> bool {
	let (new_req, old_req) = commitment;
	if !challenger.is_finite()
		|| !current.is_finite()
		|| !new_req.is_finite()
		|| !old_req.is_finite()
	{
		return false;
	}
	challenger * old_req >= new_req * current
}

const PREFERENCE: [ThreatTactic; 3] =
	[ThreatTactic::Combat, ThreatTactic::Evade, ThreatTactic::Ignore];

fn preference_rank(tactic: ThreatTactic) -> u8 {
	match tactic {
		ThreatTactic::Combat => 2,
		ThreatTactic::Evade => 1,
		ThreatTactic::Ignore => 0,
	}
}

fn available(tactic: ThreatTactic, combat: bool, evade: bool) -> bool {
	match tactic {
		ThreatTactic::Ignore => true,
		ThreatTactic::Combat => combat,
		ThreatTactic::Evade => evade,
	}
}

fn best_of(
	scores: TacticScores,
	combat: bool,
	evade: bool,
	current: ThreatTactic,
	mut eligible: impl FnMut(ThreatTactic) -> bool,
) -> ThreatTactic {
	PREFERENCE
		.into_iter()
		.filter(|tactic| available(*tactic, combat, evade) && eligible(*tactic))
		.max_by(|a, b| {
			scores
				.of(*a)
				.total_cmp(&scores.of(*b))
				.then_with(|| (*a == current).cmp(&(*b == current)))
				.then_with(|| preference_rank(*a).cmp(&preference_rank(*b)))
		})
		.unwrap_or(ThreatTactic::Ignore)
}

/// Picks the exclusive tactic. Empty knowledge always forces Ignore.
pub fn select_tactic(
	knowledge_empty: bool,
	current: ThreatTactic,
	combat_available: bool,
	evade_available: bool,
	scores: TacticScores,
	commitment: (f32, f32),
) -> ThreatTactic {
	if knowledge_empty {
		return ThreatTactic::Ignore;
	}
	let current = if available(current, combat_available, evade_available) {
		current
	} else {
		ThreatTactic::Ignore
	};
	if current == ThreatTactic::Ignore {
		return best_of(scores, combat_available, evade_available, current, |_| true);
	}
	best_of(scores, combat_available, evade_available, current, |tactic| {
		tactic == current || meets_commitment(scores.of(tactic), scores.of(current), commitment)
	})
}
