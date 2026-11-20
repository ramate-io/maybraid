//! Thin adapter onto existing LOD levels. Does not add a second streamer.

use lod::scene::LodSceneLevel;

/// Coarse patch LOD owned by the existing lifetime/LOD host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GrassPatchLod {
	#[default]
	High,
	Medium,
	Low,
	/// Hand off to the terrain turf mat; keep no GPU blades.
	UltraLow,
}

impl GrassPatchLod {
	pub fn from_scene_level(level: LodSceneLevel) -> Self {
		match level {
			LodSceneLevel::High => Self::High,
			LodSceneLevel::Medium => Self::Medium,
			LodSceneLevel::Low => Self::Low,
			LodSceneLevel::UltraLow => Self::UltraLow,
			LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => Self::Low,
		}
	}

	/// Multiplier on authored coverage. UltraLow is zero so terrain POM remains.
	pub fn density_scale(self) -> f32 {
		match self {
			Self::High => 1.0,
			Self::Medium => 0.5,
			Self::Low => 0.18,
			Self::UltraLow => 0.0,
		}
	}
}
