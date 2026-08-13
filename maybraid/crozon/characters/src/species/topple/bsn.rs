//! BSN scenes for Topple.
//!
//! `data_scene()` carries the semantic [`Topple`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::{ToppleAssets, ToppleBeakMesh, ToppleHeadMesh},
	pose::{TopplePose, TOPPLE_OVERALL_SCALE},
	ToppleColors, ToppleConfig,
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

/// Cartoonishly large head relative to the ~2 ft whelp body.
const HEAD_RIG_SOCKET_SCALE: f32 = 1.85;

/// Semantic Topple data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`ToppleConfig::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Topple {
	pub beak: ToppleBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: ToppleColors,
}

impl Topple {
	pub fn from_config(config: &ToppleConfig) -> Self {
		Self {
			beak: config.beak,
			eye: config.eye,
			hair: config.hair,
			colors: config.colors.clone(),
		}
	}
}

impl Default for Topple {
	fn default() -> Self {
		Self::from_config(&ToppleConfig::default_preview())
	}
}

impl CharacterComponents for Topple {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(TopplePose.resolve())
				.with_normalization(AssetNormalization::centroid(TOPPLE_OVERALL_SCALE)),
			humanoid::orthograde_head_rig_at(
				AssetNormalization::base_y(0.26),
				Transform::from_scale(Vec3::splat(HEAD_RIG_SOCKET_SCALE)),
			),
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
				ToppleHeadMesh::Meerkat.label(),
				ToppleHeadMesh::Meerkat.path().as_str(),
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

impl ToppleConfig {
	/// Semantic layer: the root [`Topple`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let topple = Topple::from_config(self);
		bsn! { template_value(topple) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = ToppleAssets::resolve(self);
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

fn part_color(colors: &ToppleColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::Mouth => colors.beak.color(),
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		_ => colors.plumage.color(),
	}
}
