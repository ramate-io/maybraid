//! BSN scenes for Brodler.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::BrodlerAssets, BrodlerColors, BrodlerConfig, BrodlerHeadMesh};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	species::{
		brodler::assets::HornMesh,
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			EarMesh, EyeMesh, HairMesh, MouthMesh, NoseMesh,
		},
	},
};
/// Semantic Brodler data attached to the character root entity.
///
/// Clothing is not part of the character: compose
/// [`crate::species::common::bsn::clothing_scene`] over `scene()` instead.
#[derive(Component, Clone, PartialEq)]
pub struct Brodler {
	pub head: BrodlerHeadMesh,
	pub horns: HornMesh,
	pub eye: EyeMesh,
	pub nose: NoseMesh,
	pub mouth: MouthMesh,
	pub ear: EarMesh,
	pub hair: HairMesh,
	pub colors: BrodlerColors,
}

impl Brodler {
	pub fn from_config(config: &BrodlerConfig) -> Self {
		Self {
			head: config.head,
			horns: config.horns,
			eye: config.eye,
			nose: config.nose,
			mouth: config.mouth,
			ear: config.ear,
			hair: config.hair,
			colors: config.colors.clone(),
		}
	}
}

impl Default for Brodler {
	fn default() -> Self {
		Self::from_config(&BrodlerConfig::default_preview())
	}
}

impl BrodlerConfig {
	/// Semantic layer: the root [`Brodler`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let brodler = Brodler::from_config(self);
		bsn! { template_value(brodler) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = BrodlerAssets::resolve(self);
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

fn part_color(colors: &BrodlerColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::Horns => colors.horns.color(),
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		CharacterPartSlot::Mouth => colors.mouth.color(),
		CharacterPartSlot::Hair => colors.hair.color(),
		_ => colors.skin.color(),
	}
}
