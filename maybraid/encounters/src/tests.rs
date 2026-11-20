//! Progress-owner regressions: identity, teardown, kill/free, lifetime policy.

use anyhow::{anyhow, Result};
use bevy::prelude::*;
use bevy::time::{TimeUpdateStrategy, Virtual};
use damage::{Died, Health};
use std::time::Duration;

use crate::fixtures::{install, spawn, spawn_instance};
use crate::progress::{EncounterLifetime, EncounterOutcome, EncounterPhase};
use crate::{
	EncounterFinishedReentry, EncounterId, EncounterInstance, EncounterLifetimePolicy,
	EncounterProgressStore, EncounterResolved, EncounterSignal, EncounterSignalKind,
	EncounterSleepPolicy, EncounterTimeEnabled,
};
use attacks::AttackSuspended;

const DT: f32 = 0.1;
const ID: fn() -> EncounterId = || EncounterId::from_slug("fixture.encounter.v0");

fn app() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)));
	install(&mut app);
	app.world_mut()
		.resource_mut::<Time<Virtual>>()
		.set_max_delta(Duration::from_secs(30));
	app.update();
	app
}

fn bind(app: &mut App, health: f32) -> Entity {
	let entity = spawn_instance(app.world_mut(), ID(), health);
	app.update();
	entity
}

fn resolved(app: &mut App) -> Vec<EncounterResolved> {
	app.world_mut().resource_mut::<Messages<EncounterResolved>>().drain().collect()
}

fn record(app: &App) -> Result<&crate::EncounterRecord> {
	app.world()
		.resource::<EncounterProgressStore>()
		.get(ID())
		.ok_or_else(|| anyhow!("missing record"))
}

#[test]
fn plugin_registers_store_without_hud() {
	let app = app();
	assert!(app.world().get_resource::<EncounterProgressStore>().is_some());
	assert!(app.world().get_resource::<Messages<EncounterSignal>>().is_some());
	assert!(app.world().get_resource::<Messages<EncounterResolved>>().is_some());
}

#[test]
fn fixture_binds_live_combat_progress() -> Result<()> {
	let mut app = app();
	let entity = bind(&mut app, 100.0);
	let row = record(&app)?;
	assert_eq!(row.instance, Some(entity));
	assert_eq!(row.lifetime, EncounterLifetime::InProgress);
	assert_eq!(row.phase, EncounterPhase::Combat);
	assert!(!row.completed_once);
	Ok(())
}

#[test]
fn presentation_teardown_is_not_an_outcome() -> Result<()> {
	let mut app = app();
	let entity = bind(&mut app, 100.0);
	app.world_mut().entity_mut(entity).despawn();
	app.update();
	assert!(resolved(&mut app).is_empty());
	let row = record(&app)?;
	assert_eq!(row.lifetime, EncounterLifetime::Vacant);
	assert!(row.outcome.is_none());
	assert!(row.instance.is_none());
	Ok(())
}

#[test]
fn kill_finishes_once_and_ignores_duplicate_died() -> Result<()> {
	let mut app = app();
	let boss = bind(&mut app, 100.0);
	let killer = Entity::from_bits(9);
	app.world_mut()
		.write_message(Died { entity: boss, source: Some(killer), point: Vec3::ZERO });
	app.world_mut()
		.write_message(Died { entity: boss, source: Some(killer), point: Vec3::ZERO });
	app.update();
	let events = resolved(&mut app);
	assert_eq!(events.len(), 1);
	assert_eq!(events[0].outcome, EncounterOutcome::Killed);
	assert_eq!(events[0].source, Some(killer));
	let row = record(&app)?;
	assert_eq!(row.outcome, Some(EncounterOutcome::Killed));
	assert!(row.completed_once);
	Ok(())
}

