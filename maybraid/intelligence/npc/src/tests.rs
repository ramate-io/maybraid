use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use combat_targeting::CombatTargeting;
use firearm_intelligence::FirearmIntelligence;
use meandering_intelligence::MeanderingIntelligenceUser;
use poi_intelligence::{PoiGoal, PoiId, PoiKind};
use tether_intelligence::{TetherIntelligenceUser, TetherObjective};
use threat_management_intelligence::{ThreatManagementIntelligence, ThreatTactic};

use crate::{mix_npc_brains, NpcInstall, NpcIntelligence, NpcIntelligencePlugin, Personality};

fn dummy_goal() -> PoiGoal {
	PoiGoal::new(1, PoiId(1), None, PoiKind::new("test/place"), Vec3::X, 1.0, 0.0, 0.0)
}

#[test]
fn grazer_is_unarmed_and_evade_capable() {
	let mut world = World::new();
	let npc = world.spawn_empty().id();
	Personality::Grazer.install(&mut world.commands(), npc, NpcInstall::default());
	world.flush();
	assert!(world.get::<FirearmIntelligence>(npc).is_none());
	assert!(world.get::<CombatTargeting>(npc).is_none());
	assert!(world.get::<NpcIntelligence>(npc).is_some());
	assert!(world
		.get::<MeanderingIntelligenceUser>(npc)
		.is_some_and(|user| { (user.linger_secs - 6.0).abs() < 1e-4 }));
	assert!(world.get::<ThreatManagementIntelligence>(npc).is_some_and(|threat| threat
		.combat
		.by_health
		== 0.0 && threat
		.evade
		.by_distance
		> 0.0));
}

#[test]
fn unarmed_civilian_skips_combat_brains() {
	let mut world = World::new();
	let npc = world.spawn_empty().id();
	Personality::Civilian.install(
		&mut world.commands(),
		npc,
		NpcInstall { armed: false, ..NpcInstall::default() },
	);
	world.flush();
	assert!(world.get::<CombatTargeting>(npc).is_none());
	assert!(world.get::<FirearmIntelligence>(npc).is_none());
	assert!(world.get::<evasion_intelligence::EvasionIntelligenceUser>(npc).is_some());
}

#[test]
fn predator_stores_an_inner_combat_tether() {
	let mut world = World::new();
	let anchor = world.spawn_empty().id();
	let npc = world.spawn_empty().id();
	Personality::Predator.install(
		&mut world.commands(),
		npc,
		NpcInstall { tether: Some(anchor), ..NpcInstall::default() },
	);
	world.flush();
	assert!(world.get::<FirearmIntelligence>(npc).is_some());
	assert!(world.get::<CombatTargeting>(npc).is_some());
	assert!(world
		.get::<NpcIntelligence>(npc)
		.is_some_and(|npc| npc.engaged_tether.is_some() && !npc.keep_tether_in_combat));
	assert!(matches!(
		world.get::<TetherIntelligenceUser>(npc).map(|user| user.objective),
		Some(TetherObjective::Stalk(_, _))
	));
}

#[test]
fn install_can_keep_tether_in_combat() {
	let mut world = World::new();
	let anchor = world.spawn_empty().id();
	let npc = world.spawn_empty().id();
	Personality::Predator.install(
		&mut world.commands(),
		npc,
		NpcInstall {
			tether: Some(anchor),
			keep_tether_in_combat: Some(true),
			..NpcInstall::default()
		},
	);
	world.flush();
	assert!(world.get::<NpcIntelligence>(npc).is_some_and(|npc| npc.keep_tether_in_combat));
}

#[test]
fn brawler_installs_combat_and_flee_without_hiding() {
	let mut world = World::new();
	let npc = world.spawn_empty().id();
	Personality::Brawler.install(&mut world.commands(), npc, NpcInstall::default());
	world.flush();
	assert!(world.get::<CombatTargeting>(npc).is_some());
	assert!(world.get::<FirearmIntelligence>(npc).is_some());
	assert!(world.get::<fleeing_intelligence::FleeingUser>(npc).is_some());
	assert!(world.get::<hiding_intelligence::HidingUser>(npc).is_none());
}

