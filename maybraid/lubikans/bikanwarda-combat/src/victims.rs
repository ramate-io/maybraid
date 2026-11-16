//! Area-damage / victim-filter tests. These do not assert intended-target choice
//! beyond pointing the hazard at a foe so the volume is occupied.
//!
//! Target selection lives in [`super::selection`].

use std::time::Duration;

use anyhow::{anyhow, Result};
use attacks::{AttackUser, AttacksPlugin, BeginAttack};
use avian3d::prelude::{Collider, PhysicsPlugins, Sensor};
use bevy::asset::AssetPlugin;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bikanwarda_attacks::{BikanwardaAttacksPlugin, GOD_BEAM, PARADE_BLAST};
use combat_targeting::{CombatContact, CombatTargeting, TargetSource};
use damage::{apply_hits, DamageApplied, Died, Health, Hit};
use lod_avian::PhysicsInteractionLayer;

use crate::test_support::short_params;
use crate::{allies_for_begin, BikanwardaSummonForm};

fn victim_app() -> App {
	let mut app = App::new();
	app.insert_resource(short_params());
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		PhysicsPlugins::default(),
		AssetPlugin::default(),
		MeshPlugin,
		AttacksPlugin,
		BikanwardaAttacksPlugin { placeholders: false },
	));
	app.finish();
	app.add_message::<DamageApplied>()
		.add_message::<Died>()
		.add_systems(PostUpdate, apply_hits);
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(0.05)));
	app.world_mut()
		.resource_mut::<Time<Virtual>>()
		.set_max_delta(Duration::from_secs(30));
	app.update();
	app
}

fn step(app: &mut App, dt: f32) {
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(dt)));
	app.update();
}

fn spawn_caster(app: &mut App, translation: Vec3) -> Entity {
	app.world_mut()
		.spawn((
			AttackUser::default(),
			CombatTargeting::default(),
			Transform::from_translation(translation),
			GlobalTransform::from_translation(translation),
		))
		.id()
}

fn spawn_body(app: &mut App, translation: Vec3, health: f32) -> Entity {
	app.world_mut()
		.spawn((
			Health::from_max(health),
			Transform::from_translation(translation),
			GlobalTransform::from_translation(translation),
			Collider::capsule(0.4, 1.0),
			Sensor,
			PhysicsInteractionLayer::animated_layers(),
		))
		.id()
}

fn observe(app: &mut App, caster: Entity, subject: Entity, point: Vec3) {
	let now = app.world().resource::<Time>().elapsed_secs();
	let mut targeting = app.world().get::<CombatTargeting>(caster).cloned().unwrap_or_default();
	targeting.upsert_contact(CombatContact {
		subject,
		position: point,
		movement_vector: Vec3::ZERO,
		visible_point: point,
		visible_head: None,
		last_spotted_at: now,
	});
	targeting.rebalance(now);
	app.world_mut().entity_mut(caster).insert(targeting);
}

fn drain_hits(app: &mut App) -> Vec<Hit> {
	app.world_mut().resource_mut::<Messages<Hit>>().drain().collect()
}

fn owner_targeting_with(app: &mut App, owner: Entity, ally: Option<Entity>) -> CombatTargeting {
	let mut targeting = CombatTargeting::default();
	if let Some(ally) = ally {
		targeting.include(ally, TargetSource::ALLY);
	}
	targeting.include(owner, TargetSource::ALLY);
	let _ = app;
	targeting
}

fn cast_god_beam(app: &mut App, caster: Entity, foe: Entity, point: Vec3, allies: Vec<Entity>) {
	observe(app, caster, foe, point);
	app.world_mut()
		.write_message(BeginAttack::new(caster, GOD_BEAM).with_target(foe).with_allies(allies));
}

fn run_god_beam(app: &mut App) {
	step(app, 0.05);
	step(app, 0.2);
	step(app, 0.1);
}

#[test]
fn victim_app_does_not_need_combat_hud() {
	let app = victim_app();
	assert!(app.world().get_resource::<attacks::AttackCatalog>().is_some());
}

#[test]
fn ward_god_beam_does_not_damage_owner_or_ally_in_the_column() -> Result<()> {
	let mut app = victim_app();
	let column = Vec3::new(0.0, 1.0, -4.0);
	let caster = spawn_caster(&mut app, Vec3::new(4.0, 0.0, 0.0));
	let owner = spawn_body(&mut app, column, 200.0);
	let ally = spawn_body(&mut app, column, 200.0);
	let foe = spawn_body(&mut app, column, 200.0);
	let owner_targeting = owner_targeting_with(&mut app, owner, Some(ally));
	let allies = allies_for_begin(BikanwardaSummonForm::Ward, owner, Some(&owner_targeting));
	if !allies.contains(&owner) || !allies.contains(&ally) {
		return Err(anyhow!("Ward allies must include owner and ally, got {allies:?}"));
	}
	cast_god_beam(&mut app, caster, foe, column, allies);
	run_god_beam(&mut app);
	let hits = drain_hits(&mut app);
	if hits.iter().any(|hit| hit.target == owner) {
		return Err(anyhow!(
			"Ward must not damage the protected owner inside the beam, got {hits:?}"
		));
	}
	if hits.iter().any(|hit| hit.target == ally) {
		return Err(anyhow!("Ward must not damage an ally inside the beam, got {hits:?}"));
	}
	if !hits.iter().any(|hit| hit.target == foe) {
		return Err(anyhow!("Ward must still damage the aimed foe, got {hits:?}"));
	}
	Ok(())
}