#[test]
fn same_frame_kill_beats_chain_break() -> Result<()> {
	let mut app = app();
	let boss = bind(&mut app, 100.0);
	app.world_mut().write_message(Died {
		entity: boss,
		source: Some(Entity::from_bits(3)),
		point: Vec3::ZERO,
	});
	app.world_mut()
		.write_message(EncounterSignal::chain_broken(ID(), Some(Entity::from_bits(3))));
	app.update();
	let events = resolved(&mut app);
	assert_eq!(events.len(), 1);
	assert_eq!(events[0].outcome, EncounterOutcome::Killed);
	assert_eq!(record(&app)?.outcome, Some(EncounterOutcome::Killed));
	Ok(())
}

#[test]
fn chain_break_frees_when_she_is_alive() -> Result<()> {
	let mut app = app();
	bind(&mut app, 100.0);
	app.world_mut()
		.write_message(EncounterSignal::chain_broken(ID(), Some(Entity::from_bits(4))));
	app.update();
	let events = resolved(&mut app);
	assert_eq!(events.len(), 1);
	assert_eq!(events[0].outcome, EncounterOutcome::Freed);
	Ok(())
}

#[test]
fn stream_out_then_reentry_loads_fresh_while_in_progress() -> Result<()> {
	let mut app = app();
	let first = bind(&mut app, 100.0);
	app.world_mut()
		.entity_mut(first)
		.get_mut::<Health>()
		.ok_or_else(|| anyhow!("health"))?
		.current = 40.0;
	app.world_mut().entity_mut(first).despawn();
	app.update();
	assert_eq!(record(&app)?.lifetime, EncounterLifetime::Vacant);
	let second = bind(&mut app, 100.0);
	let row = record(&app)?;
	assert_eq!(row.instance, Some(second));
	assert_eq!(row.lifetime, EncounterLifetime::InProgress);
	assert_eq!(row.phase, EncounterPhase::Combat);
	let health = app.world().get::<Health>(second).ok_or_else(|| anyhow!("second health"))?;
	assert_eq!(health.current, health.max);
	Ok(())
}

#[test]
fn stay_finished_does_not_recreate_on_reentry() -> Result<()> {
	let mut app = app();
	app.world_mut().insert_resource(EncounterLifetimePolicy::default());
	let first = bind(&mut app, 100.0);
	app.world_mut()
		.write_message(Died { entity: first, source: None, point: Vec3::ZERO });
	app.update();
	app.world_mut().entity_mut(first).despawn();
	app.update();
	let attempt = spawn(app.world_mut(), ID());
	app.update();
	assert!(app.world().get::<EncounterInstance>(attempt).is_none());
	let row = record(&app)?;
	assert!(row.completed_once);
	assert_eq!(row.outcome, Some(EncounterOutcome::Killed));
	Ok(())
}

#[test]
fn rare_regen_probability_one_allows_a_new_occurrence() -> Result<()> {
	let mut app = app();
	app.world_mut().insert_resource(EncounterLifetimePolicy {
		finished_reentry: EncounterFinishedReentry::RareTimeSlot {
			probability: 1.0,
			slot_secs: 60.0,
		},
		..EncounterLifetimePolicy::default()
	});
	let first = bind(&mut app, 100.0);
	app.world_mut()
		.write_message(Died { entity: first, source: None, point: Vec3::ZERO });
	app.update();
	app.world_mut().entity_mut(first).despawn();
	app.update();
	let regen = bind(&mut app, 100.0);
	let row = record(&app)?;
	assert_eq!(row.instance, Some(regen));
	assert_eq!(row.lifetime, EncounterLifetime::InProgress);
	assert!(row.completed_once);
	assert!(row.outcome.is_none());
	Ok(())
}

#[test]
fn player_death_loads_fresh_live_fight_and_keeps_finished() -> Result<()> {
	let mut app = app();
	let live = bind(&mut app, 80.0);
	app.world_mut()
		.entity_mut(live)
		.get_mut::<Health>()
		.ok_or_else(|| anyhow!("hp"))?
		.current = 20.0;
	app.world_mut().write_message(EncounterSignal::new(
		ID(),
		EncounterSignalKind::PlayerDied,
		None,
	));
	app.update();
	let health = app.world().get::<Health>(live).ok_or_else(|| anyhow!("hp after"))?;
	assert_eq!(health.current, health.max);
	assert_eq!(record(&app)?.lifetime, EncounterLifetime::InProgress);
	assert_eq!(record(&app)?.phase, EncounterPhase::Combat);

	app.world_mut()
		.write_message(Died { entity: live, source: None, point: Vec3::ZERO });
	app.update();
	assert_eq!(record(&app)?.outcome, Some(EncounterOutcome::Killed));
	app.world_mut().write_message(EncounterSignal::new(
		ID(),
		EncounterSignalKind::BodyReplaced,
		None,
	));
	app.update();
	assert_eq!(record(&app)?.outcome, Some(EncounterOutcome::Killed));
	assert!(record(&app)?.completed_once);
	Ok(())
}

