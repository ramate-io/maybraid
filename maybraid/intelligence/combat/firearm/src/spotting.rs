//! Adapt firearm-specific settings to the generic spotting→targeting bridge.

use bevy::prelude::*;
use combat_targeting::SpottingTargetingSettings;
use spotting_intelligence::SpottingUser;

use crate::combat::FirearmIntelligence;

/// Updates SpottingTargetingSettings based on FirearmIntelligence.
///
/// Runs before the generic sync_spotted_to_combat_targeting (in combat-targeting)
/// to convert firearm-specific settings into the generic settings component.
/// Adds SpottingTargetingSettings if missing on entities with SpottingUser.
pub(crate) fn update_firearm_spotting_settings(
	mut commands: Commands,
	mut combatants_with_settings: Query<(&FirearmIntelligence, &mut SpottingTargetingSettings)>,
	combatants_without_settings: Query<
		(Entity, &FirearmIntelligence),
		(With<SpottingUser>, Without<SpottingTargetingSettings>),
	>,
) {
	// Add SpottingTargetingSettings to entities that have SpottingUser but lack the settings
	for (entity, firearm) in &combatants_without_settings {
		let memory_secs = firearm.settings.target_spotting_memory.max(0.0);
		let continuity = firearm.settings.focus.clamp(0.0, 1.0) * 8.0;
		commands.entity(entity).insert(SpottingTargetingSettings {
			memory_secs,
			continuity,
		});
	}

	// Update existing SpottingTargetingSettings
	for (firearm, mut settings) in &mut combatants_with_settings {
		let memory_secs = firearm.settings.target_spotting_memory.max(0.0);
		let continuity = firearm.settings.focus.clamp(0.0, 1.0) * 8.0;
		if settings.memory_secs != memory_secs || settings.continuity != continuity {
			settings.memory_secs = memory_secs;
			settings.continuity = continuity;
		}
	}
}
