//! BSN scenes for Tapp.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::TappAssets, TappColors, TappConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	species::{
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			EyeMesh, HairMesh,
		},
		tapp::assets::TappBeakMesh,
	},
};

/// Semantic Tapp data attached to the character root entity.
///
/// Clothing is not part of the character: compose
/// [`crate::species::common::bsn::clothing_scene`] over `scene()` instead.
#[derive(Component, Clone, PartialEq)]
pub struct Tapp {
	pub beak: TappBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: TappColors,
}

impl Tapp {
	pub fn from_config(config: &TappConfig) -> Self {
		Self {
			beak: config.beak,
			eye: config.eye,
			hair: config.hair,
			colors: config.colors.clone(),
		}
	}
}

impl Default for Tapp {
	fn default() -> Self {
		Self::from_config(&TappConfig::default_preview())
	}
}

impl TappConfig {
	/// Semantic layer: the root [`Tapp`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let tapp = Tapp::from_config(self);
		bsn! { template_value(tapp) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = TappAssets::resolve(self);
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

fn part_color(colors: &TappColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::Mouth => colors.beak.color(),
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		// Body, head, and plumage-tinted crest.
		_ => colors.plumage.color(),
	}
}
