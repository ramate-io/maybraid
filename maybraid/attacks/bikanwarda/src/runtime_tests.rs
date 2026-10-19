use std::time::Duration;

use anyhow::{anyhow, Result};
use attacks::{
	AttackCatalog, AttackEndReason, AttackEnded, AttackInstance, AttackPresentationBinding,
	AttackStage, AttackSuspended, AttackUser, AttacksPlugin, BeginAttack, CancelAttacks,
};
use avian3d::prelude::{Collider, PhysicsPlugins, Sensor};
use bevy::asset::AssetPlugin;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use combat_targeting::{CombatContact, CombatTargeting};
use damage::{apply_hits, DamageApplied, Died, Health, Hit};
use lod_avian::PhysicsInteractionLayer;

use crate::{
	BikanwardaAttackParams, BikanwardaAttacksPlugin, BikanwardaMouthSocket, BrayAnimationCue,
	BrayAnimationPhase, EyeGlowCue, GodBeamVfxCue, ImpactCue, ParadeExplosionCue, ParadeParams,
	SonicWaveVfxCue, SonicWaveVfxKind, SpatialRumbleCue, SpatialRumblePreset, BRAY, GOD_BEAM,
	PARADE, PARADE_BLAST,
};

fn test_params() -> BikanwardaAttackParams {
	let mut params = BikanwardaAttackParams::PLACEHOLDER;
	params.parade = ParadeParams {
		windup: 0.2,
		recovery: 0.1,
		cooldown: 0.0,
		blast_count: 3,
		interval: 0.25,
		first_distance: 2.0,
		spacing: 2.0,
		blast_radius: 1.1,
		blast_execute: 0.15,
		line_half_width: 0.4,
		blast_height: 1.0,
		blast_damage: 20.0,
		blast_falloff: None,
	};
	params.god_beam.windup = 0.2;
	params.god_beam.recovery = 0.1;
	params.god_beam.cooldown = 0.0;
	params.god_beam.column_height = 8.0;
	params.bray.windup = 0.2;
	params.bray.recovery = 0.1;
	params.bray.cooldown = 0.0;
	params.bray.radius = 1.4;
	params.bray.speed = 8.0;
	params.bray.range = 8.0;
	params.bray.waves = 2;
	params.bray.interval = 0.3;
	params.bray.impact_delay = 0.05;
	params
}

fn attack_app() -> App {
	let mut app = App::new();
	app.insert_resource(test_params());
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
		.add_systems(PostUpdate, apply_hits)
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(0.05)));
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

fn spawn_owner(app: &mut App, translation: Vec3) -> Entity {
	let mouth_at = translation + Vec3::new(0.0, 1.6, -0.4);
	let owner = app
		.world_mut()
		.spawn((
			AttackUser::default(),
			Transform::from_translation(translation),
			GlobalTransform::from_translation(translation),
		))
		.id();
	app.world_mut().spawn((
		BikanwardaMouthSocket,
		ChildOf(owner),
		Transform::from_translation(Vec3::new(0.0, 1.6, -0.4)),
		GlobalTransform::from_translation(mouth_at),
	));
	owner
}

