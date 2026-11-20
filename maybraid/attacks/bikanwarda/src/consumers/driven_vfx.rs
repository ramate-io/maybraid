//! [`GodBeamVfxCue`] and [`SonicWaveVfxCue`] consumers.
//!
//! Bray rings use [`maybraid_vfx::SoundwaveMaterial`] in the existing VFX crate.
//! God Beam still draws a labeled PLACEHOLDER mesh until a driven column lands.

use std::collections::HashMap;

use attacks::{
	AttackInstanceId, AttackPresentationBinding, AttackPresentationBindingKind, PresentationPhase,
	WavefrontPresentation,
};
use bevy::prelude::*;
use maybraid_vfx::{
	attach_soundwave_trail, fade_trail, soundwave_torus_mesh, soundwave_trail_transform,
	soundwave_transform, soundwave_wake_mesh, soundwave_wake_transform, EffectProperties,
	SoundwaveMaterial, SoundwaveMaterialPlugin, VfxLibrary,
};

use crate::consumers::BikanwardaPresentationSystems;
use crate::{GodBeamVfxCue, SonicWaveVfxCue, SonicWaveVfxKind};

#[derive(Resource, Default)]
struct PlaceholderDrivenAssets {
	beam: Option<Handle<Mesh>>,
	beam_mat: Option<Handle<StandardMaterial>>,
}

#[derive(Resource, Default)]
struct SoundwaveAssets {
	ring: Option<Handle<Mesh>>,
	wake: Option<Handle<Mesh>>,
}

struct SoundwaveRing {
	ring: Entity,
	wake: Entity,
	trail: Entity,
	wisps: Entity,
	streaks: Entity,
}

#[derive(Resource, Default)]
struct DrivenBindings {
	beam: HashMap<AttackInstanceId, Entity>,
	sonic_root: HashMap<AttackInstanceId, Entity>,
	rings: HashMap<(AttackInstanceId, u8), SoundwaveRing>,
}

pub fn plugin(app: &mut App, placeholder: bool) {
	if !app.is_plugin_added::<SoundwaveMaterialPlugin>() {
		app.add_plugins(SoundwaveMaterialPlugin);
	}
	app.init_resource::<SoundwaveAssets>()
		.init_resource::<DrivenBindings>()
		.add_systems(Startup, seed_soundwave_assets)
		.add_systems(Update, consume_sonic_wave.in_set(BikanwardaPresentationSystems::Consume));
	if placeholder {
		app.init_resource::<PlaceholderDrivenAssets>()
			.add_systems(Startup, seed_placeholder_driven_assets)
			.add_systems(
				Update,
				consume_god_beam_placeholder.in_set(BikanwardaPresentationSystems::Consume),
			);
	}
}

fn seed_soundwave_assets(mut assets: ResMut<SoundwaveAssets>, mut meshes: ResMut<Assets<Mesh>>) {
	assets.ring = Some(meshes.add(soundwave_torus_mesh()));
	assets.wake = Some(meshes.add(soundwave_wake_mesh()));
}

fn seed_placeholder_driven_assets(
	mut assets: ResMut<PlaceholderDrivenAssets>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	assets.beam = Some(meshes.add(Cylinder::new(0.5, 1.0)));
	assets.beam_mat = Some(materials.add(StandardMaterial {
		base_color: Color::srgba(1.0, 0.55, 0.12, 0.55),
		emissive: LinearRgba::rgb(4.0, 1.2, 0.2),
		alpha_mode: AlphaMode::Blend,
		unlit: true,
		..default()
	}));
}

fn consume_god_beam_placeholder(
	mut cues: MessageReader<GodBeamVfxCue>,
	assets: Res<PlaceholderDrivenAssets>,
	mut bindings: ResMut<DrivenBindings>,
	mut transforms: Query<&mut Transform>,
	mut commands: Commands,
) {
	let (Some(mesh), Some(material)) = (assets.beam.as_ref(), assets.beam_mat.as_ref()) else {
		return;
	};
	for cue in cues.read() {
		match cue.phase {
			PresentationPhase::Spawned | PresentationPhase::Updated => {
				let height = cue.gameplay_height.max(0.1);
				let scale = Vec3::new(cue.diameter.max(0.1), height, cue.diameter.max(0.1));
				let transform =
					Transform::from_translation(cue.ground + Vec3::new(0.0, height * 0.5, 0.0))
						.with_scale(scale);
				if let Some(entity) = bindings.beam.get(&cue.instance) {
					if let Ok(mut tf) = transforms.get_mut(*entity) {
						*tf = transform;
					}
				} else {
					let entity = commands
						.spawn((
							Name::new("PLACEHOLDER-god-beam"),
							Mesh3d(mesh.clone()),
							MeshMaterial3d(material.clone()),
							transform,
							AttackPresentationBinding {
								instance: cue.instance,
								kind: AttackPresentationBindingKind::Hazard {
									index: cue.hazard_index,
									wave: 0,
								},
							},
						))
						.id();
					bindings.beam.insert(cue.instance, entity);
				}
			}
			PresentationPhase::Despawned => {
				if let Some(entity) = bindings.beam.remove(&cue.instance) {
					commands.entity(entity).try_despawn();
				}
			}
		}
	}
}

