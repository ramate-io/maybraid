//! Terrain-agnostic query interface for world spawning.
//!
//! Provides terrain height sampling abstracted from Durham's specific terrain system,
//! allowing encounter and spawn systems to query terrain without direct durham dependency.

use bevy::prelude::*;

/// Test-friendly terrain height provider for published terrain and pads.
///
/// Production installs a Durham-backed implementation. Tests can install a simple
/// closure returning fixed heights. Returns **published terrain height** only
/// (never the temporary hold elevation).
///
/// The spawn system queries this optionally; if missing, bosses stay in `PendingBossSpawn`.
#[derive(Resource)]
pub struct PublishedTerrainHeight(pub Box<dyn Fn(Vec2) -> Option<f32> + Send + Sync>);

impl PublishedTerrainHeight {
	/// Query published terrain height at XZ position.
	pub fn height_at(&self, xz: Vec2) -> Option<f32> {
		(self.0)(xz)
	}
	
	/// Create a test provider that returns a fixed height for all positions.
	#[cfg(any(test, feature = "test-support"))]
	pub fn test_fixed(height: f32) -> Self {
		Self(Box::new(move |_| Some(height)))
	}
}
