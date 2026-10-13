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

/// Asset ids whose `MAIN_WORLD` usage was already dropped before first extract.
/// Re-inserting [`Mesh3d`] (heal) must not flip usage again — extract would
/// try to take an already-extracted mesh.
#[derive(Resource, Default)]
pub(crate) struct ReleasedCpuMeshes(std::collections::HashSet<bevy::asset::AssetId<Mesh>>);

/// Colliders cooked from a mesh before its CPU verts were released.
///
/// HandleMap can stamp the same `Handle<Mesh>` onto a later fill. After extract
/// *takes* the asset, `TrimeshFromMesh` panics on the compute pool. Replay the
/// cached collider instead of reading attributes again.
#[derive(Resource, Default)]
pub(crate) struct CookedTerrainColliders(
	std::collections::HashMap<bevy::asset::AssetId<Mesh>, Collider>,
);

fn insert_trimesh(commands: &mut Commands, entity: Entity, collider: Collider, friction: Friction) {
	commands.entity(entity).insert((
		TerrainTrimeshCollider,
		TerrainColliderReady,
		RigidBody::Static,
		collider,
		PhysicsInteractionLayer::fixed_layers(),
		friction,
	));
}

fn cook_trimesh(meshes: &Assets<Mesh>, handle: &Handle<Mesh>) -> Option<Collider> {
	Collider::trimesh_from_mesh(meshes.get(handle)?)
}

/// Cooks a trimesh onto each fill that asked for collision and now has [`Mesh3d`].
///
/// Fallback when `Last` missed the birth frame. Never reads a mesh whose
/// `MAIN_WORLD` usage was already dropped — use [`CookedTerrainColliders`].
pub(crate) fn queue_terrain_trimesh_colliders(
	mut commands: Commands,
	friction: Res<TerrainFrictionConfig>,
	meshes: Res<Assets<Mesh>>,
	released: Res<ReleasedCpuMeshes>,
	mut cooked: ResMut<CookedTerrainColliders>,
	sources: Query<
		(Entity, &Mesh3d),
		(With<TerrainColliderMeshSource>, Without<TerrainColliderReady>),
	>,
) {
	for (entity, mesh3d) in &sources {
		let id = mesh3d.0.id();
		if let Some(collider) = cooked.0.get(&id).cloned() {
			insert_trimesh(&mut commands, entity, collider, friction.0);
			continue;
		}
		if released.0.contains(&id) {
			continue;
		}
		if let Some(collider) = cook_trimesh(&meshes, &mesh3d.0) {
			cooked.0.insert(id, collider.clone());
			insert_trimesh(&mut commands, entity, collider, friction.0);
		}
	}
}

