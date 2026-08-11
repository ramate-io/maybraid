//! Helpers for building [`WorldAsset`]s from rebuilt geometry.

use bevy::asset::Handle;
use bevy::mesh::Mesh;
use bevy::prelude::{Mesh3d, MeshMaterial3d, StandardMaterial, Transform, Visibility, World};
use bevy::world_serialization::WorldAsset;

/// Build a single-entity [`WorldAsset`] owning `mesh_handle` + `material`.
///
/// A material is required for PBR extraction; callers (e.g. vegetation) may replace it
/// after spawn.
pub(crate) fn world_asset_from_mesh(
	mesh_handle: Handle<Mesh>,
	material: Handle<StandardMaterial>,
) -> WorldAsset {
	let mut world = World::new();
	world.spawn((
		Mesh3d(mesh_handle),
		MeshMaterial3d(material),
		Transform::IDENTITY,
		Visibility::default(),
	));
	WorldAsset::new(world)
}
