use bevy::prelude::*;

use crate::CombatTargeting;

/// Ordering point for systems that consume ranked targets.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CombatTargetingSystems {
	Rank,
}

/// Rebalances every [`CombatTargeting`] during [`Update`].
pub struct CombatTargetingPlugin;

impl Plugin for CombatTargetingPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(Update, CombatTargetingSystems::Rank)
			.add_systems(Update, rank_combat_targets.in_set(CombatTargetingSystems::Rank));
	}
}

/// Runs continuously because contact expiry and influence decay are time-based.
pub fn rank_combat_targets(time: Res<Time>, mut combatants: Query<&mut CombatTargeting>) {
	let now = time.elapsed_secs();
	for mut targeting in &mut combatants {
		if targeting.needs_rebalance(now) {
			targeting.rebalance(now);
		}
	}
}