fn spawn_target(app: &mut App, translation: Vec3, health: f32) -> Entity {
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

fn observe(app: &mut App, owner: Entity, target: Entity, point: Vec3) {
	let mut targeting = app.world().get::<CombatTargeting>(owner).cloned().unwrap_or_default();
	targeting.upsert_contact(CombatContact {
		subject: target,
		position: point,
		movement_vector: Vec3::ZERO,
		visible_point: point,
		visible_head: None,
		last_spotted_at: 0.0,
	});
	targeting.rebalance(0.0);
	app.world_mut().entity_mut(owner).insert(targeting);
}

fn mouth(app: &mut App, owner: Entity) -> Result<Entity> {
	let mut sockets = app.world_mut().query::<(Entity, &ChildOf, &BikanwardaMouthSocket)>();
	sockets
		.iter(app.world())
		.find(|(_, parent, _)| parent.parent() == owner)
		.map(|(entity, _, _)| entity)
		.ok_or_else(|| anyhow!("owner is missing a mouth socket"))
}

fn cast(app: &mut App, owner: Entity, definition: attacks::AttackId, target: Entity, point: Vec3) {
	observe(app, owner, target, point);
	let socket = mouth(app, owner).ok();
	let mut begin = BeginAttack::new(owner, definition).with_target(target);
	if let Some(socket) = socket {
		begin = begin.with_emission_socket(socket);
	}
	app.world_mut().write_message(begin);
}

fn live_ids(app: &mut App) -> Vec<attacks::AttackId> {
	let mut query = app.world_mut().query::<&AttackInstance>();
	query.iter(app.world()).map(|instance| instance.definition).collect()
}

fn first_instance(app: &mut App, id: attacks::AttackId) -> Option<AttackInstance> {
	let mut query = app.world_mut().query::<&AttackInstance>();
	query.iter(app.world()).find(|instance| instance.definition == id).cloned()
}

fn drain_hits(app: &mut App) -> Vec<Hit> {
	app.world_mut().resource_mut::<Messages<Hit>>().drain().collect()
}

fn drain_ended(app: &mut App) -> Vec<AttackEnded> {
	app.world_mut().resource_mut::<Messages<AttackEnded>>().drain().collect()
}

#[test]
fn catalog_registers_the_four_shared_ids() -> Result<()> {
	let app = attack_app();
	let catalog = app.world().resource::<AttackCatalog>();
	for id in [PARADE, PARADE_BLAST, GOD_BEAM, BRAY] {
		if !catalog.contains(id) {
			return Err(anyhow!("missing {}", id.as_str()));
		}
	}
	Ok(())
}

#[test]
fn god_beam_locks_aim_point_and_writes_attributed_hits() -> Result<()> {
	let mut app = attack_app();
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -4.0), 200.0);
	cast(&mut app, owner, GOD_BEAM, target, Vec3::new(0.0, 0.0, -4.0));
	step(&mut app, 0.05);
	let instance =
		first_instance(&mut app, GOD_BEAM).ok_or_else(|| anyhow!("expected god beam windup"))?;
	if instance.stage != AttackStage::Windup {
		return Err(anyhow!("expected windup, got {:?}", instance.stage));
	}
	assert!((instance.snapshot.aim_point - Vec3::new(0.0, 0.0, -4.0)).length() < 1e-3);
	assert!((instance.snapshot.origin - Vec3::ZERO).length() < 1e-3);
	step(&mut app, 0.2);
	let instance =
		first_instance(&mut app, GOD_BEAM).ok_or_else(|| anyhow!("expected god beam execute"))?;
	if instance.stage != AttackStage::Execute {
		return Err(anyhow!("expected execute, got {:?}", instance.stage));
	}
	assert!((instance.snapshot.aim_point - Vec3::new(0.0, 0.0, -4.0)).length() < 1e-3);
	step(&mut app, 0.1);
	let hits = drain_hits(&mut app);
	let hit = hits
		.iter()
		.find(|hit| hit.target == target)
		.ok_or_else(|| anyhow!("expected a Hit"))?;
	assert_eq!(hit.source, Some(owner));
	assert!(!hit.allow_self);
	assert!((hit.amount - test_params().god_beam.damage).abs() < 1e-4);
	Ok(())
}

#[test]
fn god_beam_execution_is_ten_seconds_in_the_catalog() -> Result<()> {
	let app = attack_app();
	let catalog = app.world().resource::<AttackCatalog>();
	let definition = catalog.get(GOD_BEAM).ok_or_else(|| anyhow!("missing god beam"))?;
	assert!((definition.timing.execution - 10.0).abs() < 1e-6);
	Ok(())
}

#[test]
fn god_beam_adapter_emits_vfx_and_eye_glow_cues() -> Result<()> {
	let mut app = attack_app();
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -4.0), 200.0);
	cast(&mut app, owner, GOD_BEAM, target, Vec3::new(0.0, 0.0, -4.0));
	step(&mut app, 0.05);
	let glow: Vec<EyeGlowCue> =
		app.world_mut().resource_mut::<Messages<EyeGlowCue>>().drain().collect();
	if !glow.iter().any(|cue| cue.active) {
		return Err(anyhow!("windup must emit EyeGlowCue"));
	}
	step(&mut app, 0.2);
	let beams: Vec<GodBeamVfxCue> =
		app.world_mut().resource_mut::<Messages<GodBeamVfxCue>>().drain().collect();
	let beam = beams
		.last()
		.ok_or_else(|| anyhow!("execute must emit GodBeamVfxCue"))?;
	if (beam.diameter - 7.0).abs() >= 1e-3 {
		return Err(anyhow!("expected 7 m diameter, got {}", beam.diameter));
	}
	if (beam.active_secs - 10.0).abs() >= 1e-3 {
		return Err(anyhow!("expected 10 s active_secs"));
	}
	Ok(())
}

