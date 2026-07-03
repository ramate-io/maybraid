//! BSN scenes for Wumbus.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::WumbusAssets, WumbusColors, WumbusConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	species::{
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			EyeMesh, HairMesh,
		},
		wumbus::assets::WumbusHornMesh,
	},
};
/// Semantic Wumbus data attached to the character root entity.
///
/// Clothing is not part of the character: compose
/// [`crate::species::common::bsn::clothing_scene`] over `scene()` instead.
#[derive(Component, Clone, PartialEq)]
pub struct Wumbus {
	pub horns: WumbusHornMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: WumbusColors,
}

impl Wumbus {
	pub fn from_config(config: &WumbusConfig) -> Self {
		Self {
			horns: config.horns,
			eye: config.eye,
			hair: config.hair,
			colors: config.colors.clone(),
		}
	}
}

impl Default for Wumbus {
	fn default() -> Self {
		Self::from_config(&WumbusConfig::default_preview())
	}
}

impl WumbusConfig {
	/// Semantic layer: the root [`Wumbus`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let wumbus = Wumbus::from_config(self);
		bsn! { template_value(wumbus) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = WumbusAssets::resolve(self);
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

fn part_color(colors: &WumbusColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::Horns => colors.horns.color(),
		CharacterPartSlot::Spine => colors.spine.color(),
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => colors.ears.color(),
		CharacterPartSlot::Mouth => colors.mouth.color(),
		CharacterPartSlot::Hair => colors.hair.color(),
		_ => colors.skin.color(),
	}
}
