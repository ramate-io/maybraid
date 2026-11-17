//! Terrain-agnostic query interface for world spawning.
//!
//! Provides terrain height sampling abstracted from Durham's specific terrain system,
//! allowing encounter and spawn systems to query terrain without direct durham dependency.

use bevy::prelude::*;
use durham::DurhamSurface;
use lod::hcsg::HcsgStorage;
use maybraid_spawns_support::terrain_height_at;

/// Query published terrain height at XZ position using Durham terrain.
///
/// Returns **published terrain height** (terrain/pad with validated collider) when available.
/// Returns `None` when terrain is unpublished, storage is busy, or resources are missing.
/// Never returns the temporary hold elevation.
///
/// This function is used by spawn systems through optional resource queries,
/// allowing tests to omit Durham resources without panicking.
pub fn query_published_terrain_height(
	surface: Option<&DurhamSurface>,
	storage: Option<&HcsgStorage>,
	xz: Vec2,
) -> Option<f32> {
	let surface = surface?;
	terrain_height_at(surface, storage, xz)
}
