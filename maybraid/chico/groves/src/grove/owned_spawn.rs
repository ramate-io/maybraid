//! Deferred mesh/material insertion for grove instances with owned materials.

use bevy::prelude::*;
use chico_ball_components::frond::{FrondCrown, FrondCrownShape};
use render_item::CascadeChunk;

use crate::skipped_mesh_material::SkippedLeafMeshMaterial;

/// Spawn `marker` with `mesh` and an owned `material` asset at `transform`.
pub fn spawn_owned_mesh<M, Marker>(
	marker: Marker,
	mesh: Mesh,
	material: M,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	transform: Transform,
) -> Vec<Entity>
where
	M: Material + Clone + Send + Sync + 'static,
	Marker: Component + Clone + Send + Sync + 'static,
{
	let root = commands
		.spawn((marker, cascade_chunk.clone(), transform, Visibility::default()))
		.id();
	commands.queue(move |world: &mut World| {
		let mesh_handle = world.resource_mut::<Assets<Mesh>>().add(mesh);
		let material_handle = world.resource_mut::<Assets<M>>().add(material);
		world
			.entity_mut(root)
			.insert((Mesh3d(mesh_handle), MeshMaterial3d(material_handle)));
	});
	vec![root]
}

/// Spawn one [`FrondCrown`] mesh with a per-placement owned material.
pub fn spawn_owned_frond<LeafM>(
	shape: FrondCrownShape,
	material: LeafM,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	transform: Transform,
) -> Vec<Entity>
where
	LeafM: Material + Clone + Send + Sync + 'static,
{
	let mesh = FrondCrown::<LeafM, SkippedLeafMeshMaterial<LeafM>>::from_shape(
		shape,
		SkippedLeafMeshMaterial::default(),
	)
	.build_mesh(1.0);
	spawn_owned_mesh(PlacedFrond, mesh, material, commands, cascade_chunk, transform)
}

#[derive(Component, Clone, Copy)]
struct PlacedFrond;
