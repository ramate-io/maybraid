//! BSN scenes for Grener.
//!
//! `data_scene()` carries the semantic [`Grener`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::GrenerAssets,
	pose::{GrenerPose, GRENER_OVERALL_SCALE},
	GrenerColors, GrenerConfig,
};
use crate::{
	assembly::ResolvedCharacterPart,
	assets::AssetNormalization,
	components::CharacterComponents,
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{
		bsn::{self as common_bsn, WithBaseColor},
		nodes as humanoid, BODY_SHARK,
	},
};
use lod::gen::LodSceneLevel;

/// Semantic Grener data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`GrenerConfig::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Grener {
	pub colors: GrenerColors,
}

impl Grener {
	pub fn from_config(config: &GrenerConfig) -> Self {
		Self { colors: config.colors.clone() }
	}
}

impl Default for Grener {
	fn default() -> Self {
		Self::from_config(&GrenerConfig::default_preview())
	}
}

impl CharacterComponents for Grener {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![humanoid::forelimbed_body_rig(GrenerPose.resolve())
			.with_normalization(AssetNormalization::centroid(GRENER_OVERALL_SCALE))])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled("body", vec![humanoid::body_part("shark", BODY_SHARK.as_str())])
	}
}

impl GrenerConfig {
	pub fn data_scene(&self) -> impl Scene {
		let grener = Grener::from_config(self);
		bsn! { template_value(grener) }
	}

	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = GrenerAssets::resolve(self);
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

fn part_color(colors: &GrenerColors, _part: &ResolvedCharacterPart) -> Color {
	colors.body.color()
}
