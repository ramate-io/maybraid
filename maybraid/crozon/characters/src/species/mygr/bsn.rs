//! BSN scenes for Mygr.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::MygrAssets, MygrColors, MygrConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	species::common::{
		bsn::{self as common_bsn, WithBaseColor},
		EyeMesh, HairMesh,
	},
};
/// Semantic Mygr data attached to the character root entity.
///
/// Clothing is not part of the character: compose
/// [`crate::species::common::bsn::clothing_scene`] over `scene()` instead.
#[derive(Component, Clone, PartialEq)]
pub struct Mygr {
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: MygrColors,
}

impl Mygr {
	pub fn from_config(config: &MygrConfig) -> Self {
		Self { eye: config.eye, hair: config.hair, colors: config.colors.clone() }
	}
}

impl Default for Mygr {
	fn default() -> Self {
		Self::from_config(&MygrConfig::default_preview())
	}
}

impl MygrConfig {
	/// Semantic layer: the root [`Mygr`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let mygr = Mygr::from_config(self);
		bsn! { template_value(mygr) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = MygrAssets::resolve(self);
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

fn part_color(colors: &MygrColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		CharacterPartSlot::Mouth => colors.mouth.color(),
		CharacterPartSlot::Hair => colors.hair.color(),
		_ => colors.skin.color(),
	}
}
