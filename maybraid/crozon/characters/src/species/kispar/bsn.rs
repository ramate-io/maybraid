//! BSN scenes for Kispar.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::KisparAssets, KisparColors, KisparConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	species::{
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			EyeMesh, HairMesh,
		},
		kispar::assets::KisparBeakMesh,
	},
};

/// Semantic Kispar data attached to the character root entity.
///
/// Clothing is not part of the character: compose
/// [`crate::species::common::bsn::clothing_scene`] over `scene()` instead.
#[derive(Component, Clone, PartialEq)]
pub struct Kispar {
	pub beak: KisparBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: KisparColors,
}

impl Kispar {
	pub fn from_config(config: &KisparConfig) -> Self {
		Self {
			beak: config.beak,
			eye: config.eye,
			hair: config.hair,
			colors: config.colors.clone(),
		}
	}
}

impl Default for Kispar {
	fn default() -> Self {
		Self::from_config(&KisparConfig::default_preview())
	}
}

impl KisparConfig {
	/// Semantic layer: the root [`Kispar`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let kispar = Kispar::from_config(self);
		bsn! { template_value(kispar) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = KisparAssets::resolve(self);
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

fn part_color(colors: &KisparColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::Mouth => colors.beak.color(),
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		_ => colors.plumage.color(),
	}
}
