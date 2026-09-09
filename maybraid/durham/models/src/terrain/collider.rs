//! Terrain presentation collider wiring.
//!
//! Visual mesh children are spawned asynchronously and replaced as LOD changes.
//! Terrain collision must not follow that lifetime: Avian can retain stale
//! contact-manifold indexes when a contacted trimesh is rebuilt or removed.
//!
//! Collision is keyed by terrain origin [`Id`] on a persistent
//! [`TerrainColliderHost`] that presentation does not own. [`TerrainColliderMeshSource`]
//! scenes seed a copied [`Collider`] child; the mesh source may then despawn.
//! Visual `RegionPresenter` roots may churn without touching physics.
//! [`TerrainColliderOverlay`] hosts (padded terrain) replace the raw Durham
//! seed for the same origin id.
//!
//! Constructed trimeshes use [`PhysicsInteractionLayer::Fixed`] so they contact
//! Animated movers only — not other Fixed geometry or LOD Host volumes.

use crate::terrain::cell::TerrainCellLayout;
use crate::terrain::host::TerrainPresentEnabled;
use crate::terrain::index::TerrainEntryStore;
use avian3d::prelude::{CoefficientCombine, Collider, Friction, RigidBody};
use bevy::math::bounding::IntersectsVolume;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use lod::gen::{Id, Version};
use lod_avian::PhysicsInteractionLayer;
use std::collections::HashSet;

/// Dirt / grass grip. [`CoefficientCombine::Max`] beats the character controller's
/// `Friction::ZERO` + `Min` (Avian dynamic-character default), otherwise the
/// capsule ice-skates on the trimesh.
///
/// Playgrounds override via [`TerrainFrictionConfig`] (inserted before
/// [`crate::terrain::TerrainPlugin`]); [`queue_terrain_trimesh_colliders`] reads
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

/// Bumped when semantic terrain is regenerated so collider hosts cannot reuse
/// a recycled [`Version`] after [`crate::terrain::AvianTerrainIndex::clear`].
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainColliderEpoch(pub u64);

/// Hosts run: overlay replace, then raw sync, then trimesh bake.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainColliderSystems {
	SyncOverlays,
	SyncHosts,
	QueueMeshes,
}

/// Persistent terrain host that owns collision independently from visual meshes.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainColliderHost;

/// Marks a [`TerrainColliderHost`] seeded from an overlay model (padded terrain).
///
/// Raw Durham sync will not despawn or duplicate these hosts.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainColliderOverlay;

/// Origin cell and store version this physics host was seeded from.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainColliderCell {
	pub id: Id,
	pub version: Version,
	pub epoch: u64,
}

/// Raw Durham mesh whose generated asset can seed stable collision.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainColliderMeshSource;

/// Direct, stable terrain collider spawned outside the visual LOD roots.
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

fn terrain_seeds_collision(terrain: &crate::terrain::Terrain, layout: &TerrainCellLayout) -> bool {
	let size = (Vec3::from(terrain.cell.max) - Vec3::from(terrain.cell.min)).x;
	layout
		.stream_ring_for_cell_size(size)
		.map(|ring| ring.seeds_collision())
		.unwrap_or(true)
}

/// Spawn a hidden collider-host and seed it with `scene` (`TerrainColliderMeshSource`).
pub fn spawn_terrain_collider_host(
	commands: &mut Commands,
	id: Id,
	version: Version,
	epoch: u64,
	scene: impl bevy::scene::prelude::Scene + 'static,
	overlay: bool,
) -> Entity {
	let host = if overlay {
		commands
			.spawn((
				Name::new("Terrain collider"),
				TerrainColliderHost,
				TerrainColliderOverlay,
				TerrainColliderCell { id, version, epoch },
				Transform::IDENTITY,
				Visibility::Hidden,
			))
			.id()
	} else {
		commands
			.spawn((
				Name::new("Terrain collider"),
				TerrainColliderHost,
				TerrainColliderCell { id, version, epoch },
				Transform::IDENTITY,
				Visibility::Hidden,
			))
			.id()
	};
	commands.spawn_scene(scene).insert(ChildOf(host));
	host
}

/// Spawn or refresh physics hosts from stored Durham cells. Visual presenters
/// are not consulted and must not despawn these entities.
pub(crate) fn sync_terrain_collider_hosts(
	mut commands: Commands,
	epoch: Res<TerrainColliderEpoch>,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	present: Option<Res<TerrainPresentEnabled>>,
	hosts: Query<
		(Entity, &TerrainColliderCell, Has<TerrainColliderOverlay>),
		With<TerrainColliderHost>,
	>,
) {
	let present = present.map(|flag| flag.0).unwrap_or(true);
	let region = layout.presentation_region();
	let overlay_ids: HashSet<Id> = hosts
		.iter()
		.filter(|(_, _, overlay)| *overlay)
		.map(|(_, cell, _)| cell.id)
		.collect();
	// Playable world presents urbanized terrain only; raw `Terrain::scene`
	// must not seed a first collider mesh that the overlay later rebakes.
	let wanted: HashSet<(Id, Version)> = if present {
		store
			.terrain
			.iter()
			.filter(|(id, entry)| {
				!overlay_ids.contains(id)
					&& region.intersects(&entry.bounds)
					&& terrain_seeds_collision(&entry.value, layout.as_ref())
			})
			.map(|(id, entry)| (*id, entry.version))
			.collect()
	} else {
		HashSet::new()
	};

	for (entity, cell, overlay) in &hosts {
		if overlay {
			continue;
		}
		let current = wanted.contains(&(cell.id, cell.version)) && cell.epoch == epoch.0;
		if !current {
			commands.entity(entity).despawn();
		}
	}

	let occupied: HashSet<Id> = hosts.iter().map(|(_, cell, _)| cell.id).collect();
	for (id, version) in wanted {
		if occupied.contains(&id) {
			continue;
		}
		let Some(entry) = store.terrain.get(&id) else {
			continue;
		};
		spawn_terrain_collider_host(
			&mut commands,
			id,
			version,
			epoch.0,
			entry.value.scene(),
			false,
		);
	}
}

