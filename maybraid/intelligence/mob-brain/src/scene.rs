//! Pad, lighting, and global waypoints the pack hosts journey between.

use avian3d::prelude::*;
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;
use poi_intelligence::{GlobalPoi, LocalPoi, Poi, PoiId, PoiKind};

pub const PAD_SIDE: f32 = 360.0;
pub const PAD_EXTENT: f32 = PAD_SIDE * 0.5;
pub const JOURNEY_TILE: f32 = 48.0;
pub const WAYPOINT: PoiKind = PoiKind::new("mob-brain/waypoint");
pub const CAMP: PoiKind = PoiKind::new("mob-brain/camp");
pub const GATE: PoiKind = PoiKind::new("mob-brain/gate");
pub const FORAGE: PoiKind = PoiKind::new("mob-brain/forage");
const LOCAL_POI_ID_START: u64 = 11;

pub fn waypoint_xz() -> [Vec2; 10] {
	[
		Vec2::new(-130.0, -110.0),
		Vec2::new(20.0, -140.0),
		Vec2::new(140.0, -80.0),
		Vec2::new(120.0, 40.0),
		Vec2::new(40.0, 140.0),
		Vec2::new(-90.0, 120.0),
		Vec2::new(-140.0, 20.0),
		Vec2::new(-40.0, -40.0),
		Vec2::new(70.0, -20.0),
		Vec2::new(-20.0, 70.0),
	]
}

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
		base_color: Color::srgb(0.1, 0.13, 0.12),
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

pub fn setup_waypoints(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let mesh = meshes.add(Sphere::new(1.4));
	let material = materials.add(Color::srgb(0.95, 0.78, 0.2));
	for (index, at) in waypoint_xz().into_iter().enumerate() {
		commands.spawn((
			Name::new(format!("waypoint-{index}")),
			Mesh3d(mesh.clone()),
			MeshMaterial3d(material.clone()),
			Transform::from_xyz(at.x, 1.2, at.y),
			Poi::new(PoiId(index as u64 + 1), WAYPOINT)
				.with_arrival_radius(5.0)
				.with_salience(1.0),
			GlobalPoi,
		));
	}
}

pub fn setup_local_pois(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	placements: impl IntoIterator<Item = (PoiKind, Vec2)>,
) {
	let mesh = meshes.add(Sphere::new(0.85));
	let camp = materials.add(Color::srgb(0.25, 0.78, 0.42));
	let gate = materials.add(Color::srgb(0.95, 0.72, 0.2));
	let forage = materials.add(Color::srgb(0.35, 0.62, 0.95));
	for (index, (kind, at)) in placements.into_iter().enumerate() {
		let material = match kind {
			CAMP => camp.clone(),
			GATE => gate.clone(),
			_ => forage.clone(),
		};
		commands.spawn((
			Name::new(format!("local-{index}")),
			Mesh3d(mesh.clone()),
			MeshMaterial3d(material),
			Transform::from_xyz(at.x, 0.9, at.y),
			Poi::new(PoiId(LOCAL_POI_ID_START + index as u64), kind)
				.with_arrival_radius(2.0)
				.with_salience(1.0),
			LocalPoi,
		));
	}
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
	use super::*;

	#[test]
	fn waypoints_occupy_several_journey_tiles() {
		let tiles: std::collections::HashSet<_> = waypoint_xz()
			.into_iter()
			.map(|at| (at / JOURNEY_TILE).floor().as_ivec2())
			.collect();
		assert!(tiles.len() >= 6, "got {tiles:?}");
		for at in waypoint_xz() {
			assert!(at.x.abs() < PAD_EXTENT - 8.0);
			assert!(at.y.abs() < PAD_EXTENT - 8.0);
		}
	}
}
