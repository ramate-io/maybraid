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
	soundwave_torus_mesh, soundwave_transform, SoundwaveMaterial, SoundwaveMaterialPlugin,
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
}

#[derive(Resource, Default)]
struct DrivenBindings {
	beam: HashMap<AttackInstanceId, Entity>,
	sonic_root: HashMap<AttackInstanceId, Entity>,
	rings: HashMap<(AttackInstanceId, u8), Entity>,
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
	mut bindings: ResMut<DrivenBindings>,
	mut transforms: Query<&mut Transform>,
	mut materials: ResMut<Assets<SoundwaveMaterial>>,
	handles: Query<&MeshMaterial3d<SoundwaveMaterial>>,
	mut commands: Commands,
) {
	let Some(mesh) = assets.ring.as_ref() else {
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
					mesh,
					cue.instance,
					cue.hazard_index,
					*wave,
				);
			}
			(SonicWaveVfxKind::Despawn, _) | (_, PresentationPhase::Despawned) => {
				bindings.rings.retain(|(instance, _), entity| {
					if *instance == cue.instance {
						commands.entity(*entity).try_despawn();
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
	mesh: &Handle<Mesh>,
	instance: AttackInstanceId,
	hazard_index: usize,
	wave: WavefrontPresentation,
) {
	let key = (instance, wave.wave_index);
	let transform = soundwave_transform(wave.position, wave.direction, wave.radius, wave.thickness);
	if let Some(entity) = bindings.rings.get(&key).copied() {
		if let Ok(mut tf) = transforms.get_mut(entity) {
			*tf = transform;
		}
		if let Ok(handle) = handles.get(entity) {
			if let Some(mut material) = materials.get_mut(&handle.0) {
				follow_soundwave_material(&mut material, wave);
			}
		}
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
	let entity = commands
		.spawn((
			Name::new("vfx-soundwave-bray"),
			Mesh3d(mesh.clone()),
			MeshMaterial3d(material),
			transform,
			AttackPresentationBinding {
				instance,
				kind: AttackPresentationBindingKind::Hazard {
					index: hazard_index,
					wave: wave.wave_index,
				},
			},
		))
		.id();
	bindings.rings.insert(key, entity);
}

fn follow_soundwave_material(material: &mut SoundwaveMaterial, wave: WavefrontPresentation) {
	material.follow(wave.position, wave.direction, wave.radius, wave.thickness);
	material.set_age(wave.travel_age());
	material.set_fade(wave.trailing_fade());
}