#[test]
fn snapshot_restore_preserves_finished_without_live_ids() -> Result<()> {
	let mut app = app();
	let boss = bind(&mut app, 100.0);
	app.world_mut()
		.write_message(Died { entity: boss, source: None, point: Vec3::ZERO });
	app.update();
	let snap = app.world().resource::<EncounterProgressStore>().snapshot();
	app.world_mut().resource_mut::<EncounterProgressStore>().restore(&snap);
	let row = record(&app)?;
	assert_eq!(row.outcome, Some(EncounterOutcome::Killed));
	assert!(row.instance.is_none());
	assert!(row.completed_once);
	Ok(())
}

#[test]
fn session_reset_does_not_clear_finished_under_default_policy() -> Result<()> {
	let mut app = app();
	let boss = bind(&mut app, 100.0);
	app.world_mut()
		.write_message(Died { entity: boss, source: None, point: Vec3::ZERO });
	app.update();
	app.world_mut().write_message(EncounterSignal::session_reset());
	app.update();
	assert_eq!(record(&app)?.outcome, Some(EncounterOutcome::Killed));
	Ok(())
}

fn sleep_app() -> App {
	let mut app = app();
	app.world_mut().insert_resource(EncounterSleepPolicy::bikanwarda());
	app
}

fn set_current(app: &mut App, entity: Entity, current: f32) -> Result<()> {
	app.world_mut()
		.entity_mut(entity)
		.get_mut::<Health>()
		.ok_or_else(|| anyhow!("health"))?
		.current = current;
	Ok(())
}

fn is_sleeping(app: &App) -> Result<bool> {
	Ok(matches!(record(app)?.phase, EncounterPhase::Sleep(_)))
}

#[test]
fn spawn_already_below_twenty_does_not_enter_sleep() -> Result<()> {
	let mut app = sleep_app();
	let boss = spawn_instance(app.world_mut(), ID(), 100.0);
	set_current(&mut app, boss, 15.0)?;
	app.update();
	assert!(!is_sleeping(&app)?);
	assert_eq!(record(&app)?.phase, EncounterPhase::Combat);
	assert!(!record(&app)?.sleep_used);
	Ok(())
}

#[test]
fn crossing_twenty_enters_sleep_once_and_suspends_attacks() -> Result<()> {
	let mut app = sleep_app();
	let boss = bind(&mut app, 100.0);
	set_current(&mut app, boss, 20.0)?;
	app.update();
	assert!(is_sleeping(&app)?);
	assert!(record(&app)?.sleep_used);
	assert!(app.world().get::<AttackSuspended>(boss).is_some());
	set_current(&mut app, boss, 50.0)?;
	app.update();
	assert!(is_sleeping(&app)?);
	Ok(())
}

#[test]
fn just_above_twenty_does_not_enter() -> Result<()> {
	let mut app = sleep_app();
	let boss = bind(&mut app, 100.0);
	set_current(&mut app, boss, 20.1)?;
	app.update();
	assert!(!is_sleeping(&app)?);
	Ok(())
}

