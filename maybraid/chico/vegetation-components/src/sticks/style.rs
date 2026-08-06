//! Stick look / kit backend.

use lod::gen::LodSceneLevel;

use crate::assets::{sticks as stick_assets, AssetPath};

/// Material / kit look for a stick segment.
///
/// SDF / procedural cylinders remain as named styles until GLBs replace them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StickStyle {
	/// Procedural unit cylinder (stand-in for noisy SDF sticks).
	NoisyCylinder,
	/// GLB triad under `vegetation/sticks/standard/`.
	Standard,
	/// GLB triad under `vegetation/sticks/standard_trunk/`.
	StandardTrunk,
}

impl StickStyle {
	pub fn glb_for_level(self, level: LodSceneLevel) -> Option<AssetPath> {
		match self {
			Self::NoisyCylinder => None,
			Self::Standard => match level {
				LodSceneLevel::High => Some(stick_assets::standard::HIGH),
				LodSceneLevel::Medium => Some(stick_assets::standard::MID),
				LodSceneLevel::Low
				| LodSceneLevel::Distance(_)
				| LodSceneLevel::Resolution(_) => Some(stick_assets::standard::LOW),
				// Stick mesh UltraLow is authored as an empty scene.
				LodSceneLevel::UltraLow => None,
			},
			Self::StandardTrunk => match level {
				LodSceneLevel::High => Some(stick_assets::standard_trunk::HIGH),
				LodSceneLevel::Medium => Some(stick_assets::standard_trunk::MID),
				LodSceneLevel::Low
				| LodSceneLevel::Distance(_)
				| LodSceneLevel::Resolution(_) => Some(stick_assets::standard_trunk::LOW),
				LodSceneLevel::UltraLow => None,
			},
		}
	}
}
