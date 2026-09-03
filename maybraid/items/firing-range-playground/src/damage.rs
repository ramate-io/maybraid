use bevy::prelude::*;
use firearm_user::FirearmUser;
use player::{CAPSULE_LENGTH, CAPSULE_RADIUS, Npc, Player};

pub(crate) const RESPAWN_SECS: f32 = 2.0;
pub(crate) const HEADSHOT_MULTIPLIER: f32 = 1.25;

pub(crate) use ::damage::{DEFAULT_MAX_HEALTH as MAX_HEALTH, DamageApplied, HeadshotBand, Health};

/// Top half of the upper capsule hemisphere.
pub(crate) fn headshot_band() -> HeadshotBand {
	HeadshotBand {
		min_local_y: CAPSULE_LENGTH * 0.5 + CAPSULE_RADIUS * 0.5,
		multiplier: HEADSHOT_MULTIPLIER,
	}
}

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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn headshot_band_is_the_upper_half_of_the_top_hemisphere() {
		let band = headshot_band();
		assert!((band.min_local_y - (CAPSULE_LENGTH * 0.5 + CAPSULE_RADIUS * 0.5)).abs() < 1e-5);
		assert!((band.multiplier - HEADSHOT_MULTIPLIER).abs() < 1e-5);
	}
}
