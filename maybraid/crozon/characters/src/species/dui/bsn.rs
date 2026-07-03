//! BSN scenes for Dui.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::DuiAssets, DuiColors, DuiConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	species::{
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			HairMesh,
		},
		dui::assets::DuiNoseMesh,
	},
};
/// Semantic Dui data attached to the character root entity.
///
/// Clothing is not part of the character: compose
/// [`crate::species::common::bsn::clothing_scene`] over `scene()` instead.
#[derive(Component, Clone, PartialEq)]
pub struct Dui {
	pub nose: DuiNoseMesh,
	pub hair: HairMesh,
	pub colors: DuiColors,
}

impl Dui {
	pub fn from_config(config: &DuiConfig) -> Self {
		Self { nose: config.nose, hair: config.hair, colors: config.colors.clone() }
	}
}

impl Default for Dui {
	fn default() -> Self {
		Self::from_config(&DuiConfig::default_preview())
	}
}

impl DuiConfig {
	/// Semantic layer: the root [`Dui`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let dui = Dui::from_config(self);
		bsn! { template_value(dui) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = DuiAssets::resolve(self);
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

fn part_color(colors: &DuiColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::Nose => colors.nose_color.color(),
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		CharacterPartSlot::Mouth => colors.mouth.color(),
		CharacterPartSlot::Hair => colors.hair.color(),
		_ => colors.skin.color(),
	}
}
