//! Perception stub: copy the player into both firearm objective lists.

use bevy::prelude::*;
use firearm_intelligence::{
	FirearmIntelligence, FirearmMovementIntelligence, FirearmMovementObjective, FirearmObjective,
};
use player::{Npc, Player};

pub(crate) fn assign_player_combat_targets(
	players: Query<Entity, With<Player>>,
	mut npcs: Query<
		(&mut FirearmIntelligence, &mut FirearmMovementIntelligence),
		(With<Npc>, Without<Player>),
	>,
) {
	let Ok(player) = players.single() else {
		return;
	};
	for (mut combat, mut movement) in &mut npcs {
		if combat.objective.0.first().map(|target| target.entity) != Some(player) {
			combat.objective = FirearmObjective::from_target(player);
		}
		if movement.objective.0.first().map(|target| target.entity) != Some(player) {
			movement.objective = FirearmMovementObjective::from_target(player);
		}
	}
}
