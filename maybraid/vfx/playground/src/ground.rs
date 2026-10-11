use bevy::prelude::*;
use maybraid_vfx::{canonicalize_effect_name, SONIC_WAVE};

use crate::show::{mouth_height, LastVfxShow};

#[derive(Component)]
pub struct GroundPlane;

#[derive(Component)]
pub struct MouthEmitter;

pub fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let ground = meshes.add(Plane3d::default().mesh().size(240.0, 240.0));
	let dirt = materials.add(StandardMaterial {
		base_color: Color::srgb(0.28, 0.24, 0.18),
		perceptual_roughness: 0.96,
		..default()
	});
	commands.spawn((Mesh3d(ground), MeshMaterial3d(dirt), Transform::IDENTITY, GroundPlane));

	let band = meshes.add(Plane3d::default().mesh().size(240.0, 16.0));
	let cool = materials.add(StandardMaterial {
		base_color: Color::srgb(0.18, 0.22, 0.26),
		perceptual_roughness: 0.95,
		..default()
	});
	commands.spawn((
		Mesh3d(band),
		MeshMaterial3d(cool),
		Transform::from_xyz(0.0, 0.005, -8.0),
		GroundPlane,
	));

	let wall = meshes.add(Cuboid::new(0.22, 2.6, 3.4));
	let plaster = materials.add(StandardMaterial {
		base_color: Color::srgb(0.42, 0.36, 0.30),
		perceptual_roughness: 0.9,
		..default()
	});
	commands.spawn((
		Mesh3d(wall),
		MeshMaterial3d(plaster.clone()),
		Transform::from_xyz(-1.15, 1.3, 0.15),
		Name::new("vfx-occluder-wall"),
	));

	let crate_mesh = meshes.add(Cuboid::new(0.7, 0.7, 0.7));
	commands.spawn((
		Mesh3d(crate_mesh),
		MeshMaterial3d(plaster),
		Transform::from_xyz(1.35, 0.35, 0.8),
		Name::new("vfx-occluder-box"),
	));

	// 1.8 m stand-in just outside a 7 m God Beam so boss scale reads.
	let body = meshes.add(Capsule3d::new(0.28, 1.24));
	let figure = materials.add(StandardMaterial {
		base_color: Color::srgb(0.72, 0.28, 0.16),
		perceptual_roughness: 0.72,
		..default()
	});
	commands.spawn((
		Mesh3d(body),
		MeshMaterial3d(figure),
		Transform::from_xyz(6.4, 0.9, 4.2),
		Name::new("vfx-character-scale"),
	));
}

/// Placeholder head at mouth height so rings read as leaving a mouth.
pub fn sync_mouth_emitter(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	last: Res<LastVfxShow>,
	existing: Query<Entity, With<MouthEmitter>>,
) {
	if !last.is_changed() {
		return;
	}
	for entity in &existing {
		commands.entity(entity).despawn();
	}
	let Some(request) = last.request.as_ref() else {
		return;
	};
	if canonicalize_effect_name(&request.effect) != Some(SONIC_WAVE) {
		return;
	}
	let mouth = mouth_height(&request.drive);
	let boss = request.drive.size >= 16.0 || request.drive.radius >= 0.7;
	let (head_r, body_half, body_r) = if boss { (0.62, 1.35, 0.55) } else { (0.22, 0.62, 0.22) };
	let skin = materials.add(StandardMaterial {
		base_color: Color::srgb(0.78, 0.58, 0.46),
		perceptual_roughness: 0.7,
		..default()
	});
	let dark = materials.add(StandardMaterial {
		base_color: Color::srgb(0.18, 0.12, 0.10),
		perceptual_roughness: 0.85,
		..default()
	});
	let head = meshes.add(Sphere::new(head_r));
	let body = meshes.add(Capsule3d::new(body_r, body_half * 2.0));
	let mouth_mark = meshes.add(Sphere::new(head_r * 0.22));
	commands.spawn((
		Mesh3d(body),
		MeshMaterial3d(skin.clone()),
		Transform::from_xyz(0.0, body_half + body_r * 0.15, -head_r * 0.45),
		MouthEmitter,
		Name::new("vfx-mouth-body"),
	));
	commands.spawn((
		Mesh3d(head),
		MeshMaterial3d(skin),
		Transform::from_xyz(0.0, mouth + head_r * 0.15, -head_r * 0.55),
		MouthEmitter,
		Name::new("vfx-mouth-head"),
	));
	commands.spawn((
		Mesh3d(mouth_mark),
		MeshMaterial3d(dark),
		Transform::from_xyz(0.0, mouth, 0.02),
		MouthEmitter,
		Name::new("vfx-mouth-emitter"),
	));
}