/// Builds one direct trimesh under each persistent [`TerrainColliderHost`].
///
/// Pose comes from [`CascadeChunk::origin`], not the mesh-source `Transform`.
/// CpuShot verts are local; an identity source (ChildOf required-component
/// default) must not bake a collider at the world origin. The `Mesh3d` child
/// is generated asynchronously with an identity transform. After the trimesh
/// is copied into Avian, the mesh source is despawned so visual LOD can own
/// drawing.
pub(crate) fn queue_terrain_trimesh_colliders(
	mut commands: Commands,
	friction: Res<TerrainFrictionConfig>,
	meshes: Res<Assets<Mesh>>,
	hosts: Query<Entity, (With<TerrainColliderHost>, Without<TerrainColliderReady>)>,
	children: Query<&Children>,
	sources: Query<(Entity, &CascadeChunk), With<TerrainColliderMeshSource>>,
	mesh_entities: Query<&Mesh3d>,
) {
	for host in &hosts {
		let Some((source_entity, chunk, mesh)) =
			children.iter_descendants(host).find_map(|candidate| {
				let (entity, chunk) = sources.get(candidate).ok()?;
				let mesh = children
					.iter_descendants(candidate)
					.find_map(|descendant| mesh_entities.get(descendant).ok())?;
				Some((entity, chunk, mesh))
			})
		else {
			continue;
		};
		let Some(mesh) = meshes.get(&mesh.0) else {
			continue;
		};
		let Some(collider) = Collider::trimesh_from_mesh(mesh) else {
			continue;
		};
		let pose = Transform::from_translation(chunk.origin);
		commands.spawn((
			Name::new("Stable terrain collider"),
			TerrainTrimeshCollider,
			ChildOf(host),
			pose,
			chunk.clone(),
			RigidBody::Static,
			collider,
			PhysicsInteractionLayer::fixed_layers(),
			friction.0,
		));
		commands.entity(source_entity).despawn();
		if let Ok(mut entity) = commands.get_entity(host) {
			entity.insert(TerrainColliderReady);
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
	fn collider_survives_visual_source_despawn() {
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

		let host = app.world_mut().spawn((TerrainColliderHost, Transform::default())).id();
		let origin = Vec3::new(2.0, 3.0, 4.0);
		let source = app
			.world_mut()
			.spawn((
				TerrainColliderMeshSource,
				Transform::IDENTITY,
				CascadeChunk { origin, ..CascadeChunk::default() },
				ChildOf(host),
			))
			.id();
		app.world_mut().spawn((Mesh3d(mesh), Transform::default(), ChildOf(source)));

		app.update();

		let mut colliders = app
			.world_mut()
			.query_filtered::<(&ChildOf, &Transform), With<TerrainTrimeshCollider>>();
		let stable: Vec<_> = colliders.iter(app.world()).collect();
		assert_eq!(stable.len(), 1);
		assert_eq!(stable[0].0.parent(), host);
		assert_eq!(*stable[0].1, Transform::from_translation(origin));
		assert!(app.world().get_entity(source).is_err());

		app.update();

		let mut colliders =
			app.world_mut().query_filtered::<Entity, With<TerrainTrimeshCollider>>();
		assert_eq!(colliders.iter(app.world()).count(), 1);
		assert!(app.world().get_entity(host).is_ok());
	}

	#[test]
	fn visual_root_despawn_leaves_collider_host() {
		let mut app = App::new();
		let host = app.world_mut().spawn((TerrainColliderHost, Transform::default())).id();
		app.world_mut()
			.spawn((TerrainTrimeshCollider, CascadeChunk::unit_chunk(), ChildOf(host)));
		let visual = app.world_mut().spawn(Name::new("Terrain cell")).id();

		app.world_mut().entity_mut(visual).despawn();

		assert!(app.world().get_entity(host).is_ok());
		let mut colliders =
			app.world_mut().query_filtered::<Entity, With<TerrainTrimeshCollider>>();
		assert_eq!(colliders.iter(app.world()).count(), 1);
	}

	#[test]
	fn recycled_store_version_is_not_current_across_epochs() {
		assert_ne!(TerrainColliderEpoch(0), TerrainColliderEpoch(1));
	}

	#[test]
	fn overlay_host_is_not_despawned_by_raw_sync() {
		let mut app = App::new();
		app.insert_resource(TerrainEntryStore::default())
			.insert_resource(TerrainCellLayout::default())
			.insert_resource(TerrainColliderEpoch::default())
			.add_systems(Update, sync_terrain_collider_hosts);

		let id = Id::from_cell(bevy::math::bounding::Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
		let overlay = app
			.world_mut()
			.spawn((
				TerrainColliderHost,
				TerrainColliderOverlay,
				TerrainColliderCell { id, version: Version(1), epoch: 0 },
			))
			.id();

		app.update();

		assert!(app.world().get_entity(overlay).is_ok());
	}
}
