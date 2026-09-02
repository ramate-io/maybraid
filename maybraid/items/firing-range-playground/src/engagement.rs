//! Playground ceasefire: the NPC returns fire after the player shoots.

use bevy::prelude::*;
use firearm_user::FirearmUser;
use firearms::WeaponTrigger;
use player::{Npc, Player};
use projectiles::{Flight, ProjectileSource};

#[derive(Resource, Default)]
pub(crate) struct NpcEngagement {
	provoked_pair: Option<(Entity, Entity)>,
}

impl NpcEngagement {
	pub fn is_provoked(&self, player: Entity, npc: Entity) -> bool {
		self.provoked_pair == Some((player, npc))
	}
}

/// Observe actual spawned projectiles, rather than raw trigger input.
pub(crate) fn record_player_shot(
	players: Query<Entity, With<Player>>,
	npcs: Query<Entity, With<Npc>>,
	shots: Query<&ProjectileSource, (With<Flight>, Added<ProjectileSource>)>,
	mut engagement: ResMut<NpcEngagement>,
) {
	let (Ok(player), Ok(npc)) = (players.single(), npcs.single()) else {
		return;
	};
	if shots.iter().any(|source| source.0 == player) {
		engagement.provoked_pair = Some((player, npc));
	}
}

/// Intelligence may continue spotting and aiming during the ceasefire; only
/// prevent its held weapon from consuming the trigger.
pub(crate) fn gate_npc_fire(
	engagement: Res<NpcEngagement>,
	players: Query<Entity, With<Player>>,
	npcs: Query<(Entity, &FirearmUser), With<Npc>>,
	mut triggers: Query<&mut WeaponTrigger>,
) {
	let (Ok(player), Ok((npc, user))) = (players.single(), npcs.single()) else {
		return;
	};
	if engagement.is_provoked(player, npc) {
		return;
	}
	if let Ok(mut trigger) = triggers.get_mut(user.held) {
		trigger.0 = false;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn either_respawn_resets_the_engagement_pair() {
		let player = Entity::from_bits(1);
		let npc = Entity::from_bits(2);
		let engagement = NpcEngagement { provoked_pair: Some((player, npc)) };
		assert!(engagement.is_provoked(player, npc));
		assert!(!engagement.is_provoked(Entity::from_bits(3), npc));
		assert!(!engagement.is_provoked(player, Entity::from_bits(4)));
	}
}