fn consume_sonic_wave(
	mut cues: MessageReader<SonicWaveVfxCue>,
	assets: Res<SoundwaveAssets>,
	library: Option<Res<VfxLibrary>>,
	mut bindings: ResMut<DrivenBindings>,
	mut transforms: Query<&mut Transform>,
	mut materials: ResMut<Assets<SoundwaveMaterial>>,
	handles: Query<&MeshMaterial3d<SoundwaveMaterial>>,
	mut properties: Query<&mut EffectProperties>,
	mut commands: Commands,
) {
	let (Some(mesh), Some(wake)) = (assets.ring.as_ref(), assets.wake.as_ref()) else {
		return;
	};
	for cue in cues.read() {
		match (&cue.kind, cue.phase) {
			(
				SonicWaveVfxKind::Follow(wave),
				PresentationPhase::Spawned | PresentationPhase::Updated,
			) => {
				follow_wave(
					&mut commands,
					&mut bindings,
					&mut transforms,
					&mut materials,
					&handles,
					&mut properties,
					library.as_deref(),
					mesh,
					wake,
					cue.instance,
					cue.hazard_index,
					*wave,
				);
			}
			(SonicWaveVfxKind::Despawn, _) | (_, PresentationPhase::Despawned) => {
				bindings.rings.retain(|(instance, _), ring| {
					if *instance == cue.instance {
						commands.entity(ring.ring).try_despawn();
						commands.entity(ring.wake).try_despawn();
						commands.entity(ring.trail).try_despawn();
						false
					} else {
						true
					}
				});
				if let Some(root) = bindings.sonic_root.remove(&cue.instance) {
					commands.entity(root).try_despawn();
				}
			}
			_ => {}
		}
	}
}

fn follow_wave(
	commands: &mut Commands,
	bindings: &mut DrivenBindings,
	transforms: &mut Query<&mut Transform>,
	materials: &mut Assets<SoundwaveMaterial>,
	handles: &Query<&MeshMaterial3d<SoundwaveMaterial>>,
	properties: &mut Query<&mut EffectProperties>,
	library: Option<&VfxLibrary>,
	mesh: &Handle<Mesh>,
	wake_mesh: &Handle<Mesh>,
	instance: AttackInstanceId,
	hazard_index: usize,
	wave: WavefrontPresentation,
) {
	let key = (instance, wave.wave_index);
	let transform = soundwave_transform(wave.position, wave.direction, wave.radius, wave.thickness);
	let wake_transform =
		soundwave_wake_transform(wave.position, wave.direction, wave.radius, wave.thickness);
	let trail_transform = soundwave_trail_transform(wave.position, wave.direction, wave.radius);
	if let Some(ring) = bindings.rings.get(&key) {
		if let Ok(mut tf) = transforms.get_mut(ring.ring) {
			*tf = transform;
		}
		if let Ok(mut tf) = transforms.get_mut(ring.wake) {
			*tf = wake_transform;
		}
		if let Ok(mut tf) = transforms.get_mut(ring.trail) {
			*tf = trail_transform;
		}
		if let Ok(handle) = handles.get(ring.ring) {
			if let Some(mut material) = materials.get_mut(&handle.0) {
				follow_soundwave_material(&mut material, wave);
			}
		}
		fade_trail_entity(properties, ring.wisps, wave.trailing_fade());
		fade_trail_entity(properties, ring.streaks, wave.trailing_fade());
		return;
	}
	let mut material = SoundwaveMaterial::new(
		wave.position,
		wave.direction,
		wave.radius,
		wave.thickness,
		Color::srgb(0.72, 0.88, 1.0),
		1.2,
	);
	follow_soundwave_material(&mut material, wave);
	let material = materials.add(material);
	let binding = AttackPresentationBinding {
		instance,
		kind: AttackPresentationBindingKind::Hazard { index: hazard_index, wave: wave.wave_index },
	};
	let ring = commands
		.spawn((
			Name::new("vfx-soundwave-bray"),
			Mesh3d(mesh.clone()),
			MeshMaterial3d(material.clone()),
			transform,
			// Torus and ribbon meshes read as filled ovals from the player.
			// Keep the entities for live uniforms / heading tests; draw particles.
			Visibility::Hidden,
			binding,
		))
		.id();
	let wake = commands
		.spawn((
			Name::new("vfx-soundwave-bray-wake"),
			Mesh3d(wake_mesh.clone()),
			MeshMaterial3d(material),
			wake_transform,
			Visibility::Hidden,
			binding,
		))
		.id();
	let trail = commands
		.spawn((
			Name::new("vfx-soundwave-bray-trail"),
			trail_transform,
			Visibility::Inherited,
			binding,
		))
		.id();
	let (wisps, streaks) = attach_soundwave_trail(
		commands,
		trail,
		library.and_then(|lib| lib.soundwave.wisps.as_ref()),
		library.and_then(|lib| lib.soundwave.streaks.as_ref()),
	);
	bindings.rings.insert(key, SoundwaveRing { ring, wake, trail, wisps, streaks });
}

fn fade_trail_entity(properties: &mut Query<&mut EffectProperties>, entity: Entity, fade: f32) {
	if let Ok(mut props) = properties.get_mut(entity) {
		fade_trail(&mut props, fade);
	}
}

fn follow_soundwave_material(material: &mut SoundwaveMaterial, wave: WavefrontPresentation) {
	material.follow(wave.position, wave.direction, wave.radius, wave.thickness);
	material.set_age(wave.travel_age());
	material.set_fade(wave.trailing_fade());
}
