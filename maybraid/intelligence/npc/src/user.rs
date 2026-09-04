use bevy::prelude::*;
use tether_intelligence::TetherObjective;

/// Installed NPC mixer policy. Presence means this entity uses the shared
/// threat → tether → meander priority stack.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct NpcIntelligence {
	/// Applied while the tactic is Ignore (and as the restore target after combat).
	pub idle_tether: Option<TetherObjective>,
	/// Applied while the tactic is Combat. Movement still belongs to firearm
	/// unless [`Self::keep_tether_in_combat`] is set.
	pub engaged_tether: Option<TetherObjective>,
	/// When true, tether may write during Combat (Hunt / close-the-ring).
	pub keep_tether_in_combat: bool,
}

impl NpcIntelligence {
	pub fn new(idle_tether: Option<TetherObjective>) -> Self {
		Self { idle_tether, engaged_tether: None, keep_tether_in_combat: false }
	}
}
