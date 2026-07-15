//! Thumplus asset catalog and assembly resolver.

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget,
	},
	assets::AssetNormalization,
	species::{
		common::{BODY_WHALE, FORELIMBED_RIG},
		thumplus::{
			pose::{ThumplusPose, THUMPLUS_OVERALL_SCALE},
			ThumplusConfig,
		},
	},
};

pub struct ThumplusAssets;

impl ThumplusAssets {
	pub fn resolve(_config: &ThumplusConfig) -> ResolvedCharacterAssembly {
		ResolvedCharacterAssembly::new(
			"Thumplus",
			RigAsset::new("Forelimbed", FORELIMBED_RIG)
				.with_normalization(AssetNormalization::centroid(THUMPLUS_OVERALL_SCALE)),
			ThumplusPose.resolve(),
		)
		.with_part(Self::body_mesh())
	}

	fn body_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new("whale", BODY_WHALE, AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			None,
		)
	}
}
