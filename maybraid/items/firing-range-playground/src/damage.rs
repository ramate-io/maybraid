use bevy::prelude::*;
use firearm_intelligence::{FirearmIntelligence, FirearmMovementIntelligence, FirearmSpotting};
use firearm_user::FirearmUser;
use movement_intelligence::{MovementIntelligence, ReplanMovement};
use player::{LocomotionCapsule, MoveWish, Npc, Player};

pub(crate) const RESPAWN_SECS: f32 = 2.0;
pub(crate) const HEADSHOT_MULTIPLIER: f32 = 1.25;

pub(crate) use ::damage::{DamageApplied, HeadshotBand, Health, DEFAULT_MAX_HEALTH as MAX_HEALTH};

/// Top half of the upper capsule hemisphere.
pub(crate) fn headshot_band() -> HeadshotBand {
	headshot_band_for(LocomotionCapsule::HUMANOID)
}

pub(crate) fn headshot_band_for(hull: LocomotionCapsule) -> HeadshotBand {
	HeadshotBand { min_local_y: hull.headshot_min_local_y(), multiplier: HEADSHOT_MULTIPLIER }
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

type DownedCombatants<'w, 's> = Query<
	'w,
	's,
	(Entity, Option<&'static FirearmUser>, Has<Player>, Has<Npc>),
	Added<::damage::Downed>,
>;

pub(crate) fn queue_downed_respawns(
	time: Res<Time>,
	mut respawn: ResMut<CombatRespawn>,
	mut engagement: ResMut<crate::engagement::NpcEngagement>,
	mut commands: Commands,
	combatants: DownedCombatants,
) {
	let now = time.elapsed_secs();
	for (entity, user, is_player, is_npc) in &combatants {
		commands.entity(entity).remove::<(
			FirearmIntelligence,
			FirearmMovementIntelligence,
			FirearmSpotting,
			MovementIntelligence,
			ReplanMovement,
			MoveWish,
		)>();
		if is_player {
			respawn.player_at = Some(now + RESPAWN_SECS);
			engagement.reset();
		}
		if is_npc {
			respawn.npc_at.push(now + RESPAWN_SECS);
		}
		if let Some(user) = user {
			commands.entity(user.held).try_insert(::damage::DespawnAfter::seconds(0.0));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn headshot_band_is_the_upper_half_of_the_top_hemisphere() {
		let band = headshot_band();
		let hull = LocomotionCapsule::HUMANOID;
		assert!((band.min_local_y - hull.headshot_min_local_y()).abs() < 1e-5);
		assert!((band.multiplier - HEADSHOT_MULTIPLIER).abs() < 1e-5);
	}

	#[test]
	fn downing_retires_playground_intelligence_immediately() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<CombatRespawn>()
			.init_resource::<crate::engagement::NpcEngagement>()
			.add_systems(Update, queue_downed_respawns);
		let entity = app
			.world_mut()
			.spawn((
				Npc,
				::damage::Downed { source: None, point: Vec3::ZERO, at: 0.0 },
				FirearmSpotting::default(),
				MovementIntelligence::new(movement_intelligence::MovementObjective::Reach(
					movement_intelligence::MovementLocation::new(Vec3::ZERO, 0.4),
				)),
				MoveWish::default(),
			))
			.id();

		app.update();

		assert!(app.world().get::<FirearmSpotting>(entity).is_none());
		assert!(app.world().get::<MovementIntelligence>(entity).is_none());
		assert!(app.world().get::<MoveWish>(entity).is_none());
		assert_eq!(app.world().resource::<CombatRespawn>().npc_at.len(), 1);
	}
}