#[test]
fn parade_walks_blasts_along_the_line() -> Result<()> {
	let mut app = attack_app();
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let near = spawn_target(&mut app, Vec3::new(0.0, 1.0, -2.0), 200.0);
	let far = spawn_target(&mut app, Vec3::new(0.0, 1.0, -6.0), 200.0);
	observe(&mut app, owner, far, Vec3::new(0.0, 1.0, -6.0));
	app.world_mut().write_message(BeginAttack::new(owner, PARADE).with_target(far));
	step(&mut app, 0.05);
	step(&mut app, 0.2);
	let parent =
		first_instance(&mut app, PARADE).ok_or_else(|| anyhow!("expected parade execute"))?;
	if parent.stage != AttackStage::Execute {
		return Err(anyhow!("expected execute, got {:?}", parent.stage));
	}
	step(&mut app, 0.05);
	step(&mut app, 0.15);
	let early = drain_hits(&mut app);
	if early.iter().any(|hit| hit.target == far) {
		return Err(anyhow!("far target must wait for a later blast"));
	}
	if !early.iter().any(|hit| hit.target == near && hit.source == Some(owner)) {
		return Err(anyhow!("first blast should hit the near target, got {early:?}"));
	}
	step(&mut app, 0.25);
	step(&mut app, 0.25);
	step(&mut app, 0.2);
	let later = drain_hits(&mut app);
	if !later.iter().any(|hit| hit.target == far && hit.source == Some(owner)) {
		return Err(anyhow!("later blast should hit the far target, got {later:?}"));
	}
	Ok(())
}

#[test]
fn parade_emits_fiery_explosion_cues() -> Result<()> {
	let mut app = attack_app();
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -4.0), 200.0);
	cast(&mut app, owner, PARADE, target, Vec3::new(0.0, 1.0, -4.0));
	step(&mut app, 0.05);
	step(&mut app, 0.2);
	step(&mut app, 0.05);
	step(&mut app, 0.15);
	let cues: Vec<ParadeExplosionCue> =
		app.world_mut().resource_mut::<Messages<ParadeExplosionCue>>().drain().collect();
	if cues.is_empty() {
		return Err(anyhow!("parade blasts must emit ParadeExplosionCue for fiery_explosion"));
	}
	assert_eq!(cues[0].owner, owner);
	Ok(())
}

#[test]
fn bray_emits_mouth_waves_and_separate_cues() -> Result<()> {
	let mut app = attack_app();
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -3.0), 200.0);
	cast(&mut app, owner, BRAY, target, Vec3::new(0.0, 1.0, -3.0));
	step(&mut app, 0.05);
	step(&mut app, 0.2);
	step(&mut app, 0.1);
	let instance = first_instance(&mut app, BRAY).ok_or_else(|| anyhow!("expected bray"))?;
	assert!((instance.snapshot.emission.y - 1.6).abs() < 0.3);
	let rumbles: Vec<SpatialRumbleCue> =
		app.world_mut().resource_mut::<Messages<SpatialRumbleCue>>().drain().collect();
	let impacts: Vec<ImpactCue> =
		app.world_mut().resource_mut::<Messages<ImpactCue>>().drain().collect();
	if rumbles.is_empty() {
		return Err(anyhow!("bray waves must dispatch SpatialRumbleCue"));
	}
	if !rumbles.iter().any(|cue| cue.preset == SpatialRumblePreset::BrayWave) {
		return Err(anyhow!("wave rumble must use BrayWave preset"));
	}
	if !rumbles.iter().any(|cue| cue.preset == SpatialRumblePreset::HeavyLanding) {
		return Err(anyhow!("ImpactCue must map to HeavyLanding SpatialRumbleCue"));
	}
	let sonic: Vec<SonicWaveVfxCue> =
		app.world_mut().resource_mut::<Messages<SonicWaveVfxCue>>().drain().collect();
	if !sonic.iter().any(|cue| matches!(cue.kind, SonicWaveVfxKind::Spawn { .. })) {
		return Err(anyhow!("bray must emit SonicWaveVfxCue::Spawn"));
	}
	if !sonic.iter().any(|cue| matches!(cue.kind, SonicWaveVfxKind::Follow(_))) {
		return Err(anyhow!("bray must emit SonicWaveVfxCue::Follow"));
	}
	let anims: Vec<BrayAnimationCue> =
		app.world_mut().resource_mut::<Messages<BrayAnimationCue>>().drain().collect();
	if !anims.iter().any(|cue| cue.phase == BrayAnimationPhase::Execute) {
		return Err(anyhow!("bray execute must emit BrayAnimationCue"));
	}
	if impacts.is_empty() {
		return Err(anyhow!("bray execute must dispatch a separate ImpactCue"));
	}
	step(&mut app, 0.35);
	let hits = drain_hits(&mut app);
	if !hits.iter().any(|hit| hit.target == target && hit.source == Some(owner)) {
		return Err(anyhow!("bray wave should write an attributed Hit"));
	}
	Ok(())
}

