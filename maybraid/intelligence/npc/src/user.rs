use bevy::prelude::*;
use tether_intelligence::TetherObjective;

/// Installed NPC mixer policy. Presence means this entity uses the shared
/// threat → tether → meander priority stack.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct NpcIntelligence {
	/// Applied while the tactic is Ignore (and as the restore target after combat).
	pub idle_tether: Option<TetherObjective>,
	/// Stored inner leash for personalities that close a ring. During Combat
	/// the mixer writes this (or [`Self::idle_tether`]) when
	/// [`Self::keep_tether_in_combat`] is set.
	pub engaged_tether: Option<TetherObjective>,
	/// Keep the tether grant on during [`ThreatTactic::Combat`] so a leashed
	/// combatant cannot walk out of its radius while pursuing.
	pub keep_tether_in_combat: bool,
}

impl NpcIntelligence {
	pub fn new(idle_tether: Option<TetherObjective>) -> Self {
		Self { idle_tether, engaged_tether: None, keep_tether_in_combat: false }
	}
}
