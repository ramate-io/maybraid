//! [`GodBeamVfxCue`] and [`SonicWaveVfxCue`] consumers.
//!
//! With `vfx-driven` and landed [#1150](https://github.com/ramate-io/maybraid/pull/1150),
//! map cues onto `maybraid-vfx` `god_beam` / `sonic_wave`. Until then, optional
//! **PLACEHOLDER** meshes follow the same cue contract.

use std::collections::HashMap;

use attacks::{
	AttackInstanceId, AttackPresentationBinding, AttackPresentationBindingKind, PresentationPhase,
};
use bevy::prelude::*;

use crate::consumers::BikanwardaPresentationSystems;
use crate::{GodBeamVfxCue, SonicWaveVfxCue, SonicWaveVfxKind};

#[derive(Resource, Default)]
struct PlaceholderDrivenAssets {
	beam: Option<Handle<Mesh>>,
	ring: Option<Handle<Mesh>>,
	beam_mat: Option<Handle<StandardMaterial>>,
	ring_mat: Option<Handle<StandardMaterial>>,
}

#[derive(Resource, Default)]
struct DrivenBindings {
	beam: HashMap<AttackInstanceId, Entity>,
	sonic_root: HashMap<AttackInstanceId, Entity>,
	rings: HashMap<(AttackInstanceId, u8), Entity>,
}

pub fn plugin(app: &mut App, placeholder: bool) {
	if placeholder {
		app.init_resource::<PlaceholderDrivenAssets>()
			.init_resource::<DrivenBindings>()
			.add_systems(Startup, seed_placeholder_driven_assets)
			.add_systems(
				Update,
				(consume_god_beam_placeholder, consume_sonic_wave_placeholder)
					.in_set(BikanwardaPresentationSystems::Consume),
			);
	}
}

fn seed_placeholder_driven_assets(
	mut assets: ResMut<PlaceholderDrivenAssets>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	assets.beam = Some(meshes.add(Cylinder::new(0.5, 1.0)));
	assets.ring = Some(meshes.add(Torus::new(0.55, 0.08)));
	assets.beam_mat = Some(materials.add(StandardMaterial {
		base_color: Color::srgba(1.0, 0.55, 0.12, 0.55),
		emissive: LinearRgba::rgb(4.0, 1.2, 0.2),
		alpha_mode: AlphaMode::Blend,
		unlit: true,
		..default()
	}));
	assets.ring_mat = Some(materials.add(StandardMaterial {
		base_color: Color::srgba(0.55, 0.85, 1.0, 0.42),
		emissive: LinearRgba::rgb(1.5, 3.0, 5.0),
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

fn consume_sonic_wave_placeholder(
	mut cues: MessageReader<SonicWaveVfxCue>,
	assets: Res<PlaceholderDrivenAssets>,
	mut bindings: ResMut<DrivenBindings>,
	mut transforms: Query<&mut Transform>,
	mut commands: Commands,
) {
	let (Some(mesh), Some(material)) = (assets.ring.as_ref(), assets.ring_mat.as_ref()) else {
		return;
	};
	for cue in cues.read() {
		match (&cue.kind, cue.phase) {
			(
				SonicWaveVfxKind::Follow(wave),
				PresentationPhase::Spawned | PresentationPhase::Updated,
			) => {
				let key = (cue.instance, wave.wave_index);
				let heading = wave.direction.normalize_or(Vec3::Z);
				let rotation = Quat::from_rotation_arc(Vec3::Z, heading);
				let transform = Transform::from_translation(wave.position)
					.with_rotation(rotation)
					.with_scale(Vec3::new(
						wave.radius.max(0.05) * 2.0,
						wave.thickness.max(0.05),
						wave.radius.max(0.05) * 2.0,
					));
				if let Some(entity) = bindings.rings.get(&key) {
					if let Ok(mut tf) = transforms.get_mut(*entity) {
						*tf = transform;
					}
				} else {
					let entity = commands
						.spawn((
							Name::new("PLACEHOLDER-sonic-ring"),
							Mesh3d(mesh.clone()),
							MeshMaterial3d(material.clone()),
							transform,
							AttackPresentationBinding {
								instance: cue.instance,
								kind: AttackPresentationBindingKind::Hazard {
									index: cue.hazard_index,
									wave: wave.wave_index,
								},
							},
						))
						.id();
					bindings.rings.insert(key, entity);
				}
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
