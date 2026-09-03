//! Plugin: firearm movement writes [`MovementObjective`], combat aims and fires.

use bevy::prelude::*;
use firearm_user::FirearmUserSystems;
use firearms::{FirearmHostSystems, FirearmWeaponSystems};
use movement_intelligence::MovementIntelligenceSystems;
use player::PlayerPoseSystems;

use crate::combat::{
	aim_at_firearm_targets, fire_at_spotted_targets, note_weapon_recoil, orient_firearm_combatants,
};
use crate::movement::write_firearm_movement_objectives;
use crate::spotting::spot_firearm_targets;

/// Perception, movement policy, desired aim, then actual-bore fire control.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FirearmIntelligenceSystems {
	Spotting,
	Movement,
	Aim,
	Orient,
	Fire,
}

pub struct FirearmIntelligencePlugin;

impl Plugin for FirearmIntelligencePlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(
				FirearmIntelligenceSystems::Spotting.before(FirearmIntelligenceSystems::Movement),
				FirearmIntelligenceSystems::Movement.before(MovementIntelligenceSystems::Replan),
				FirearmIntelligenceSystems::Aim
					.after(FirearmIntelligenceSystems::Spotting)
					.before(FirearmUserSystems::Recoil)
					.before(FirearmIntelligenceSystems::Orient),
				FirearmIntelligenceSystems::Orient
					.after(FirearmIntelligenceSystems::Aim)
					.before(PlayerPoseSystems::Item),
			),
		)
		.configure_sets(
			PostUpdate,
			FirearmIntelligenceSystems::Fire
				.after(TransformSystems::Propagate)
				.after(FirearmHostSystems::Pose)
				.before(FirearmWeaponSystems::Fire),
		)
		.add_systems(Update, spot_firearm_targets.in_set(FirearmIntelligenceSystems::Spotting))
		.add_systems(
			Update,
			write_firearm_movement_objectives.in_set(FirearmIntelligenceSystems::Movement),
		)
		.add_systems(Update, aim_at_firearm_targets.in_set(FirearmIntelligenceSystems::Aim))
		.add_systems(Update, orient_firearm_combatants.in_set(FirearmIntelligenceSystems::Orient))
		.add_systems(PostUpdate, fire_at_spotted_targets.in_set(FirearmIntelligenceSystems::Fire))
		.add_systems(PostUpdate, note_weapon_recoil.after(FirearmWeaponSystems::Fire));
	}
}
