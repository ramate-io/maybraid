//! BSN scenes for Lero.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::LeroAssets, LeroColors, LeroConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	species::{
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			HairMesh,
		},
		lero::assets::LeroMouthMesh,
	},
};
/// Semantic Lero data attached to the character root entity.
///
/// Clothing is not part of the character: compose
/// [`crate::species::common::bsn::clothing_scene`] over `scene()` instead.
#[derive(Component, Clone, PartialEq)]
pub struct Lero {
	pub mouth: LeroMouthMesh,
	pub hair: HairMesh,
	pub colors: LeroColors,
}

impl Lero {
	pub fn from_config(config: &LeroConfig) -> Self {
		Self { mouth: config.mouth, hair: config.hair, colors: config.colors.clone() }
	}
}

impl Default for Lero {
	fn default() -> Self {
		Self::from_config(&LeroConfig::default_preview())
	}
}

impl LeroConfig {
	/// Semantic layer: the root [`Lero`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let lero = Lero::from_config(self);
		bsn! { template_value(lero) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = LeroAssets::resolve(self);
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

fn part_color(colors: &LeroColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::Mouth => colors.mouth.color(),
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		CharacterPartSlot::Tail => colors.tail.color(),
		CharacterPartSlot::Spine => colors.spine.color(),
		CharacterPartSlot::Hair => colors.hair.color(),
		_ => colors.skin.color(),
	}
}