#[test]
fn sleep_completion_restores_to_fifty_and_returns_control() -> Result<()> {
	let mut app = sleep_app();
	app.world_mut().insert_resource(EncounterSleepPolicy {
		duration_secs: 0.25,
		..EncounterSleepPolicy::bikanwarda()
	});
	let boss = bind(&mut app, 100.0);
	set_current(&mut app, boss, 18.0)?;
	app.update();
	assert!(is_sleeping(&app)?);
	for _ in 0..8 {
		app.update();
	}
	let health = app.world().get::<Health>(boss).ok_or_else(|| anyhow!("hp"))?;
	assert!((health.current - 50.0).abs() < 1e-4);
	assert_eq!(record(&app)?.phase, EncounterPhase::Combat);
	assert!(app.world().get::<AttackSuspended>(boss).is_none());
	Ok(())
}

#[test]
fn ten_percent_wakes_without_remaining_restore() -> Result<()> {
	let mut app = sleep_app();
	let boss = bind(&mut app, 100.0);
	set_current(&mut app, boss, 18.0)?;
	app.update();
	assert!(is_sleeping(&app)?);
	set_current(&mut app, boss, 10.0)?;
	app.update();
	assert!(!is_sleeping(&app)?);
	let health = app.world().get::<Health>(boss).ok_or_else(|| anyhow!("hp"))?;
	assert!((health.current - 10.0).abs() < 1e-4);
	assert!(app.world().get::<AttackSuspended>(boss).is_none());
	Ok(())
}

#[test]
fn hit_crossing_both_thresholds_wakes_the_same_frame() -> Result<()> {
	let mut app = sleep_app();
	let boss = bind(&mut app, 100.0);
	set_current(&mut app, boss, 8.0)?;
	app.update();
	assert!(!is_sleeping(&app)?);
	assert!(record(&app)?.sleep_used);
	assert!(matches!(record(&app)?.phase, EncounterPhase::Waking | EncounterPhase::Combat));
	let health = app.world().get::<Health>(boss).ok_or_else(|| anyhow!("hp"))?;
	assert!((health.current - 8.0).abs() < 1e-4);
	Ok(())
}

#[test]
fn death_during_sleep_does_not_resurrect() -> Result<()> {
	let mut app = sleep_app();
	let boss = bind(&mut app, 100.0);
	set_current(&mut app, boss, 18.0)?;
	app.update();
	assert!(is_sleeping(&app)?);
	app.world_mut()
		.write_message(Died { entity: boss, source: None, point: Vec3::ZERO });
	set_current(&mut app, boss, 0.0)?;
	app.update();
	assert_eq!(record(&app)?.outcome, Some(EncounterOutcome::Killed));
	let health = app.world().get::<Health>(boss).ok_or_else(|| anyhow!("hp"))?;
	assert_eq!(health.current, 0.0);
	assert!(app.world().get::<AttackSuspended>(boss).is_none());
	Ok(())
}

#[test]
fn pause_freezes_sleep_elapsed() -> Result<()> {
	let mut app = sleep_app();
	let boss = bind(&mut app, 100.0);
	set_current(&mut app, boss, 18.0)?;
	app.update();
	let before = match record(&app)?.phase {
		EncounterPhase::Sleep(sleep) => sleep.elapsed_secs,
		_ => return Err(anyhow!("expected sleep")),
	};
	app.world_mut().insert_resource(EncounterTimeEnabled(false));
	for _ in 0..10 {
		app.update();
	}
	let after = match record(&app)?.phase {
		EncounterPhase::Sleep(sleep) => sleep.elapsed_secs,
		_ => return Err(anyhow!("still sleep")),
	};
	assert_eq!(before, after);
	assert!(app.world().get::<AttackSuspended>(boss).is_some());
	Ok(())
}

#[test]
fn once_policy_does_not_sleep_again_after_restore() -> Result<()> {
	let mut app = sleep_app();
	app.world_mut().insert_resource(EncounterSleepPolicy {
		duration_secs: 0.15,
		..EncounterSleepPolicy::bikanwarda()
	});
	let boss = bind(&mut app, 100.0);
	set_current(&mut app, boss, 18.0)?;
	app.update();
	for _ in 0..8 {
		app.update();
	}
	assert_eq!(record(&app)?.phase, EncounterPhase::Combat);
	set_current(&mut app, boss, 15.0)?;
	app.update();
	assert!(!is_sleeping(&app)?);
	assert!(record(&app)?.sleep_used);
	Ok(())
}
