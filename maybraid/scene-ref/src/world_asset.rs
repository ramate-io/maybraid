//! Helpers for building [`WorldAsset`]s from rebuilt geometry.

use bevy::asset::Handle;
use bevy::mesh::Mesh;
use bevy::prelude::{Mesh3d, Transform, World};
use bevy::world_serialization::WorldAsset;

/// Build a single-entity [`WorldAsset`] owning `mesh_handle`.
pub(crate) fn world_asset_from_mesh(mesh_handle: Handle<Mesh>) -> WorldAsset {
	let mut world = World::new();
	world.spawn((Mesh3d(mesh_handle), Transform::IDENTITY));
	WorldAsset::new(world)
}
