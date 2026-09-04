use bevy::prelude::*;
use combat_targeting::CombatTargeting;
use evasion_intelligence::EvasionIntelligenceUser;
use firearm_intelligence::{FirearmIntelligence, FirearmMovementIntelligence, FirearmTargeting};
use firearm_user::FirearmUser;
use fleeing_intelligence::FleeingUser;
use hiding_intelligence::{HideClaim, HidingUser};
use meandering_intelligence::MeanderingIntelligenceUser;
use movement_intelligence::{MovementIntelligence, ReplanMovement};
use npc_intelligence::NpcIntelligence;
use player::{LocomotionCapsule, MoveWish, Npc, Player};
use poi_intelligence::{PoiGoal, PoiIntelligenceUser, PoiKnowledge, PoiVisitState};
use spotting_intelligence::{SpotSubject, SpottingUser};

use crate::session::{Civilian, RangeSession, FLEE_OUT_RANGE};

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
	pub civilian_at: Vec<f32>,
}

impl CombatRespawn {
	pub fn clear(&mut self) {
		self.player_at = None;
		self.npc_at.clear();
		self.civilian_at.clear();
	}
}

type DownedCombatants<'w, 's> = Query<
	'w,
	's,
	(Entity, Option<&'static FirearmUser>, Has<Player>, Has<Npc>, Has<crate::session::Civilian>),
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
	for (entity, user, is_player, is_npc, is_civilian) in &combatants {
		commands.entity(entity).remove::<(
			FirearmIntelligence,
			FirearmMovementIntelligence,
			FirearmTargeting,
			CombatTargeting,
			EvasionIntelligenceUser,
			FleeingUser,
			HidingUser,
			HideClaim,
			SpottingUser,
			SpotSubject,
			MovementIntelligence,
			ReplanMovement,
			MoveWish,
		)>();
		commands.entity(entity).remove::<(
			threat_management_intelligence::ThreatManagementIntelligence,
			threat_management_intelligence::CombatSelected,
			threat_management_intelligence::EvadeSelected,
			NpcIntelligence,
		)>();
		commands.entity(entity).remove::<(
			MeanderingIntelligenceUser,
			PoiIntelligenceUser,
			PoiKnowledge,
			PoiVisitState,
			PoiGoal,
			tether_intelligence::TetherIntelligenceUser,
			tether_intelligence::TetherMemory,
		)>();
		if is_player {
			respawn.player_at = Some(now + RESPAWN_SECS);
			engagement.reset();
		}
		if is_civilian {
			respawn.civilian_at.push(now + RESPAWN_SECS);
		} else if is_npc {
			respawn.npc_at.push(now + RESPAWN_SECS);
		}
		if let Some(user) = user {
			commands.entity(user.held).try_insert(::damage::DespawnAfter::seconds(0.0));
		}
	}
}

pub(crate) fn xz_from_origin(at: Vec3) -> f32 {
	Vec2::new(at.x, at.z).length()
}

/// AFFA: despawn civilians who fled past the pad disk and queue a ring respawn.
pub(crate) fn queue_flee_out_respawns(
	session: Res<RangeSession>,
	time: Res<Time>,
	mut respawn: ResMut<CombatRespawn>,
	mut commands: Commands,
	fleers: Query<
		(Entity, &Transform, &EvasionIntelligenceUser, Option<&FirearmUser>),
		(With<Civilian>, Without<::damage::Downed>),
	>,
) {
	if !session.is_assault_free_for_all() {
		return;
	}
	let now = time.elapsed_secs();
	for (entity, transform, evasion, user) in &fleers {
		if !evasion.signal.is_flee() {
			continue;
		}
		if xz_from_origin(transform.translation) <= FLEE_OUT_RANGE {
			continue;
		}
		if let Some(user) = user {
			commands.entity(user.held).try_despawn();
		}
		commands.entity(entity).try_despawn();
		respawn.civilian_at.push(now);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use spotting_intelligence::{InterestLayers, SpotBounds};

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
				SpottingUser::default(),
				SpotSubject::new(InterestLayers::CHARACTER, SpotBounds::capsule(0.4, 0.9)),
				CombatTargeting::default(),
				EvasionIntelligenceUser::default(),
				FirearmTargeting::default(),
				MovementIntelligence::new(movement_intelligence::MovementObjective::Reach(
					movement_intelligence::MovementLocation::new(Vec3::ZERO, 0.4),
				)),
				MoveWish::default(),
			))
			.id();

		app.update();

		assert!(app.world().get::<SpottingUser>(entity).is_none());
		assert!(app.world().get::<SpotSubject>(entity).is_none());
		assert!(app.world().get::<CombatTargeting>(entity).is_none());
		assert!(app.world().get::<EvasionIntelligenceUser>(entity).is_none());
		assert!(app.world().get::<MovementIntelligence>(entity).is_none());
		assert!(app.world().get::<MoveWish>(entity).is_none());
		assert_eq!(app.world().resource::<CombatRespawn>().npc_at.len(), 1);
	}

	#[test]
	fn pad_disk_keeps_the_spawn_ring_and_recycles_beyond_it() {
		assert!(xz_from_origin(Vec3::X * 36.0) < FLEE_OUT_RANGE);
		assert!(xz_from_origin(Vec3::new(48.0, 3.0, 0.0)) > FLEE_OUT_RANGE - 1e-4);
		assert!(xz_from_origin(Vec3::X * 60.0) > FLEE_OUT_RANGE);
	}

	#[test]
	fn flee_out_recycles_civilians_past_the_pad() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<CombatRespawn>()
			.insert_resource(RangeSession {
				mode: crate::session::RangeMode::AssaultFreeForAll,
				npc_count: 4,
				civilian_count: 6,
				seed: None,
				epoch: 1,
			})
			.add_systems(Update, queue_flee_out_respawns);
		let mut evasion = EvasionIntelligenceUser::default();
		evasion.signal = evasion_intelligence::EvasionSignal {
			actuator: evasion_intelligence::EvasionActuator::Flee,
			threat: None,
		};
		let entity = app
			.world_mut()
			.spawn((Npc, Civilian, Transform::from_translation(Vec3::X * 60.0), evasion))
			.id();

		app.update();

		assert!(app.world().get_entity(entity).is_err());
		assert_eq!(app.world().resource::<CombatRespawn>().civilian_at.len(), 1);
	}

	#[test]
	fn hiding_civilians_are_not_recycled_for_distance() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<CombatRespawn>()
			.insert_resource(RangeSession {
				mode: crate::session::RangeMode::AssaultFreeForAll,
				npc_count: 4,
				civilian_count: 6,
				seed: None,
				epoch: 1,
			})
			.add_systems(Update, queue_flee_out_respawns);
		let mut evasion = EvasionIntelligenceUser::default();
		evasion.signal = evasion_intelligence::EvasionSignal {
			actuator: evasion_intelligence::EvasionActuator::Hide,
			threat: None,
		};
		let entity = app
			.world_mut()
			.spawn((Npc, Civilian, Transform::from_translation(Vec3::X * 60.0), evasion))
			.id();

		app.update();

		assert!(app.world().get::<Civilian>(entity).is_some());
		assert!(app.world().resource::<CombatRespawn>().civilian_at.is_empty());
	}
}
