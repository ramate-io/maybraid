//! BSN scenes for Tipple.
//!
//! `data_scene()` carries the semantic [`Tipple`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::{TippleAssets, TippleBeakMesh, TippleHeadMesh},
	pose::{TipplePose, TIPPLE_OVERALL_SCALE},
	TippleColors, TippleConfig,
};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	assets::AssetNormalization,
	components::CharacterComponents,
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{
		bsn::{self as common_bsn, WithBaseColor},
		nodes as humanoid, EyeMesh, HairMesh,
	},
};
use lod::gen::LodSceneLevel;

/// Semantic Tipple data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`TippleConfig::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Tipple {
	pub beak: TippleBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: TippleColors,
}

impl Tipple {
	pub fn from_config(config: &TippleConfig) -> Self {
		Self {
			beak: config.beak,
			eye: config.eye,
			hair: config.hair,
			colors: config.colors.clone(),
		}
	}
}

impl Default for Tipple {
	fn default() -> Self {
		Self::from_config(&TippleConfig::default_preview())
	}
}

impl CharacterComponents for Tipple {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(TipplePose.resolve())
				.with_normalization(AssetNormalization::centroid(TIPPLE_OVERALL_SCALE)),
			humanoid::orthograde_head_rig(),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = Layers::from_labeled(
			"body",
			vec![humanoid::body_part("whelp", "characters/bodies/whelp_bird.glb")],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				TippleHeadMesh::Meerkat.label(),
				TippleHeadMesh::Meerkat.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::eye_left(self.eye),
			humanoid::eye_right(self.eye),
			humanoid::head_feature(
				CharacterPartSlot::Mouth,
				self.beak.label(),
				self.beak.path().as_str(),
				AssetNormalization::centroid(0.35),
				"mouth_socket",
				humanoid::mouth_socket_local(),
			),
		];
		if let Some(hair) = humanoid::hair_scaled(
			self.hair,
			match self.hair {
				HairMesh::FeatherHawk => 0.4,
				_ => 1.0,
			},
		) {
			features.push(hair);
		}
		out.extend_labeled("features", features);
		out
	}
}

impl TippleConfig {
	/// Semantic layer: the root [`Tipple`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let tipple = Tipple::from_config(self);
		bsn! { template_value(tipple) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = TippleAssets::resolve(self);
		let colors = self.colors.clone();
		common_bsn::assembly_visual_scene::<M>(
			&assembly,
			|part| part.asset.normalization.transform(),
			move |part| part_color(&colors, part),
		)
	}

	/// Full character: semantic root with the visual hierarchy underneath.
	pub fn scene<M: WithBaseColor>(&self) -> impl Scene {
		let data = self.data_scene();
		let visual = self.visual_scene::<M>();
		bsn! {
			{data}
			Children [ ({visual}) ]
		}
	}
}

fn part_color(colors: &TippleColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::Mouth => colors.beak.color(),
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		// Body, head, and plumage-tinted crest.
		_ => colors.plumage.color(),
	}
}
