use bevy::prelude::*;
use firearm_user::FirearmUser;
use player::{Npc, Player};

pub(crate) const RESPAWN_SECS: f32 = 2.0;

pub(crate) use ::damage::{DamageApplied, Health, DEFAULT_MAX_HEALTH as MAX_HEALTH};

#[derive(Resource, Default)]
pub(crate) struct CombatRespawn {
	pub player_at: Option<f32>,
	pub npc_at: Vec<f32>,
}

impl CombatRespawn {
	pub fn clear(&mut self) {
		self.player_at = None;
		self.npc_at.clear();
	}
}

type DeadCombatants<'w, 's> =
	Query<'w, 's, (Entity, &'static Health, Option<&'static FirearmUser>, Has<Player>, Has<Npc>)>;

pub(crate) fn despawn_dead(
	time: Res<Time>,
	mut respawn: ResMut<CombatRespawn>,
	mut engagement: ResMut<crate::engagement::NpcEngagement>,
	mut commands: Commands,
	combatants: DeadCombatants,
) {
	let now = time.elapsed_secs();
	for (entity, health, user, is_player, is_npc) in &combatants {
		if !health.is_dead() {
			continue;
		}
		if is_player {
			respawn.player_at = Some(now + RESPAWN_SECS);
			engagement.reset();
		}
		if is_npc {
			respawn.npc_at.push(now + RESPAWN_SECS);
		}
		if let Some(user) = user {
			commands.entity(user.held).try_despawn();
		}
		commands.entity(entity).try_despawn();
	}
}
