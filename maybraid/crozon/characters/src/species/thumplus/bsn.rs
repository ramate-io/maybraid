//! BSN scenes for Thumplus.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::ThumplusAssets, ThumplusColors, ThumplusConfig};
use crate::{
	assembly::ResolvedCharacterPart,
	species::common::bsn::{self as common_bsn, WithBaseColor},
};

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
