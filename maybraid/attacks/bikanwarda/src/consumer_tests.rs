//! Regression tests for presentation consumers (requires `presentation-consumers`).

use anyhow::anyhow;
use bevy::prelude::*;
use character_motion::{
	ActorBodyRig, AnimClip, AnimRef, AnimRefRoot, CharacterRig, CharacterRigRole,
};
use maybraid_spatial_rumble::{SpatialRumblePlugin, SpatialRumbleSource};

use crate::runtime_tests::{attack_app_with_extra_plugins, cast, spawn_owner, spawn_target, step};
use crate::{
	BikanwardaAttackParams, BikanwardaPresentationConsumersPlugin, PendingEyeGlow,
	SpatialRumbleCue, BRAY, GOD_BEAM,
};

fn consumer_app() -> App {
	attack_app_with_extra_plugins(|app| {
		app.add_plugins(SpatialRumblePlugin);
		app.add_plugins(BikanwardaPresentationConsumersPlugin { placeholder_driven_vfx: false });
	})
}

fn spawn_actor_with_body_rig(app: &mut App, translation: Vec3, prior: AnimRef) -> (Entity, Entity) {
	let owner = spawn_owner(app, translation);
	let rig = app
		.world_mut()
		.spawn((
			CharacterRig { role: CharacterRigRole::Body, skeleton: Default::default() },
			AnimRefRoot(prior),
			ChildOf(owner),
		))
		.id();
	app.world_mut().entity_mut(owner).insert(ActorBodyRig::of(rig));
	(owner, rig)
}

#[test]
fn spatial_rumble_cue_presets_map_to_spatial_rumble() {
	let params = BikanwardaAttackParams::PLACEHOLDER;
	let instance = attacks::AttackInstanceId(1);
	let owner = Entity::from_bits(9);
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
	let prior = AnimRef::new(AnimClip::quadruped_run());
	let (owner, rig) = spawn_actor_with_body_rig(&mut app, Vec3::ZERO, prior);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -3.0), 200.0);
	cast(&mut app, owner, BRAY, target, Vec3::new(0.0, 1.0, -3.0));
	step(&mut app, 0.05);
	step(&mut app, 0.05);
	if app.world().get::<AnimRefRoot>(owner).is_some() {
		return Err(anyhow!("clip must stay on the bound body rig, not the gameplay actor"));
	}
	let anim = app
		.world()
		.get::<AnimRefRoot>(rig)
		.ok_or_else(|| anyhow!("missing body-rig anim"))?;
	if anim.0.clip.id() != AnimClip::epiphant_bray().id() {
		return Err(anyhow!("consumer must drive Epiphant Bray clip on the bound body rig"));
	}
	Ok(())
}

#[test]
fn bray_animation_consumer_restores_prior_clip_on_the_bound_rig() -> anyhow::Result<()> {
	use crate::{BrayAnimationCue, BrayAnimationPhase};
	use attacks::AttackInstanceId;

	let mut app = consumer_app();
	let prior = AnimRef::new(AnimClip::quadruped_run());
	let (owner, rig) = spawn_actor_with_body_rig(&mut app, Vec3::ZERO, prior);
	let instance = AttackInstanceId(7);
	app.world_mut().write_message(BrayAnimationCue {
		instance,
		owner,
		phase: BrayAnimationPhase::Execute,
	});
	app.update();
	let playing = app
		.world()
		.get::<AnimRefRoot>(rig)
		.ok_or_else(|| anyhow!("missing body-rig anim"))?;
	if playing.0.clip.id() != AnimClip::epiphant_bray().id() {
		return Err(anyhow!("direct Execute cue must play Bray on the bound body rig"));
	}
	app.world_mut().write_message(BrayAnimationCue {
		instance,
		owner,
		phase: BrayAnimationPhase::Cancelled,
	});
	app.update();
	let restored = app
		.world()
		.get::<AnimRefRoot>(rig)
		.ok_or_else(|| anyhow!("missing body-rig anim"))?;
	if restored.0.clip.id() != prior.clip.id() {
		return Err(anyhow!(
			"Cancelled cue must restore the prior clip on the bound body rig, got {:?}",
			restored.0.clip.id()
		));
	}
	Ok(())
}

#[test]
fn bray_animation_consumer_restores_after_real_bray_completes() -> anyhow::Result<()> {
	let mut app = consumer_app();
	let prior = AnimRef::new(AnimClip::quadruped_run());
	let (owner, rig) = spawn_actor_with_body_rig(&mut app, Vec3::ZERO, prior);
	let target = spawn_target(&mut app, Vec3::new(0.0, 1.0, -3.0), 200.0);
	cast(&mut app, owner, BRAY, target, Vec3::new(0.0, 1.0, -3.0));
	let mut saw_bray = false;
	for _ in 0..40 {
		step(&mut app, 0.2);
		let clip_id = app
			.world()
			.get::<AnimRefRoot>(rig)
			.ok_or_else(|| anyhow!("body rig lost AnimRefRoot"))?
			.0
			.clip
			.id();
		if clip_id == AnimClip::epiphant_bray().id() {
			saw_bray = true;
		} else if saw_bray && clip_id == prior.clip.id() {
			return Ok(());
		}
	}
	let anim = app
		.world()
		.get::<AnimRefRoot>(rig)
		.ok_or_else(|| anyhow!("missing body-rig anim"))?;
	Err(anyhow!(
		"expected Bray then restore of prior clip on the body rig; saw_bray={saw_bray}, final={:?}",
		anim.0.clip.id()
	))
}

