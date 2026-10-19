//! Headless Bikanwarda outcomes through the world adapter. No combat-hud.

use bevy::prelude::*;
use bevy::time::{TimeUpdateStrategy, Virtual};
use bikanwarda_encounter::{
	chained_encounter_id, BikanwardaEncounterPlugin, ENRAGED_DEFINITION_ID, WARD_DEFINITION_ID,
};
use damage::Died;
use encounters::fixtures::spawn_instance;
use encounters::{EncounterOutcome, EncounterProgressStore};
use lubikans::{LubikanAccountId, LubikanLedger, LubikanOwner, LubikansPlugin};
use maybraid_world::{world_encounter, WorldGameplayEnabled, WorldPlaySessionEnded};
use std::time::Duration;

const DT: f32 = 0.1;

fn app() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)))
		.init_resource::<WorldGameplayEnabled>()
		.init_resource::<Messages<WorldPlaySessionEnded>>()
		.add_plugins(LubikansPlugin)
		.add_plugins(BikanwardaEncounterPlugin);
	world_encounter::configure(&mut app);
	app.world_mut()
		.resource_mut::<Time<Virtual>>()
		.set_max_delta(Duration::from_secs(30));
	app.update();
	app
}

#[test]
fn world_kill_awards_enraged_without_combat_hud() -> anyhow::Result<()> {
	let mut app = app();
	let owner = app.world_mut().spawn(LubikanOwner).id();
	app.update();
	let boss = spawn_instance(app.world_mut(), chained_encounter_id(), 100.0);
	app.update();
	app.world_mut()
		.write_message(Died { entity: boss, source: Some(owner), point: Vec3::ZERO });
	app.update();
	let ledger = app
		.world()
		.get::<LubikanLedger>(owner)
		.ok_or_else(|| anyhow::anyhow!("ledger"))?;
	anyhow::ensure!(
		ledger.contains(LubikanAccountId::unique(ENRAGED_DEFINITION_ID)),
		"enraged unlock"
	);
	anyhow::ensure!(
		!ledger.contains(LubikanAccountId::unique(WARD_DEFINITION_ID)),
		"ward must not drop on kill"
	);
	let row = app
		.world()
		.resource::<EncounterProgressStore>()
		.get(chained_encounter_id())
		.ok_or_else(|| anyhow::anyhow!("progress"))?;
	anyhow::ensure!(row.outcome == Some(EncounterOutcome::Killed), "killed");
	Ok(())
}

#[test]
fn session_end_does_not_unfinish_a_completed_encounter() -> anyhow::Result<()> {
	let mut app = app();
	let owner = app.world_mut().spawn(LubikanOwner).id();
	app.update();
	let boss = spawn_instance(app.world_mut(), chained_encounter_id(), 100.0);
	app.update();
	app.world_mut()
		.write_message(Died { entity: boss, source: Some(owner), point: Vec3::ZERO });
	app.update();
	app.world_mut().write_message(WorldPlaySessionEnded);
	app.update();
	let row = app
		.world()
		.resource::<EncounterProgressStore>()
		.get(chained_encounter_id())
		.ok_or_else(|| anyhow::anyhow!("progress"))?;
	anyhow::ensure!(row.completed_once, "finished flag");
	anyhow::ensure!(row.outcome == Some(EncounterOutcome::Killed), "still killed");
	Ok(())
}
