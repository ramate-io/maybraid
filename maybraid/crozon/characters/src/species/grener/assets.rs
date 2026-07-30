//! Grener asset catalog and assembly resolver.

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget,
	},
	assets::AssetNormalization,
	species::{
		common::{BODY_SHARK, FORELIMBED_RIG},
		grener::{
			pose::{GrenerPose, GRENER_OVERALL_SCALE},
			GrenerConfig,
		},
	},
};

pub struct GrenerAssets;

impl GrenerAssets {
	pub fn resolve(_config: &GrenerConfig) -> ResolvedCharacterAssembly {
		ResolvedCharacterAssembly::new(
			"Grener",
			RigAsset::new("Forelimbed", FORELIMBED_RIG)
				.with_normalization(AssetNormalization::centroid(GRENER_OVERALL_SCALE)),
			GrenerPose.resolve(),
		)
		.with_part(Self::body_mesh())
	}

	fn body_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new("shark", BODY_SHARK, AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			None,
		)
	}
}