#[test]
fn sleep_cancels_god_beam_and_clears_eye_glow() -> Result<()> {
	let mut app = attack_app();
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -4.0), 200.0);
	cast(&mut app, owner, GOD_BEAM, target, Vec3::new(0.0, 0.0, -4.0));
	step(&mut app, 0.05);
	let glow: Vec<EyeGlowCue> =
		app.world_mut().resource_mut::<Messages<EyeGlowCue>>().drain().collect();
	if !glow.iter().any(|cue| cue.active) {
		return Err(anyhow!("windup must turn eye glow on"));
	}
	app.world_mut().entity_mut(owner).insert(AttackSuspended);
	step(&mut app, 0.05);
	if live_ids(&mut app).contains(&GOD_BEAM) {
		return Err(anyhow!("sleep must tear down god beam"));
	}
	let ended = drain_ended(&mut app);
	if !ended.iter().any(|event| event.reason == AttackEndReason::OwnerSuspended) {
		return Err(anyhow!("expected OwnerSuspended, got {ended:?}"));
	}
	let glow: Vec<EyeGlowCue> =
		app.world_mut().resource_mut::<Messages<EyeGlowCue>>().drain().collect();
	if !glow.iter().any(|cue| !cue.active) {
		return Err(anyhow!("sleep must clear eye glow"));
	}
	let mut bindings = app.world_mut().query::<&AttackPresentationBinding>();
	if bindings.iter(app.world()).next().is_some() {
		return Err(anyhow!("sleep must despawn presentation bindings"));
	}
	Ok(())
}

#[test]
fn cancel_stops_remaining_parade_blasts() -> Result<()> {
	let mut app = attack_app();
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -6.0), 200.0);
	cast(&mut app, owner, PARADE, target, Vec3::new(0.0, 1.0, -6.0));
	step(&mut app, 0.05);
	step(&mut app, 0.2);
	app.world_mut().write_message(CancelAttacks::all(owner));
	step(&mut app, 0.05);
	if !live_ids(&mut app).is_empty() {
		return Err(anyhow!("cancel must clear parade and any live blasts"));
	}
	Ok(())
}

#[test]
fn friendly_fire_skips_listed_allies() -> Result<()> {
	let mut app = attack_app();
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let ally = spawn_target(&mut app, Vec3::new(0.0, 1.0, -4.0), 200.0);
	let foe = spawn_target(&mut app, Vec3::new(0.4, 1.0, -4.0), 200.0);
	observe(&mut app, owner, foe, Vec3::new(0.0, 0.0, -4.0));
	app.world_mut()
		.write_message(BeginAttack::new(owner, GOD_BEAM).with_target(foe).with_allies(vec![ally]));
	step(&mut app, 0.05);
	step(&mut app, 0.2);
	step(&mut app, 0.1);
	let hits = drain_hits(&mut app);
	if hits.iter().any(|hit| hit.target == ally) {
		return Err(anyhow!("ally must be excluded, got {hits:?}"));
	}
	if !hits.iter().any(|hit| hit.target == foe) {
		return Err(anyhow!("foe should still be hit"));
	}
	Ok(())
}

#[test]
fn summoned_owner_reuses_the_same_ids() -> Result<()> {
	let mut app = attack_app();
	let encounter = spawn_owner(&mut app, Vec3::new(-4.0, 0.0, 0.0));
	let summon = spawn_owner(&mut app, Vec3::new(4.0, 0.0, 0.0));
	let victim = spawn_target(&mut app, Vec3::new(4.0, 1.0, -4.0), 200.0);
	observe(&mut app, summon, victim, Vec3::new(4.0, 0.0, -4.0));
	app.world_mut().write_message(
		BeginAttack::new(summon, GOD_BEAM)
			.with_target(victim)
			.with_allies(vec![encounter]),
	);
	step(&mut app, 0.05);
	let instance =
		first_instance(&mut app, GOD_BEAM).ok_or_else(|| anyhow!("summon should cast god beam"))?;
	assert_eq!(instance.owner, summon);
	assert_eq!(instance.definition, GOD_BEAM);
	assert!(instance.allies.contains(&encounter));
	Ok(())
}
