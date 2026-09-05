//! Terrain presentation collider wiring.
//!
//! Visual mesh children are spawned asynchronously and replaced as LOD changes.
//! Terrain collision must not follow that lifetime: Avian 0.7 can retain stale
//! contact-manifold indexes when a contacted trimesh is rebuilt or removed.
//!
//! High-detail level content is therefore only a [`TerrainColliderMeshSource`].
//! Once its mesh asset is ready, this module creates a direct [`Collider`] as a
//! child of the persistent [`TerrainColliderHost`]. Visual level roots may then
//! swap without touching the physics entity.
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

/// Persistent terrain [`lod::LodSceneHost`] that owns collision independently
/// from its visual level roots.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainColliderHost;

/// High-detail visual content whose generated mesh can seed stable collision.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainColliderMeshSource;

/// Direct, stable terrain collider spawned outside the visual LOD roots.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainTrimeshCollider;

#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct TerrainColliderReady;

/// Builds one direct trimesh under each persistent [`TerrainColliderHost`].
///
/// The mesh source has the same host-local transform as the collider. Its
/// `Mesh3d` child is generated asynchronously with an identity transform.
pub(crate) fn queue_terrain_trimesh_colliders(
	mut commands: Commands,
	friction: Res<TerrainFrictionConfig>,
	meshes: Res<Assets<Mesh>>,
	hosts: Query<Entity, (With<TerrainColliderHost>, Without<TerrainColliderReady>)>,
	children: Query<&Children>,
	sources: Query<(&Transform, &CascadeChunk), With<TerrainColliderMeshSource>>,
	mesh_entities: Query<&Mesh3d>,
) {
	for host in &hosts {
		let Some((source_transform, chunk, mesh)) =
			children.iter_descendants(host).find_map(|candidate| {
				let (transform, chunk) = sources.get(candidate).ok()?;
				let mesh = children
					.iter_descendants(candidate)
					.find_map(|descendant| mesh_entities.get(descendant).ok())?;
				Some((transform, chunk, mesh))
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
		commands.spawn((
			Name::new("Stable terrain collider"),
			TerrainTrimeshCollider,
			ChildOf(host),
			source_transform.clone(),
			chunk.clone(),
			RigidBody::Static,
			collider,
			PhysicsInteractionLayer::fixed_layers(),
			friction.0,
		));
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
		let source_transform = Transform::from_xyz(2.0, 3.0, 4.0);
		let source = app
			.world_mut()
			.spawn((
				TerrainColliderMeshSource,
				source_transform,
				CascadeChunk::default(),
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
		assert_eq!(*stable[0].1, source_transform);

		app.world_mut().entity_mut(source).despawn();
		app.update();

		let mut colliders =
			app.world_mut().query_filtered::<Entity, With<TerrainTrimeshCollider>>();
		assert_eq!(colliders.iter(app.world()).count(), 1);
	}
}
