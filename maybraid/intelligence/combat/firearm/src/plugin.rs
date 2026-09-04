//! Plugin: firearm movement writes [`MovementObjective`], combat aims and fires.

use bevy::prelude::*;
use combat_targeting::{CombatTargetingPlugin, CombatTargetingSystems};
use damage::{DamageApplied, DamageSystems};
use firearm_user::FirearmUserSystems;
use firearms::{FirearmHostSystems, FirearmWeaponSystems, WeaponFired};
use movement_intelligence::MovementIntelligenceSystems;
use player::PlayerPoseSystems;
use spotting_intelligence::SpottingSystems;
use spotting_intelligence_avian::SpottingAvianPlugin;

use crate::combat::{
	aim_at_firearm_targets, fire_at_spotted_targets, note_weapon_recoil, orient_firearm_combatants,
};
use crate::engagement::authorize_return_fire_from_damage;
use crate::movement::write_firearm_movement_objectives;
use crate::spotting::sync_spotted_combat_targets;
use crate::targeting::validate_firearm_aim_trajectories;

/// Perception, movement policy, desired aim, then actual-bore fire control.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FirearmIntelligenceSystems {
	Spotting,
	Movement,
	Aim,
	Orient,
	ValidateAim,
	Fire,
}

pub struct FirearmIntelligencePlugin;

impl Plugin for FirearmIntelligencePlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SpottingAvianPlugin>() {
			app.add_plugins(SpottingAvianPlugin);
		}
		if !app.is_plugin_added::<CombatTargetingPlugin>() {
			app.add_plugins(CombatTargetingPlugin);
		}
		app.add_message::<DamageApplied>().add_message::<WeaponFired>();
		app.configure_sets(
			Update,
			(
				SpottingSystems::Observe.before(FirearmIntelligenceSystems::Spotting),
				FirearmIntelligenceSystems::Spotting.before(CombatTargetingSystems::Rank),
				FirearmIntelligenceSystems::Movement
					.after(CombatTargetingSystems::Rank)
					.before(MovementIntelligenceSystems::Replan),
				FirearmIntelligenceSystems::Aim
					.after(CombatTargetingSystems::Rank)
					.before(FirearmUserSystems::Recoil)
					.before(FirearmIntelligenceSystems::Orient),
				FirearmIntelligenceSystems::Orient
					.after(FirearmIntelligenceSystems::Aim)
					.before(PlayerPoseSystems::Item),
			),
		)
		.configure_sets(
			PostUpdate,
			(
				FirearmIntelligenceSystems::ValidateAim
					.after(TransformSystems::Propagate)
					.after(FirearmHostSystems::Pose),
				FirearmIntelligenceSystems::Fire
					.after(FirearmIntelligenceSystems::ValidateAim)
					.before(FirearmWeaponSystems::Fire),
			),
		)
		.add_systems(
			Update,
			sync_spotted_combat_targets.in_set(FirearmIntelligenceSystems::Spotting),
		)
		.add_systems(
			Update,
			write_firearm_movement_objectives.in_set(FirearmIntelligenceSystems::Movement),
		)
		.add_systems(Update, aim_at_firearm_targets.in_set(FirearmIntelligenceSystems::Aim))
		.add_systems(Update, orient_firearm_combatants.in_set(FirearmIntelligenceSystems::Orient))
		.add_systems(
			PostUpdate,
			validate_firearm_aim_trajectories.in_set(FirearmIntelligenceSystems::ValidateAim),
		)
		.add_systems(PostUpdate, authorize_return_fire_from_damage.after(DamageSystems::Apply))
		.add_systems(PostUpdate, fire_at_spotted_targets.in_set(FirearmIntelligenceSystems::Fire))
		.add_systems(PostUpdate, note_weapon_recoil.after(FirearmWeaponSystems::Fire));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::message::Messages;

	#[test]
	fn plugin_registers_messages_its_systems_read() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_plugins(FirearmIntelligencePlugin);
		assert!(app.world().get_resource::<Messages<DamageApplied>>().is_some());
		assert!(app.world().get_resource::<Messages<WeaponFired>>().is_some());
	}
}
