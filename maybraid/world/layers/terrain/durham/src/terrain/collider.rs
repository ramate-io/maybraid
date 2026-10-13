//! Terrain presentation collider wiring.
//!
//! Near (and FinePatch) fill scenes mark [`TerrainColliderMeshSource`]. After
//! Cached fulfill puts [`Mesh3d`] on that same entity, this module cooks an
//! Avian trimesh onto it. Far / background cells omit the marker.
//!
//! Constructed trimeshes use [`PhysicsInteractionLayer::Fixed`] so they contact
//! Animated movers only — not other Fixed geometry or LOD Host volumes.

use avian3d::prelude::{CoefficientCombine, Collider, Friction, RigidBody};
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use lod_avian::PhysicsInteractionLayer;

/// Dirt / grass grip. [`CoefficientCombine::Max`] beats the character controller's
/// `Friction::ZERO` + `Min` (Avian dynamic-character default), otherwise the
/// capsule ice-skates on the trimesh.
///
/// Playgrounds override via [`TerrainFrictionConfig`] (inserted before Durham
/// generation); [`queue_terrain_trimesh_colliders`] reads
/// that resource, not this constant, when both exist.
pub const TERRAIN_FRICTION: Friction = Friction {
	dynamic_coefficient: 0.75,
	static_coefficient: 0.95,
	combine_rule: CoefficientCombine::Max,
};

/// Friction applied to new terrain trimeshes. Defaults to [`TERRAIN_FRICTION`].
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct TerrainFrictionConfig(pub Friction);

impl Default for TerrainFrictionConfig {
	fn default() -> Self {
		Self(TERRAIN_FRICTION)
	}
}

/// Presenters run before [`Self::QueueMeshes`] so a new fill can bake the same frame.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainColliderSystems {
	SyncOverlays,
	SyncHosts,
	QueueMeshes,
	/// Drop MAIN_WORLD vertex copies once Avian has the trimesh (or never needed it).
	ReleaseCpu,
}

/// Fill scene that should receive a trimesh once [`Mesh3d`] is fulfilled.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainColliderMeshSource;

/// Cooked Avian trimesh on a Near / FinePatch fill scene.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainTrimeshCollider;