#[test]
fn assassin_prefers_a_stalking_idle_tether() {
	let mut world = World::new();
	let anchor = world.spawn_empty().id();
	let npc = world.spawn_empty().id();
	Personality::Assassin.install(
		&mut world.commands(),
		npc,
		NpcInstall { tether: Some(anchor), ..NpcInstall::default() },
	);
	world.flush();
	let objective = world.get::<TetherIntelligenceUser>(npc).map(|user| user.objective);
	assert!(matches!(objective, Some(TetherObjective::Stalk(_, _))));
	assert!(world
		.get::<NpcIntelligence>(npc)
		.is_some_and(|npc| npc.engaged_tether.is_some()));
	assert!(world.get::<hiding_intelligence::HidingUser>(npc).is_some());
}

#[test]
fn combat_retracts_meander_and_drops_the_poi_goal() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, NpcIntelligencePlugin));
	let npc = app
		.world_mut()
		.spawn((
			NpcIntelligence::default(),
			{
				let mut threat = ThreatManagementIntelligence::default();
				threat.tactic = ThreatTactic::Combat;
				threat
			},
			MeanderingIntelligenceUser::default(),
			dummy_goal(),
		))
		.id();
	app.update();
	assert!(app
		.world()
		.get::<MeanderingIntelligenceUser>(npc)
		.is_some_and(|user| !user.enabled));
	assert!(app.world().get::<PoiGoal>(npc).is_none());
}

#[test]
fn ignore_restores_meander_and_idle_tether() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, NpcIntelligencePlugin));
	let anchor = app.world_mut().spawn_empty().id();
	let idle = TetherObjective::Tether(anchor, 12.0);
	let engaged = TetherObjective::Tether(anchor, 4.0);
	let mut tether = TetherIntelligenceUser::new(engaged).with_enabled(false);
	tether.objective = engaged;
	let mut meandering = MeanderingIntelligenceUser::default();
	meandering.enabled = false;
	let npc = app
		.world_mut()
		.spawn((
			NpcIntelligence { idle_tether: Some(idle), engaged_tether: Some(engaged), ..default() },
			ThreatManagementIntelligence::default(),
			meandering,
			tether,
		))
		.id();
	app.update();
	assert!(app
		.world()
		.get::<MeanderingIntelligenceUser>(npc)
		.is_some_and(|user| user.enabled));
	assert!(app
		.world()
		.get::<TetherIntelligenceUser>(npc)
		.is_some_and(|user| { user.enabled && user.objective == idle }));
}

#[test]
fn combat_disables_tether_unless_kept() {
	let mut world = World::new();
	let anchor = world.spawn_empty().id();
	let idle = TetherObjective::Tether(anchor, 12.0);
	let engaged = TetherObjective::Tether(anchor, 4.0);
	let mut threat = ThreatManagementIntelligence::default();
	threat.tactic = ThreatTactic::Combat;
	let npc = world
		.spawn((
			NpcIntelligence {
				idle_tether: Some(idle),
				engaged_tether: Some(engaged),
				keep_tether_in_combat: false,
			},
			threat,
			TetherIntelligenceUser::new(idle).with_enabled(true),
		))
		.id();
	world.run_system_once(mix_npc_brains).unwrap();
	assert!(world
		.get::<TetherIntelligenceUser>(npc)
		.is_some_and(|user| { !user.enabled && user.objective == engaged }));
}

#[test]
fn evade_always_disables_tether() {
	let mut world = World::new();
	let anchor = world.spawn_empty().id();
	let idle = TetherObjective::Tether(anchor, 12.0);
	let mut threat = ThreatManagementIntelligence::default();
	threat.tactic = ThreatTactic::Evade;
	let npc = world
		.spawn((
			NpcIntelligence { idle_tether: Some(idle), keep_tether_in_combat: true, ..default() },
			threat,
			TetherIntelligenceUser::new(idle).with_enabled(true),
		))
		.id();
	world.run_system_once(mix_npc_brains).unwrap();
	assert!(world.get::<TetherIntelligenceUser>(npc).is_some_and(|user| !user.enabled));
}