#[test]
fn enraged_god_beam_damages_the_summoner_in_the_column() -> Result<()> {
	let mut app = victim_app();
	let column = Vec3::new(0.0, 1.0, -4.0);
	let caster = spawn_caster(&mut app, Vec3::new(4.0, 0.0, 0.0));
	let owner = spawn_body(&mut app, column, 200.0);
	let foe = spawn_body(&mut app, column, 200.0);
	let allies = allies_for_begin(BikanwardaSummonForm::Enraged, owner, None);
	if !allies.is_empty() {
		return Err(anyhow!("Enraged must not stamp the summoner as an ally"));
	}
	cast_god_beam(&mut app, caster, foe, column, allies);
	run_god_beam(&mut app);
	let hits = drain_hits(&mut app);
	if !hits.iter().any(|hit| hit.target == owner) {
		return Err(anyhow!("Enraged must damage the summoner inside the beam, got {hits:?}"));
	}
	if !hits.iter().any(|hit| hit.target == foe) {
		return Err(anyhow!("Enraged must still damage the other combatant, got {hits:?}"));
	}
	Ok(())
}

#[test]
fn ward_parade_blast_skips_owner_enraged_does_not() -> Result<()> {
	let mut app = victim_app();
	let blast = Vec3::new(0.0, 1.0, 0.0);
	let ward = spawn_caster(&mut app, Vec3::new(6.0, 0.0, 0.0));
	let enraged = spawn_caster(&mut app, Vec3::new(8.0, 0.0, 0.0));
	let owner = spawn_body(&mut app, blast, 200.0);
	let foe = spawn_body(&mut app, blast + Vec3::X * 0.3, 200.0);
	let owner_targeting = owner_targeting_with(&mut app, owner, None);
	let ward_allies = allies_for_begin(BikanwardaSummonForm::Ward, owner, Some(&owner_targeting));
	app.world_mut().write_message(
		BeginAttack::new(ward, PARADE_BLAST)
			.with_allies(ward_allies)
			.with_emission(blast),
	);
	step(&mut app, 0.05);
	step(&mut app, 0.15);
	let ward_hits = drain_hits(&mut app);
	if ward_hits.iter().any(|hit| hit.target == owner) {
		return Err(anyhow!("Ward blast must skip the owner, got {ward_hits:?}"));
	}
	if !ward_hits.iter().any(|hit| hit.target == foe) {
		return Err(anyhow!("Ward blast must hit the overlapping foe, got {ward_hits:?}"));
	}

	app.world_mut().write_message(
		BeginAttack::new(enraged, PARADE_BLAST)
			.with_allies(Vec::new())
			.with_emission(blast),
	);
	step(&mut app, 0.05);
	step(&mut app, 0.15);
	let enraged_hits = drain_hits(&mut app);
	if !enraged_hits.iter().any(|hit| hit.target == owner) {
		return Err(anyhow!(
			"Enraged blast must damage the overlapping summoner, got {enraged_hits:?}"
		));
	}
	Ok(())
}

#[test]
fn victim_filter_is_independent_of_who_was_selected() -> Result<()> {
	let mut app = victim_app();
	let column = Vec3::new(0.0, 1.0, -4.0);
	let caster = spawn_caster(&mut app, Vec3::new(4.0, 0.0, 0.0));
	let owner = spawn_body(&mut app, column, 200.0);
	let foe = spawn_body(&mut app, column, 200.0);
	let owner_targeting = owner_targeting_with(&mut app, owner, None);
	let allies = allies_for_begin(BikanwardaSummonForm::Ward, owner, Some(&owner_targeting));
	cast_god_beam(&mut app, caster, foe, column, allies);
	run_god_beam(&mut app);
	let hits = drain_hits(&mut app);
	if hits.iter().any(|hit| hit.target == owner) {
		return Err(anyhow!(
			"owner overlap must still be filtered when the beam is aimed at the foe, got {hits:?}"
		));
	}
	if !hits.iter().any(|hit| hit.target == foe) {
		return Err(anyhow!("foe in the column must still be hit"));
	}
	Ok(())
}