/// True when any collider column covers `point` on XZ (Y is ignored).
pub fn terrain_collider_covers_xz<'a>(
	point: Vec3,
	chunks: impl IntoIterator<Item = &'a CascadeChunk>,
) -> bool {
	chunks.into_iter().any(|chunk| chunk.column_contains_point(point))
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct TerrainColliderReady;

/// Cooks a trimesh onto each fill that asked for collision and now has [`Mesh3d`].
pub(crate) fn queue_terrain_trimesh_colliders(
	mut commands: Commands,
	friction: Res<TerrainFrictionConfig>,
	mut meshes: ResMut<Assets<Mesh>>,
	sources: Query<
		(Entity, &Mesh3d),
		(With<TerrainColliderMeshSource>, Without<TerrainColliderReady>),
	>,
) {
	for (entity, mesh3d) in &sources {
		let Some(mesh) = meshes.get(&mesh3d.0) else {
			continue;
		};
		let Some(collider) = Collider::trimesh_from_mesh(mesh) else {
			continue;
		};
		if let Some(mut mesh) = meshes.get_mut(&mesh3d.0) {
			release_cpu_mesh_after_extract(&mut mesh);
		}
		commands.entity(entity).insert((
			TerrainTrimeshCollider,
			TerrainColliderReady,
			RigidBody::Static,
			collider,
			PhysicsInteractionLayer::fixed_layers(),
			friction.0,
		));
	}
}

/// Far / overlay CpuShot meshes never cook a trimesh, but they still start
/// with `MAIN_WORLD` so a shared fill handle can bake first. Once no fill is
/// waiting on that asset, drop the CPU copy so extract can free it.
pub(crate) fn release_cpu_on_visual_only_meshes(
	mut meshes: ResMut<Assets<Mesh>>,
	pending: Query<&Mesh3d, (With<TerrainColliderMeshSource>, Without<TerrainColliderReady>)>,
	live: Query<&Mesh3d>,
) {
	let pending: std::collections::HashSet<_> = pending.iter().map(|mesh| mesh.0.id()).collect();
	let mut seen = std::collections::HashSet::new();
	for mesh3d in &live {
		let id = mesh3d.0.id();
		if !seen.insert(id) || pending.contains(&id) {
			continue;
		}
		if let Some(mut mesh) = meshes.get_mut(id) {
			release_cpu_mesh_after_extract(&mut mesh);
		}
	}
}

fn release_cpu_mesh_after_extract(mesh: &mut Mesh) {
	if mesh.asset_usage.contains(bevy::asset::RenderAssetUsages::MAIN_WORLD) {
		mesh.asset_usage = bevy::asset::RenderAssetUsages::RENDER_WORLD;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_terrain_friction_config_matches_const() {
		assert_eq!(TerrainFrictionConfig::default().0, TERRAIN_FRICTION);
	}

	#[test]
	fn distant_collider_does_not_cover_spawn_column() {
		let spawn = Vec3::ZERO;
		let local = CascadeChunk::unit_chunk();
		let distant = CascadeChunk {
			origin: Vec3::new(1_000.0, -2_000.0, 1_000.0),
			size: 160.0,
			..CascadeChunk::unit_chunk()
		};
		assert!(terrain_collider_covers_xz(spawn, [&local]));
		assert!(!terrain_collider_covers_xz(spawn, [&distant]));
		assert!(terrain_collider_covers_xz(spawn, [&distant, &local]));
	}

	#[test]
	fn trimesh_bakes_onto_the_fill_scene() {
		let mut app = App::new();
		app.insert_resource(Assets::<Mesh>::default())
			.insert_resource(TerrainFrictionConfig::default())
			.add_systems(Update, queue_terrain_trimesh_colliders);

		let mut mesh = Mesh::new(
			bevy::mesh::PrimitiveTopology::TriangleList,
			bevy::asset::RenderAssetUsages::MAIN_WORLD,
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
		);
		mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2]));
		let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);

		let pose = Transform::from_xyz(2.0, 3.0, 4.0);
		let scene = app
			.world_mut()
			.spawn((TerrainColliderMeshSource, Mesh3d(mesh.clone()), pose, CascadeChunk::default()))
			.id();

		app.update();

		assert!(app.world().get::<TerrainTrimeshCollider>(scene).is_some());
		assert!(app.world().get::<Collider>(scene).is_some());
		assert_eq!(app.world().get::<Transform>(scene).copied(), Some(pose));

		app.update();
		assert!(app.world().get::<TerrainTrimeshCollider>(scene).is_some());
		let usage = app.world().resource::<Assets<Mesh>>().get(&mesh).expect("mesh").asset_usage;
		assert_eq!(
			usage,
			bevy::asset::RenderAssetUsages::RENDER_WORLD,
			"CPU verts should drop after the trimesh bake"
		);
	}

	#[test]
	fn visual_only_mesh_drops_main_world_when_no_cook_is_pending() {
		let mut app = App::new();
		app.insert_resource(Assets::<Mesh>::default())
			.add_systems(Update, release_cpu_on_visual_only_meshes);

		let mut mesh = Mesh::new(
			bevy::mesh::PrimitiveTopology::TriangleList,
			bevy::asset::RenderAssetUsages::MAIN_WORLD
				| bevy::asset::RenderAssetUsages::RENDER_WORLD,
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
		);
		let handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
		app.world_mut().spawn(Mesh3d(handle.clone()));
		app.update();

		let usage = app.world().resource::<Assets<Mesh>>().get(&handle).expect("mesh").asset_usage;
		assert_eq!(usage, bevy::asset::RenderAssetUsages::RENDER_WORLD);
	}

	#[test]
	fn pending_collider_keeps_main_world_until_cook() {
		let mut app = App::new();
		app.insert_resource(Assets::<Mesh>::default())
			.add_systems(Update, release_cpu_on_visual_only_meshes);

		let mut mesh = Mesh::new(
			bevy::mesh::PrimitiveTopology::TriangleList,
			bevy::asset::RenderAssetUsages::MAIN_WORLD
				| bevy::asset::RenderAssetUsages::RENDER_WORLD,
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
		);
		let handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
		app.world_mut().spawn((TerrainColliderMeshSource, Mesh3d(handle.clone())));
		app.update();

		let usage = app.world().resource::<Assets<Mesh>>().get(&handle).expect("mesh").asset_usage;
		assert!(usage.contains(bevy::asset::RenderAssetUsages::MAIN_WORLD));
	}

	#[test]
	fn despawn_of_the_fill_scene_removes_its_collider() {
		let mut app = App::new();
		let scene = app
			.world_mut()
			.spawn((TerrainTrimeshCollider, TerrainColliderMeshSource, CascadeChunk::unit_chunk()))
			.id();

		app.world_mut().entity_mut(scene).despawn();

		let mut colliders =
			app.world_mut().query_filtered::<Entity, With<TerrainTrimeshCollider>>();
		assert_eq!(colliders.iter(app.world()).count(), 0);
	}
}
