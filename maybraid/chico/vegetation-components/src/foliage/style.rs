//! Foliage look / kit backend.

use lod::gen::LodSceneLevel;

use crate::assets::{foliage as foliage_assets, AssetPath};

/// Material / kit look for a foliage cluster.
///
/// SDF / inline builders remain as named styles until GLBs replace them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FoliageStyle {
	/// Procedural unit sphere (stand-in for noisy SDF balls).
	NoisyBall,
	/// Inline icosphere + plate shell (plane splay).
	PlaneSplay,
	/// GLB triad under `vegetation/foliage/standard/` (layered ball kit).
	Standard,
}

impl FoliageStyle {
	/// Layered-ball GLB for `self` at `level`, when this style is asset-backed.
	pub fn layered_ball_glb_for_level(self, level: LodSceneLevel) -> Option<AssetPath> {
		match self {
			Self::NoisyBall | Self::PlaneSplay => None,
			Self::Standard => Some(match level {
				LodSceneLevel::High => foliage_assets::standard::LAYERED_BALL_HIGH,
				LodSceneLevel::Medium => foliage_assets::standard::LAYERED_BALL_MID,
				LodSceneLevel::Low
				| LodSceneLevel::UltraLow
				| LodSceneLevel::Distance(_)
				| LodSceneLevel::Resolution(_) => foliage_assets::standard::LAYERED_BALL_LOW,
			}),
		}
	}
}
