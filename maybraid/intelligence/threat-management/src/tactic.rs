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

/// Emitted once when an entity changes tactic.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThreatTacticChanged {
	pub entity: Entity,
	pub from: ThreatTactic,
	pub to: ThreatTactic,
	pub generation: u64,
}
