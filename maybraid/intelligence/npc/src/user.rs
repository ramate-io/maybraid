use bevy::prelude::*;
use tether_intelligence::TetherObjective;

/// Installed NPC mixer policy. Presence means this entity uses the shared
/// threat → tether → meander priority stack.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct NpcIntelligence {
	/// Applied while the tactic is Ignore (and as the restore target after combat).
	pub idle_tether: Option<TetherObjective>,
	/// Stored inner leash for personalities that close a ring. The mixer does
	/// not write tether during Combat; a mob lock swaps [`Self::idle_tether`]'s
	/// subject instead.
	pub engaged_tether: Option<TetherObjective>,
	/// Unused by the mixer. Pack lock/release is the mob-level stickiness.
	pub keep_tether_in_combat: bool,
}

impl NpcIntelligence {
	pub fn new(idle_tether: Option<TetherObjective>) -> Self {
		Self { idle_tether, engaged_tether: None, keep_tether_in_combat: false }
	}
}
