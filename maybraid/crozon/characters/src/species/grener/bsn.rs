//! BSN scenes for Grener.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::GrenerAssets, GrenerColors, GrenerConfig};
use crate::{
	assembly::ResolvedCharacterPart,
	species::common::bsn::{self as common_bsn, WithBaseColor},
};

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
