//! Playground ceasefire: NPCs return fire after the player shoots.

use bevy::prelude::*;
use firearm_user::FirearmUser;
use firearms::{WeaponFired, WeaponTrigger};
use player::{Npc, Player};

#[derive(Resource, Default)]
pub(crate) struct NpcEngagement {
	live: bool,
}

impl NpcEngagement {
	pub fn is_live(&self) -> bool {
		self.live
	}

	pub fn reset(&mut self) {
		self.live = false;
	}
}

/// Observe actual shots, rather than raw trigger input.
pub(crate) fn record_player_shot(
	players: Query<Entity, With<Player>>,
	mut fired: MessageReader<WeaponFired>,
	mut engagement: ResMut<NpcEngagement>,
) {
	if fired.read().any(|event| players.contains(event.shooter)) {
		engagement.live = true;
	}
}

/// Intelligence may continue spotting and aiming during the ceasefire; only
/// prevent held weapons from consuming the trigger.
pub(crate) fn gate_npc_fire(
	engagement: Res<NpcEngagement>,
	npcs: Query<&FirearmUser, With<Npc>>,
	mut triggers: Query<&mut WeaponTrigger>,
) {
	if engagement.is_live() {
		return;
	}
	for user in &npcs {
		if let Ok(mut trigger) = triggers.get_mut(user.held) {
			trigger.0 = false;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ceasefire_stays_cold_until_a_player_shot() {
		let mut engagement = NpcEngagement::default();
		assert!(!engagement.is_live());
		engagement.live = true;
		assert!(engagement.is_live());
		engagement.reset();
		assert!(!engagement.is_live());
	}
}
