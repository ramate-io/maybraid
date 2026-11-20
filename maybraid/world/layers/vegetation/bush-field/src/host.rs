//! Thin adapter onto existing LOD levels. Does not add a second streamer.

use lod::scene::LodSceneLevel;

/// Coarse patch LOD owned by the existing lifetime/LOD host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BushPatchLod {
	#[default]
	High,
	Medium,
	Low,
	/// Far proxy only — unlike grass, UltraLow still keeps a crown silhouette.
	UltraLow,
}

impl BushPatchLod {
	pub fn from_scene_level(level: LodSceneLevel) -> Self {
		match level {
			LodSceneLevel::High => Self::High,
			LodSceneLevel::Medium => Self::Medium,
			LodSceneLevel::Low => Self::Low,
			LodSceneLevel::UltraLow => Self::UltraLow,
			LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => Self::Low,
		}
	}

	/// Multiplier on authored foliage density. Wood trunks stay until the whole bush culls.
	pub fn density_scale(self) -> f32 {
		match self {
			Self::High => 1.0,
			Self::Medium => 0.7,
			Self::Low => 0.35,
			Self::UltraLow => 0.12,
		}
	}
}
