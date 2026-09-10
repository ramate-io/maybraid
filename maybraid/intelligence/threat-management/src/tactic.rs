use bevy::prelude::*;

/// Exclusive response to the current threat set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThreatTactic {
	#[default]
	Ignore,
	Evade,
	Combat,
}

/// Marker that combat is the granted tactic.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CombatSelected;

/// Marker that evasion is the granted tactic.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EvadeSelected;

/// Timed lockout that forces [`ThreatTactic::Ignore`], then expires.
///
/// Unlike [`damage::Downed`], this is a short forget — the next select after
/// [`Self::until`] can grant Combat or Evade again.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct SkillDaze {
	pub until: f32,
}

impl SkillDaze {
	pub fn for_secs(now: f32, duration: f32) -> Self {
		Self { until: now + duration.max(0.0) }
	}

	pub fn active(self, now: f32) -> bool {
		now < self.until
	}
}

/// Emitted once when an entity changes tactic.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThreatTacticChanged {
	pub entity: Entity,
	pub from: ThreatTactic,
	pub to: ThreatTactic,
	pub generation: u64,
}
