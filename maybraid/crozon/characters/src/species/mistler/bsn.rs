//! BSN scenes for Mistler.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::MistlerAssets, MistlerColors, MistlerConfig};
use crate::{
	assembly::ResolvedCharacterPart,
	species::common::bsn::{self as common_bsn, WithBaseColor},
};

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
