//! Plugin: firearm movement writes [`MovementObjective`], combat aims and fires.

use bevy::prelude::*;
use movement_intelligence::MovementIntelligenceSystems;

use crate::combat::engage_firearm_targets;
use crate::movement::write_firearm_movement_objectives;

/// Firearm movement, then combat engage. Movement runs before path replans.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FirearmIntelligenceSystems {
	Movement,
	Engage,
}

pub struct FirearmIntelligencePlugin;

impl Plugin for FirearmIntelligencePlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(
				FirearmIntelligenceSystems::Movement.before(MovementIntelligenceSystems::Replan),
				FirearmIntelligenceSystems::Engage.after(FirearmIntelligenceSystems::Movement),
			),
		)
		.add_systems(
			Update,
			write_firearm_movement_objectives.in_set(FirearmIntelligenceSystems::Movement),
		)
		.add_systems(Update, engage_firearm_targets.in_set(FirearmIntelligenceSystems::Engage));
	}
}
