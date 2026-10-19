//! Outcome and unlock regressions. No combat-hud.

use anyhow::{anyhow, Result};
use bevy::prelude::*;
use bevy::time::{TimeUpdateStrategy, Virtual};
use damage::Died;
use encounters::fixtures::{install as install_encounters, spawn_instance};
use encounters::{
	EncounterFinishedReentry, EncounterId, EncounterLifetimePolicy, EncounterOutcome,
	EncounterSignal,
};
use lubikans::{LubikanAccountId, LubikanCatalog, LubikanLedger, LubikanOwner, LubikansPlugin};
use std::time::Duration;

use crate::identity::{chained_encounter_id, ENRAGED_DEFINITION_ID, WARD_DEFINITION_ID};
use crate::outcomes::{BikanwardaDeparted, BikanwardaDeparture, EncounterUnlockAwarded};
use crate::plugin::BikanwardaEncounterPlugin;

const DT: f32 = 0.1;

fn gameplay() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)));
	install_encounters(&mut app);
	if !app.is_plugin_added::<LubikansPlugin>() {
		app.add_plugins(LubikansPlugin);
	}
	app.add_plugins(BikanwardaEncounterPlugin);
	app.world_mut()
		.resource_mut::<Time<Virtual>>()
		.set_max_delta(Duration::from_secs(30));
	app.update();
	app
}

fn spawn_owner(app: &mut App) -> Entity {
	let owner = app.world_mut().spawn(LubikanOwner).id();
	app.update();
	owner
}

fn spawn_boss(app: &mut App) -> Entity {
	let boss = spawn_instance(app.world_mut(), chained_encounter_id(), 100.0);
	app.update();
	boss
}

fn drain_awards(app: &mut App) -> Vec<EncounterUnlockAwarded> {
	app.world_mut()
		.resource_mut::<Messages<EncounterUnlockAwarded>>()
		.drain()
		.collect()
}

fn drain_departures(app: &mut App) -> Vec<BikanwardaDeparted> {
	app.world_mut().resource_mut::<Messages<BikanwardaDeparted>>().drain().collect()
}

fn owned(app: &App, owner: Entity, definition: lubikans::LubikanDefinitionId) -> Result<bool> {
	let ledger = app.world().get::<LubikanLedger>(owner).ok_or_else(|| anyhow!("ledger"))?;
	Ok(ledger.contains(LubikanAccountId::unique(definition)))
}

fn account_count(app: &App, owner: Entity) -> Result<usize> {
	let ledger = app.world().get::<LubikanLedger>(owner).ok_or_else(|| anyhow!("ledger"))?;
	Ok(ledger.iter().count())
}

#[test]
fn catalog_registers_ward_and_enraged_without_hud() -> Result<()> {
	let app = gameplay();
	let catalog = app.world().resource::<LubikanCatalog>();
	anyhow::ensure!(catalog.get(ENRAGED_DEFINITION_ID).is_some(), "enraged");
	anyhow::ensure!(catalog.get(WARD_DEFINITION_ID).is_some(), "ward");
	anyhow::ensure!(catalog.get(lubikans::LubikanDefinitionId::SELF).is_some(), "self");
	Ok(())
}

#[test]
fn kill_awards_enraged_once() -> Result<()> {
	let mut app = gameplay();
	let owner = spawn_owner(&mut app);
	let boss = spawn_boss(&mut app);
	app.world_mut()
		.write_message(Died { entity: boss, source: Some(owner), point: Vec3::ZERO });
	app.update();
	let awards = drain_awards(&mut app);
	assert_eq!(awards.len(), 1);
	assert!(awards[0].first_grant);
	assert_eq!(awards[0].outcome, EncounterOutcome::Killed);
	assert!(owned(&app, owner, ENRAGED_DEFINITION_ID)?);
	assert!(!owned(&app, owner, WARD_DEFINITION_ID)?);
	let departures = drain_departures(&mut app);
	assert_eq!(departures.len(), 1);
	assert_eq!(departures[0].kind, BikanwardaDeparture::Killed);

	app.world_mut()
		.write_message(Died { entity: boss, source: Some(owner), point: Vec3::ZERO });
	app.update();
	assert!(drain_awards(&mut app).is_empty());
	assert_eq!(account_count(&app, owner)?, 2);
	Ok(())
}

#[test]
fn chain_break_awards_ward_and_transcends() -> Result<()> {
	let mut app = gameplay();
	let owner = spawn_owner(&mut app);
	spawn_boss(&mut app);
	app.world_mut()
		.write_message(EncounterSignal::chain_broken(chained_encounter_id(), Some(owner)));
	app.update();
	let awards = drain_awards(&mut app);
	assert_eq!(awards.len(), 1);
	assert_eq!(awards[0].outcome, EncounterOutcome::Freed);
	assert!(owned(&app, owner, WARD_DEFINITION_ID)?);
	assert!(!owned(&app, owner, ENRAGED_DEFINITION_ID)?);
	let departures = drain_departures(&mut app);
	assert_eq!(departures[0].kind, BikanwardaDeparture::Transcend);
	Ok(())
}

#[test]
fn same_frame_kill_and_break_awards_only_enraged() -> Result<()> {
	let mut app = gameplay();
	let owner = spawn_owner(&mut app);
	let boss = spawn_boss(&mut app);
	app.world_mut()
		.write_message(Died { entity: boss, source: Some(owner), point: Vec3::ZERO });
	app.world_mut()
		.write_message(EncounterSignal::chain_broken(chained_encounter_id(), Some(owner)));
	app.update();
	let awards = drain_awards(&mut app);
	assert_eq!(awards.len(), 1);
	assert_eq!(awards[0].outcome, EncounterOutcome::Killed);
	assert!(owned(&app, owner, ENRAGED_DEFINITION_ID)?);
	assert!(!owned(&app, owner, WARD_DEFINITION_ID)?);
	Ok(())
}

#[test]
fn inventory_can_hold_both_across_occurrences() -> Result<()> {
	let mut app = gameplay();
	app.world_mut().insert_resource(EncounterLifetimePolicy {
		finished_reentry: EncounterFinishedReentry::RareTimeSlot {
			probability: 1.0,
			slot_secs: 60.0,
		},
		..EncounterLifetimePolicy::bikanwarda()
	});
	let owner = spawn_owner(&mut app);
	let first = spawn_boss(&mut app);
	app.world_mut()
		.write_message(Died { entity: first, source: Some(owner), point: Vec3::ZERO });
	app.update();
	drain_awards(&mut app);
	app.world_mut().entity_mut(first).despawn();
	app.update();
	spawn_boss(&mut app);
	app.world_mut()
		.write_message(EncounterSignal::chain_broken(chained_encounter_id(), Some(owner)));
	app.update();
	assert!(owned(&app, owner, ENRAGED_DEFINITION_ID)?);
	assert!(owned(&app, owner, WARD_DEFINITION_ID)?);
	assert_eq!(account_count(&app, owner)?, 3);
	Ok(())
}

#[test]
fn missing_source_finishes_without_inventing_an_owner() -> Result<()> {
	let mut app = gameplay();
	let owner = spawn_owner(&mut app);
	let boss = spawn_boss(&mut app);
	app.world_mut()
		.write_message(Died { entity: boss, source: None, point: Vec3::ZERO });
	app.update();
	assert!(drain_awards(&mut app).is_empty());
	assert!(!owned(&app, owner, ENRAGED_DEFINITION_ID)?);
	Ok(())
}

#[test]
fn chained_slug_is_the_fixture_key() {
	assert_eq!(chained_encounter_id(), EncounterId::from_slug("bikanwarda.chained.v0"));
}
