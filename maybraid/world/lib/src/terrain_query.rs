//! Terrain-agnostic query interface for world spawning.
//!
//! Provides terrain height sampling abstracted from Durham's specific terrain system,
//! allowing encounter and spawn systems to query terrain without direct durham dependency.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use durham::DurhamSurface;
use lod::hcsg::HcsgStorage;
use maybraid_spawns_support::terrain_height_at;

/// Terrain height query result for a world XZ position.
///
/// Returns published terrain/pad height when available, None when terrain
/// is unpublished or busy (caller should retry later).
pub fn query_terrain_height(
	surface: &DurhamSurface,
	storage: Option<&HcsgStorage>,
	xz: Vec2,
) -> Option<f32> {
	terrain_height_at(surface, storage, xz)
}

/// System parameter for terrain height queries.
///
/// Wraps Durham-specific terrain access in a spawn-friendly interface.
#[derive(SystemParam)]
pub struct TerrainHeightQuery<'w> {
	surface: DurhamSurface<'w>,
	storage: Option<Res<'w, HcsgStorage>>,
}

impl TerrainHeightQuery<'_> {
	/// Query published terrain height at XZ position.
	///
	/// Returns Some(height) when terrain/pad is published and ready,
	/// None when terrain is unavailable (unpublished or storage busy).
	pub fn height_at(&self, xz: Vec2) -> Option<f32> {
		query_terrain_height(&self.surface, self.storage.as_deref(), xz)
	}
}
