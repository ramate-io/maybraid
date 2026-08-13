//! BSN scenes for Thumplus.
//!
//! `data_scene()` carries the semantic [`Thumplus`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::ThumplusAssets,
	pose::{ThumplusPose, THUMPLUS_OVERALL_SCALE},
	ThumplusColors, ThumplusConfig,
};
use crate::{
	assembly::ResolvedCharacterPart,
	assets::AssetNormalization,
	components::CharacterComponents,
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{
		bsn::{self as common_bsn, WithBaseColor},
		nodes as humanoid, BODY_WHALE,
	},
};
use lod::gen::LodSceneLevel;

/// Semantic Thumplus data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`ThumplusConfig::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Thumplus {
	pub colors: ThumplusColors,
}

impl Thumplus {
	pub fn from_config(config: &ThumplusConfig) -> Self {
		Self { colors: config.colors.clone() }
	}
}

impl Default for Thumplus {
	fn default() -> Self {
		Self::from_config(&ThumplusConfig::default_preview())
	}
}

impl CharacterComponents for Thumplus {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![humanoid::forelimbed_body_rig(ThumplusPose.resolve())
			.with_normalization(AssetNormalization::centroid(THUMPLUS_OVERALL_SCALE))])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled("body", vec![humanoid::body_part("whale", BODY_WHALE.as_str())])
	}
}

impl ThumplusConfig {
	pub fn data_scene(&self) -> impl Scene {
		let thumplus = Thumplus::from_config(self);
		bsn! { template_value(thumplus) }
	}

	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = ThumplusAssets::resolve(self);
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

fn part_color(colors: &ThumplusColors, _part: &ResolvedCharacterPart) -> Color {
	colors.body.color()
}
