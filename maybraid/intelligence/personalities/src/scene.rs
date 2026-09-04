//! Flat 400 m pad, sparse cover, and POIs the proto-mobs meander toward.

use avian3d::prelude::*;
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;
use poi_intelligence::{LocalPoi, Poi, PoiId, PoiKind};

pub const PAD_SIDE: f32 = 400.0;
pub const PAD_EXTENT: f32 = PAD_SIDE * 0.5;
pub const SPOTTING_RING: f32 = 80.0;
pub const HIGH_RING: f32 = 200.0;

pub const CAMP: PoiKind = PoiKind::new("personalities/camp");
pub const GATE: PoiKind = PoiKind::new("personalities/gate");
pub const FORAGE: PoiKind = PoiKind::new("personalities/forage");
pub const PIT: PoiKind = PoiKind::new("personalities/pit");

pub fn setup_lighting(mut commands: Commands) {
	commands.insert_resource(GlobalAmbientLight { brightness: 520.0, ..default() });
	commands.spawn((
		DirectionalLight { illuminance: 14_000.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.6, 0.0)),
	));
}

pub fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let mesh = meshes.add(Cuboid::new(PAD_SIDE, 0.2, PAD_SIDE));
	let material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.11, 0.14, 0.12),
		perceptual_roughness: 0.94,
		..default()
	});
	commands.spawn((
		Name::new("pad"),
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::from_xyz(0.0, -0.1, 0.0),
		RigidBody::Static,
		Collider::cuboid(PAD_SIDE, 0.2, PAD_SIDE),
		PhysicsInteractionLayer::fixed_layers(),
	));
}

pub fn setup_cover(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let mesh = meshes.add(Cuboid::new(3.2, 2.4, 1.6));
	let material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.28, 0.26, 0.22),
		perceptual_roughness: 0.9,
		..default()
	});
	let poses = [
		Vec3::new(-48.0, 1.2, 6.0),
		Vec3::new(-62.0, 1.2, 18.0),
		Vec3::new(-40.0, 1.2, -8.0),
		Vec3::new(28.0, 1.2, 14.0),
		Vec3::new(38.0, 1.2, -2.0),
		Vec3::new(16.0, 1.2, 148.0),
		Vec3::new(28.0, 1.2, 158.0),
		Vec3::new(-4.0, 1.2, 138.0),
		Vec3::new(112.0, 1.2, -36.0),
		Vec3::new(128.0, 1.2, -48.0),
		Vec3::new(-86.0, 1.2, -82.0),
		Vec3::new(74.0, 1.2, 28.0),
	];
	for (index, at) in poses.into_iter().enumerate() {
		commands.spawn((
			Name::new(format!("cover-{index}")),
			Mesh3d(mesh.clone()),
			MeshMaterial3d(material.clone()),
			Transform::from_translation(at),
			RigidBody::Static,
			Collider::cuboid(3.2, 2.4, 1.6),
			PhysicsInteractionLayer::fixed_layers(),
		));
	}
}

pub fn setup_pois(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let local = meshes.add(Sphere::new(1.1));
	let camp_mat = materials.add(Color::srgb(0.25, 0.78, 0.42));
	let gate_mat = materials.add(Color::srgb(0.95, 0.72, 0.2));
	let forage_mat = materials.add(Color::srgb(0.35, 0.62, 0.95));
	let pit_mat = materials.add(Color::srgb(0.92, 0.28, 0.32));

	let mut next_id = 1_u64;
	let mut spawn = |kind: PoiKind, at: Vec2, material: Handle<StandardMaterial>| {
		let id = PoiId(next_id);
		next_id += 1;
		commands.spawn((
			Mesh3d(local.clone()),
			MeshMaterial3d(material),
			Transform::from_xyz(at.x, 1.2, at.y),
			Poi::new(id, kind).with_arrival_radius(3.5).with_salience(1.0),
			LocalPoi,
		));
	};

	spawn(CAMP, Vec2::new(-55.0, 10.0), camp_mat.clone());
	spawn(CAMP, Vec2::new(-68.0, 4.0), camp_mat.clone());
	spawn(CAMP, Vec2::new(-46.0, 22.0), camp_mat.clone());
	spawn(CAMP, Vec2::new(-50.0, -6.0), camp_mat);

	spawn(GATE, Vec2::new(80.0, 20.0), gate_mat);

	spawn(PIT, Vec2::new(120.0, -40.0), pit_mat.clone());
	spawn(PIT, Vec2::new(112.0, -48.0), pit_mat);

	spawn(FORAGE, Vec2::new(-90.0, -90.0), forage_mat.clone());
	spawn(FORAGE, Vec2::new(-104.0, -78.0), forage_mat.clone());
	spawn(FORAGE, Vec2::new(-76.0, -102.0), forage_mat.clone());
	spawn(FORAGE, Vec2::new(-14.0, 18.0), forage_mat.clone());
	spawn(FORAGE, Vec2::new(-22.0, 8.0), forage_mat.clone());
	spawn(FORAGE, Vec2::new(-6.0, 26.0), forage_mat.clone());
	spawn(FORAGE, Vec2::new(162.0, 170.0), forage_mat.clone());
	spawn(FORAGE, Vec2::new(178.0, 162.0), forage_mat.clone());
	for x in [-100.0_f32, 0.0, 100.0] {
		for z in [-100.0_f32, 0.0, 100.0] {
			if x.abs() < 1.0 && z.abs() < 1.0 {
				continue;
			}
			spawn(FORAGE, Vec2::new(x, z), forage_mat.clone());
		}
	}
}