#[test]
fn sonic_wave_torus_maps_y_axle_to_heading() -> anyhow::Result<()> {
	use attacks::{AttackInstanceId, PresentationPhase, WavefrontPresentation};
	use maybraid_vfx::{soundwave_transform, soundwave_wake_transform};

	let mut app = consumer_app();
	app.world_mut().write_message(crate::SonicWaveVfxCue {
		instance: AttackInstanceId(3),
		owner: Entity::from_bits(9),
		phase: PresentationPhase::Spawned,
		hazard_index: 0,
		kind: crate::SonicWaveVfxKind::Follow(WavefrontPresentation {
			origin: Vec3::new(1.0, 2.0, 3.0),
			direction: Vec3::X,
			position: Vec3::new(4.0, 2.0, 3.0),
			speed: 10.0,
			range: 8.0,
			radius: 2.0,
			thickness: 0.7,
			wave_index: 0,
		}),
	});
	app.update();
	app.update();
	let got = app
		.world_mut()
		.query::<(&Name, &Transform)>()
		.iter(app.world())
		.find(|(name, _)| name.as_str() == "vfx-soundwave-bray")
		.map(|(_, transform)| *transform);
	let Some(got) = got else {
		return Err(anyhow!("Bray consumer must spawn a soundwave ring"));
	};
	let expected = soundwave_transform(Vec3::new(4.0, 2.0, 3.0), Vec3::X, 2.0, 0.7);
	let axle = got.rotation * Vec3::Y;
	if (axle - Vec3::X).length() > 1e-3 {
		return Err(anyhow!("torus Y-axle must map onto the wave heading, got {axle}"));
	}
	let wrong = Quat::from_rotation_arc(Vec3::Z, Vec3::X) * Vec3::Y;
	if (wrong - Vec3::X).length() < 0.5 {
		return Err(anyhow!("regression would accept the Z-to-heading orientation bug"));
	}
	if (got.translation - expected.translation).length() > 1e-3 {
		return Err(anyhow!("ring center must follow the live wave position"));
	}
	let wake = app
		.world_mut()
		.query::<(&Name, &Transform)>()
		.iter(app.world())
		.find(|(name, _)| name.as_str() == "vfx-soundwave-bray-wake")
		.map(|(_, transform)| *transform);
	let Some(wake) = wake else {
		return Err(anyhow!("Bray consumer must spawn a trailing spirit wake"));
	};
	let expected_wake = soundwave_wake_transform(Vec3::new(4.0, 2.0, 3.0), Vec3::X, 2.0, 0.7);
	if (wake.translation - expected_wake.translation).length() > 1e-3 {
		return Err(anyhow!(
			"wake must trail behind the live front, got {} expected {}",
			wake.translation,
			expected_wake.translation
		));
	}
	let first = soundwave_material(&mut app)?;
	let first_center = Vec3::new(first.wave.x, first.wave.y, first.wave.z);
	if (first_center - Vec3::new(4.0, 2.0, 3.0)).length() > 1e-3 {
		return Err(anyhow!(
			"compression uniform must follow the live front, got {first_center} (origin was 1,2,3)"
		));
	}
	if first.extras.x <= 0.0 {
		return Err(anyhow!("spawned wave must carry travel age, got {}", first.extras.x));
	}
	if (first.extras.z - 1.0).abs() < 1e-4 {
		return Err(anyhow!("spawned wave must already trail-fade, got {}", first.extras.z));
	}
	app.world_mut().write_message(crate::SonicWaveVfxCue {
		instance: AttackInstanceId(3),
		owner: Entity::from_bits(9),
		phase: PresentationPhase::Updated,
		hazard_index: 0,
		kind: crate::SonicWaveVfxKind::Follow(WavefrontPresentation {
			origin: Vec3::new(1.0, 2.0, 3.0),
			direction: Vec3::X,
			position: Vec3::new(7.0, 2.0, 3.0),
			speed: 10.0,
			range: 8.0,
			radius: 2.0,
			thickness: 0.7,
			wave_index: 0,
		}),
	});
	app.update();
	let second = soundwave_material(&mut app)?;
	let second_center = Vec3::new(second.wave.x, second.wave.y, second.wave.z);
	if (second_center - Vec3::new(7.0, 2.0, 3.0)).length() > 1e-3 {
		return Err(anyhow!(
			"updated compression uniform must follow the front, got {second_center}"
		));
	}
	if second.extras.x <= first.extras.x + 1e-4 {
		return Err(anyhow!(
			"travel age must advance on update, first={} second={}",
			first.extras.x,
			second.extras.x
		));
	}
	if second.extras.z >= first.extras.z - 1e-4 {
		return Err(anyhow!(
			"trailing fade must drop as the wave travels, first={} second={}",
			first.extras.z,
			second.extras.z
		));
	}
	Ok(())
}

fn soundwave_material(app: &mut App) -> anyhow::Result<maybraid_vfx::SoundwaveMaterial> {
	use maybraid_vfx::SoundwaveMaterial;
	let handle = app
		.world_mut()
		.query::<(&Name, &MeshMaterial3d<SoundwaveMaterial>)>()
		.iter(app.world())
		.find(|(name, _)| name.as_str() == "vfx-soundwave-bray")
		.map(|(_, material)| material.0.clone());
	let Some(handle) = handle else {
		return Err(anyhow!("missing soundwave material handle"));
	};
	app.world()
		.resource::<Assets<SoundwaveMaterial>>()
		.get(&handle)
		.cloned()
		.ok_or_else(|| anyhow!("missing soundwave material asset"))
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
