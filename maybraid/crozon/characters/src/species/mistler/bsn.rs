//! BSN scenes for Mistler.
//!
//! `data_scene()` carries the semantic [`Mistler`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::MistlerAssets,
	pose::{MistlerPose, MISTLER_OVERALL_SCALE},
	MistlerColors, MistlerConfig,
};
use crate::{
	assembly::ResolvedCharacterPart,
	assets::AssetNormalization,
	components::CharacterComponents,
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{
		bsn::{self as common_bsn, WithBaseColor},
		nodes as humanoid, BODY_SPRITE_FISH,
	},
};
use lod::gen::LodSceneLevel;

/// Semantic Mistler data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`MistlerConfig::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Mistler {
	pub colors: MistlerColors,
}

impl Mistler {
	pub fn from_config(config: &MistlerConfig) -> Self {
		Self { colors: config.colors.clone() }
	}
}

impl Default for Mistler {
	fn default() -> Self {
		Self::from_config(&MistlerConfig::default_preview())
	}
}

impl CharacterComponents for Mistler {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![humanoid::forelimbed_body_rig(MistlerPose.resolve())
			.with_normalization(AssetNormalization::centroid(MISTLER_OVERALL_SCALE))])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled(
			"body",
			vec![humanoid::body_part("sprite-fish", BODY_SPRITE_FISH.as_str())],
		)
	}
}

impl MistlerConfig {
	pub fn data_scene(&self) -> impl Scene {
		let mistler = Mistler::from_config(self);
		bsn! { template_value(mistler) }
	}

	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = MistlerAssets::resolve(self);
		let colors = self.colors.clone();
		common_bsn::assembly_visual_scene::<M>(
			&assembly,
			|part| part.asset.normalization.transform(),
			move |part| part_color(&colors, part),
		)
	}

	pub fn scene<M: WithBaseColor>(&self) -> impl Scene {
		let data = self.data_scene();
		let visual = self.visual_scene::<M>();
		bsn! {
			{data}
			Children [ ({visual}) ]
		}
	}
}

fn part_color(colors: &MistlerColors, _part: &ResolvedCharacterPart) -> Color {
	colors.body.color()
}
