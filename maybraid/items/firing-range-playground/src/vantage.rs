//! Perception candidates: route the player into firearm spotting.

use bevy::prelude::*;
use firearm_intelligence::{CombatTarget, FirearmSpotting, TargetCapsule};
use player::{Npc, Player};

pub(crate) fn assign_player_combat_targets(
	players: Query<Entity, With<Player>>,
	mut npcs: Query<&mut FirearmSpotting, (With<Npc>, Without<Player>)>,
) {
	let Ok(player) = players.single() else {
		return;
	};
	let capsule = TargetCapsule::new(
		player::CAPSULE_RADIUS,
		player::CAPSULE_LENGTH * 0.5 + player::CAPSULE_RADIUS,
	);
	for mut spotting in &mut npcs {
		if spotting.candidates.first().map(|target| target.entity) != Some(player) {
			spotting.candidates = vec![CombatTarget::new(player, capsule)];
		}
	}
}
