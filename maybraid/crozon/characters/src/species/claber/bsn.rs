//! BSN scenes for Claber.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::ClaberAssets, sliders::ClaberSliders, ClaberColors, ClaberConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	presets::{BuildPreset, GenderPreset},
	species::{
		claber::assets::{ClaberBodyMesh, ClaberHeadMesh, ClaberHornMesh},
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			EyeMesh,
		},
	},
};

/// Semantic Claber data attached to the character root entity.
#[derive(Component, Clone, PartialEq)]
pub struct Claber {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub horns: ClaberHornMesh,
	pub eye: EyeMesh,
	pub colors: ClaberColors,
	pub sliders: ClaberSliders,
}

impl Claber {
	pub fn from_config(config: &ClaberConfig) -> Self {
		Self {
			gender: config.gender,
			build: config.build,
			horns: config.horns,
			eye: config.eye,
			colors: config.colors.clone(),
			sliders: config.sliders,
		}
	}
}

impl Default for Claber {
	fn default() -> Self {
		Self::from_config(&ClaberConfig::default_preview())
	}
}

impl ClaberConfig {
	pub fn data_scene(&self) -> impl Scene {
		let claber = Claber::from_config(self);
		bsn! { template_value(claber) }
	}

	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = ClaberAssets::resolve(self);
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

fn part_color(colors: &ClaberColors, part: &ResolvedCharacterPart) -> Color {
	let tone = match part.slot {
		CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => colors.head,
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes,
		CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => colors.ears,
		CharacterPartSlot::Mouth => colors.mouth,
		CharacterPartSlot::Horns => colors.horns,
		CharacterPartSlot::Tail => colors.tail,
		CharacterPartSlot::BodyMesh => colors.body,
		_ => colors.body,
	};
	tone.color()
}

// Keep fixed mesh enums referenced for compile-time asset wiring checks.
const _: ClaberBodyMesh = ClaberBodyMesh::Gumbus;
const _: ClaberHeadMesh = ClaberHeadMesh::Caole;
const _: ClaberHornMesh = ClaberHornMesh::HarrowedCrown;
