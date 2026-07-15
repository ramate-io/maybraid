//! BSN scenes for Tuberwaber.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::TuberwaberAssets, sliders::TuberwaberSliders, TuberwaberColors, TuberwaberConfig,
};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	presets::{BuildPreset, GenderPreset},
	species::{
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			EarMesh, EyeMesh, HairMesh, MouthMesh, NoseMesh,
		},
		tuberwaber::assets::{TuberwaberBodyMesh, TuberwaberHeadMesh},
	},
};

/// Semantic Tuberwaber data attached to the character root entity.
#[derive(Component, Clone, PartialEq)]
pub struct Tuberwaber {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub body: TuberwaberBodyMesh,
	pub head: TuberwaberHeadMesh,
	pub eye: EyeMesh,
	pub nose: NoseMesh,
	pub mouth: MouthMesh,
	pub ear: EarMesh,
	pub hair: HairMesh,
	pub colors: TuberwaberColors,
	pub sliders: TuberwaberSliders,
}

impl Tuberwaber {
	pub fn from_config(config: &TuberwaberConfig) -> Self {
		Self {
			gender: config.gender,
			build: config.build,
			body: config.body,
			head: config.head,
			eye: config.eye,
			nose: config.nose,
			mouth: config.mouth,
			ear: config.ear,
			hair: config.hair,
			colors: config.colors.clone(),
			sliders: config.sliders,
		}
	}
}

impl Default for Tuberwaber {
	fn default() -> Self {
		Self::from_config(&TuberwaberConfig::default_preview())
	}
}

impl TuberwaberConfig {
	pub fn data_scene(&self) -> impl Scene {
		let tuberwaber = Tuberwaber::from_config(self);
		bsn! { template_value(tuberwaber) }
	}

	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = TuberwaberAssets::resolve(self);
		let sliders = self.sliders.clamped();
		let colors = self.colors.clone();
		common_bsn::assembly_visual_scene::<M>(
			&assembly,
			move |part| {
				part.asset
					.normalization
					.transform()
					.mul_transform(sliders.feature_transform(part.slot))
			},
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

fn part_color(colors: &TuberwaberColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh | CharacterPartSlot::Horns => {
			colors.head.color()
		}
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		CharacterPartSlot::Nose => colors.nose.color(),
		CharacterPartSlot::Mouth => colors.mouth.color(),
		CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => colors.ears.color(),
		CharacterPartSlot::Hair => colors.hair.color(),
		_ => colors.body.color(),
	}
}

const _: TuberwaberBodyMesh = TuberwaberBodyMesh::Tuberwaber;
const _: TuberwaberHeadMesh = TuberwaberHeadMesh::Tuberwaber;
