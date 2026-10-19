//! Regression tests for presentation consumers (requires `presentation-consumers`).

use anyhow::anyhow;
use bevy::prelude::*;
use character_motion::{AnimClip, AnimRef, AnimRefRoot};
use maybraid_spatial_rumble::{SpatialRumblePlugin, SpatialRumbleSource};

use crate::{
	BikanwardaAttackParams, BikanwardaPresentationConsumersPlugin, PendingEyeGlow, SpatialRumbleCue,
	BRAY, GOD_BEAM,
};
use crate::runtime_tests::{attack_app_with_extra_plugins, cast, spawn_owner, spawn_target, step};

fn consumer_app() -> App {
	attack_app_with_extra_plugins(|app| {
		app.add_plugins(SpatialRumblePlugin);
		app.add_plugins(BikanwardaPresentationConsumersPlugin { placeholder_driven_vfx: false });
	})
}

#[test]
fn spatial_rumble_cue_presets_map_to_spatial_rumble() {
	let params = BikanwardaAttackParams::PLACEHOLDER;
	let instance = attacks::AttackInstanceId(1);
	let owner = Entity::from_raw_u32(9).unwrap();
	let wave = SpatialRumbleCue::bray_wave(instance, owner, Vec3::ZERO, &params.bray);
	let wave_rumble = wave.to_spatial_rumble();
	let land = SpatialRumbleCue::heavy_landing(instance, owner, Vec3::ZERO);
	let land_rumble = land.to_spatial_rumble();
	assert!(wave_rumble.falloff.outer_radius > land_rumble.falloff.outer_radius);
	assert!(wave_rumble.duration > 0.0);
	assert!(land_rumble.duration > 0.0);
}

#[test]
fn spatial_rumble_consumer_ingests_bray_wave() -> anyhow::Result<()> {
	let mut app = consumer_app();
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -3.0), 200.0);
	cast(&mut app, owner, BRAY, target, Vec3::new(0.0, 1.0, -3.0));
	step(&mut app, 0.05);
	step(&mut app, 0.2);
	let live = app.world_mut().query::<&SpatialRumbleSource>().iter(app.world()).count();
	if live == 0 {
		return Err(anyhow!("consumer must ingest at least one SpatialRumble source during Bray"));
	}
	Ok(())
}

#[test]
fn bray_animation_consumer_plays_epiphant_bray_clip() -> anyhow::Result<()> {
	let mut app = consumer_app();
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	app.world_mut().entity_mut(owner).insert(AnimRefRoot(AnimRef::new(AnimClip::still())));
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -3.0), 200.0);
	cast(&mut app, owner, BRAY, target, Vec3::new(0.0, 1.0, -3.0));
	step(&mut app, 0.05);
	step(&mut app, 0.05);
	let anim = app.world().get::<AnimRefRoot>(owner).ok_or_else(|| anyhow!("missing anim"))?;
	if anim.0.clip.id() != AnimClip::epiphant_bray().id() {
		return Err(anyhow!("consumer must drive Epiphant Bray clip"));
	}
	Ok(())
}

#[test]
fn eye_glow_consumer_tracks_pending_owners() -> anyhow::Result<()> {
	let mut app = consumer_app();
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -4.0), 200.0);
	cast(&mut app, owner, GOD_BEAM, target, Vec3::new(0.0, 0.0, -4.0));
	step(&mut app, 0.05);
	let pending = app.world().resource::<PendingEyeGlow>();
	if !pending.owners.contains(&owner) {
		return Err(anyhow!("God Beam windup must register pending eye glow"));
	}
	Ok(())
}
