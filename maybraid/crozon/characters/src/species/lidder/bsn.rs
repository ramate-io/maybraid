//! BSN scenes for Lidder.
//!
//! `data_scene()` carries the semantic [`Lidder`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::{LidderAssets, LidderBeakMesh, LidderHeadMesh},
	pose::LidderPose,
	LidderColors, LidderConfig,
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

/// Semantic Lidder data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`LidderConfig::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Lidder {
	pub beak: LidderBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: LidderColors,
}

impl Lidder {
	pub fn from_config(config: &LidderConfig) -> Self {
		Self {
			beak: config.beak,
			eye: config.eye,
			hair: config.hair,
			colors: config.colors.clone(),
		}
	}
}

impl Default for Lidder {
	fn default() -> Self {
		Self::from_config(&LidderConfig::default_preview())
	}
}

impl CharacterComponents for Lidder {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(LidderPose.resolve()),
			humanoid::orthograde_head_rig(),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = Layers::from_labeled(
			"body",
			vec![humanoid::body_part("crane", "characters/bodies/crane_body.glb")],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				LidderHeadMesh::Meerkat.label(),
				LidderHeadMesh::Meerkat.path().as_str(),
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

impl LidderConfig {
	/// Semantic layer: the root [`Lidder`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let lidder = Lidder::from_config(self);
		bsn! { template_value(lidder) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = LidderAssets::resolve(self);
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

fn part_color(colors: &LidderColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::Mouth => colors.beak.color(),
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		// Body, head, and plumage-tinted crest.
		_ => colors.plumage.color(),
	}
}
