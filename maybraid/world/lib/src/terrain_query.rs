//! Terrain-agnostic query interface for world spawning.
//!
//! Provides terrain height sampling abstracted from Durham's specific terrain system,
//! allowing encounter and spawn systems to query terrain without direct durham dependency.

use bevy::prelude::*;

/// Published terrain height provider for spawn systems.
///
/// Returns **published terrain height** (terrain/pad with validated collider) only,
/// never the temporary hold elevation. The Durham/world integration installs this.
/// Tests can install it with fixed heights.
#[derive(Resource)]
pub struct PublishedTerrainHeight(pub Box<dyn Fn(Vec2) -> Option<f32> + Send + Sync>);

impl PublishedTerrainHeight {
	/// Query published terrain height at XZ position.
	pub fn height_at(&self, xz: Vec2) -> Option<f32> {
		(self.0)(xz)
	}
	
	/// Create a test provider that returns a fixed height for all positions.
	pub fn test_fixed(height: f32) -> Self {
		Self(Box::new(move |_| Some(height)))
	}
}

/// Query published terrain height using Durham (world-level integration only).
///
/// This function is called by world-level terrain integration to provide
/// Durham-backed terrain height queries. Boss spawn and encounter code do NOT
/// call this - they only query the PublishedTerrainHeight resource.
pub fn query_durham_terrain_height(
	surface: &durham::DurhamSurface,
	storage: Option<&lod::hcsg::HcsgStorage>,
	xz: Vec2,
) -> Option<f32> {
	maybraid_spawns_support::terrain_height_at(surface, storage, xz)
}
