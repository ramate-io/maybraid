//! Mistler asset catalog and assembly resolver.

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget,
	},
	assets::AssetNormalization,
	species::{
		common::{BODY_SPRITE_FISH, FORELIMBED_RIG},
		mistler::{
			pose::{MistlerPose, MISTLER_OVERALL_SCALE},
			MistlerConfig,
		},
	},
};

pub struct MistlerAssets;

impl MistlerAssets {
	pub fn resolve(_config: &MistlerConfig) -> ResolvedCharacterAssembly {
		ResolvedCharacterAssembly::new(
			"Mistler",
			RigAsset::new("Forelimbed", FORELIMBED_RIG)
				.with_normalization(AssetNormalization::centroid(MISTLER_OVERALL_SCALE)),
			MistlerPose.resolve(),
		)
		.with_part(Self::body_mesh())
	}

	fn body_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new("sprite-fish", BODY_SPRITE_FISH, AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			None,
		)
	}
}
