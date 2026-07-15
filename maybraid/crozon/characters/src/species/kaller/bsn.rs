//! BSN scenes for Kaller.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::KallerAssets, KallerColors, KallerConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	species::common::{
		bsn::{self as common_bsn, WithBaseColor},
		EyeMesh, HairMesh,
	},
};

/// Semantic Kaller data attached to the character root entity.
///
/// Clothing is not part of the character: compose
/// [`crate::species::common::bsn::clothing_scene`] over `scene()` instead.
#[derive(Component, Clone, PartialEq)]
pub struct Kaller {
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: KallerColors,
}

impl Kaller {
	pub fn from_config(config: &KallerConfig) -> Self {
		Self {
			eye: config.eye,
			hair: config.hair,
			colors: config.colors.clone(),
		}
	}
}

impl Default for Kaller {
	fn default() -> Self {
		Self::from_config(&KallerConfig::default_preview())
	}
}

impl KallerConfig {
	/// Semantic layer: the root [`Kaller`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let kaller = Kaller::from_config(self);
		bsn! { template_value(kaller) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = KallerAssets::resolve(self);
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

fn part_color(colors: &KallerColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::Mouth => colors.snout.color(),
		CharacterPartSlot::Horns => colors.crown.color(),
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		_ => colors.plumage.color(),
	}
}
