//! Terrain presentation collider wiring.
//!
//! Mesh children are spawned asynchronously by [`render_item`]'s fetch path, so
//! Avian's [`ColliderConstructorHierarchy`] cannot be used directly (it removes
//! itself after one pass). Instead the scene marks roots with
//! [`TerrainTrimeshCollider`], and this module queues
//! [`ColliderConstructor::TrimeshFromMesh`] on mesh descendants when they appear.
//!
//! Constructed trimeshes use [`PhysicsInteractionLayer::Fixed`] so they contact
//! Animated movers only — not other Fixed geometry or LOD Host volumes.

use avian3d::prelude::{CoefficientCombine, Collider, ColliderConstructor, Friction};
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;

/// Dirt / grass grip. [`CoefficientCombine::Max`] beats the character controller's
/// `Friction::ZERO` + `Min` (Avian dynamic-character default), otherwise the
/// capsule ice-skates on the trimesh.
pub const TERRAIN_FRICTION: Friction = Friction {
	dynamic_coefficient: 0.75,
	static_coefficient: 0.95,
	combine_rule: CoefficientCombine::Max,
};

/// Marks a terrain presentation root whose mesh descendants should get trimesh colliders.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainTrimeshCollider;

/// Inserts [`ColliderConstructor::TrimeshFromMesh`] on mesh children of
/// [`TerrainTrimeshCollider`] roots that do not yet have a collider.
pub fn queue_terrain_trimesh_colliders(
	mut commands: Commands,
	roots: Query<Entity, With<TerrainTrimeshCollider>>,
	children: Query<&Children>,
	mesh_entities: Query<Entity, (With<Mesh3d>, Without<Collider>, Without<ColliderConstructor>)>,
) {
	for root in &roots {
		for descendant in children.iter_descendants(root) {
			if mesh_entities.get(descendant).is_ok() {
				commands.entity(descendant).insert((
					ColliderConstructor::TrimeshFromMesh,
					PhysicsInteractionLayer::fixed_layers(),
					TERRAIN_FRICTION,
				));
			}
		}
	}
}