/// Cook (if needed) and drop `MAIN_WORLD` on the same frame [`Mesh3d`] appears.
///
/// `Last` runs before render extract. Flipping usage here lets extract *take*
/// the CpuShot buffers instead of cloning them for the rest of the session.
/// Collider fills keep a cached trimesh so a later shared handle can attach
/// without reading extracted attributes.
pub(crate) fn cook_and_release_cpu_before_extract(
	mut commands: Commands,
	friction: Res<TerrainFrictionConfig>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut released: ResMut<ReleasedCpuMeshes>,
	mut cooked: ResMut<CookedTerrainColliders>,
	added: Query<
		(Entity, &Mesh3d, Has<TerrainColliderMeshSource>, Has<TerrainColliderReady>),
		Added<Mesh3d>,
	>,
) {
	for (entity, mesh3d, is_source, ready) in &added {
		let id = mesh3d.0.id();
		let mut can_release = !is_source || ready;
		if is_source && !ready {
			if let Some(collider) = cooked.0.get(&id).cloned() {
				insert_trimesh(&mut commands, entity, collider, friction.0);
				can_release = true;
			} else if !released.0.contains(&id) {
				if let Some(collider) = cook_trimesh(&meshes, &mesh3d.0) {
					cooked.0.insert(id, collider.clone());
					insert_trimesh(&mut commands, entity, collider, friction.0);
					can_release = true;
				}
			}
		}
		if !can_release {
			continue;
		}
		if !released.0.insert(id) {
			continue;
		}
		if let Some(mut mesh) = meshes.get_mut(id) {
			if mesh.asset_usage.contains(bevy::asset::RenderAssetUsages::MAIN_WORLD) {
				mesh.asset_usage = bevy::asset::RenderAssetUsages::RENDER_WORLD;
			}
		}
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
			.init_resource::<ReleasedCpuMeshes>()
			.init_resource::<CookedTerrainColliders>()
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
	}

	fn both_worlds_triangle() -> Mesh {
		let mut mesh = Mesh::new(
			bevy::mesh::PrimitiveTopology::TriangleList,
			bevy::asset::RenderAssetUsages::MAIN_WORLD
				| bevy::asset::RenderAssetUsages::RENDER_WORLD,
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
		);
		mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2]));
		mesh
	}

	#[test]
	fn last_cooks_and_drops_main_world_before_extract() {
		let mut app = App::new();
		app.insert_resource(Assets::<Mesh>::default())
			.insert_resource(TerrainFrictionConfig::default())
			.init_resource::<ReleasedCpuMeshes>()
			.init_resource::<CookedTerrainColliders>()
			.add_systems(Last, cook_and_release_cpu_before_extract);

		let handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(both_worlds_triangle());
		let scene = app
			.world_mut()
			.spawn((TerrainColliderMeshSource, Mesh3d(handle.clone())))
			.id();
		app.update();

		assert!(app.world().get::<TerrainTrimeshCollider>(scene).is_some());
		let usage = app.world().resource::<Assets<Mesh>>().get(&handle).expect("mesh").asset_usage;
		assert_eq!(usage, bevy::asset::RenderAssetUsages::RENDER_WORLD);
	}

	#[test]
	fn last_drops_main_world_on_visual_only_mesh3d() {
		let mut app = App::new();
		app.insert_resource(Assets::<Mesh>::default())
			.insert_resource(TerrainFrictionConfig::default())
			.init_resource::<ReleasedCpuMeshes>()
			.init_resource::<CookedTerrainColliders>()
			.add_systems(Last, cook_and_release_cpu_before_extract);

		let handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(both_worlds_triangle());
		app.world_mut().spawn(Mesh3d(handle.clone()));
		app.update();

		let usage = app.world().resource::<Assets<Mesh>>().get(&handle).expect("mesh").asset_usage;
		assert_eq!(usage, bevy::asset::RenderAssetUsages::RENDER_WORLD);
	}

	#[test]
	fn heal_reinsert_skips_already_released_mesh() {
		let mut app = App::new();
		app.insert_resource(Assets::<Mesh>::default())
			.insert_resource(TerrainFrictionConfig::default())
			.init_resource::<ReleasedCpuMeshes>()
			.init_resource::<CookedTerrainColliders>()
			.add_systems(Last, cook_and_release_cpu_before_extract);

		let handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(both_worlds_triangle());
		let scene = app.world_mut().spawn(Mesh3d(handle.clone())).id();
		app.update();
		assert!(app.world().resource::<ReleasedCpuMeshes>().0.contains(&handle.id()));

		app.world_mut().entity_mut(scene).remove::<Mesh3d>();
		app.update();
		app.world_mut().entity_mut(scene).insert(Mesh3d(handle.clone()));
		app.update();

		let usage = app.world().resource::<Assets<Mesh>>().get(&handle).expect("mesh").asset_usage;
		assert_eq!(usage, bevy::asset::RenderAssetUsages::RENDER_WORLD);
		assert_eq!(app.world().resource::<ReleasedCpuMeshes>().0.len(), 1);
	}

	#[test]
	fn shared_handle_reuses_cached_collider_after_release() {
		let mut app = App::new();
		app.insert_resource(Assets::<Mesh>::default())
			.insert_resource(TerrainFrictionConfig::default())
			.init_resource::<ReleasedCpuMeshes>()
			.init_resource::<CookedTerrainColliders>()
			.add_systems(Last, cook_and_release_cpu_before_extract);

		let handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(both_worlds_triangle());
		let first = app
			.world_mut()
			.spawn((TerrainColliderMeshSource, Mesh3d(handle.clone())))
			.id();
		app.update();
		assert!(app.world().get::<TerrainTrimeshCollider>(first).is_some());
		assert_eq!(
			app.world().resource::<Assets<Mesh>>().get(&handle).expect("mesh").asset_usage,
			bevy::asset::RenderAssetUsages::RENDER_WORLD
		);

		let second = app
			.world_mut()
			.spawn((TerrainColliderMeshSource, Mesh3d(handle.clone())))
			.id();
		app.update();
		assert!(app.world().get::<TerrainTrimeshCollider>(second).is_some());
		assert!(app.world().resource::<CookedTerrainColliders>().0.contains_key(&handle.id()));
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
