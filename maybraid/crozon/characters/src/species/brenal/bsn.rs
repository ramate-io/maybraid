//! BSN scenes for Brenal.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::BrenalAssets, sliders::BrenalSliders, BrenalColors, BrenalConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	presets::{BuildPreset, GenderPreset},
	species::{
		brenal::assets::{BrenalBodyMesh, BrenalHeadMesh, BrenalHornMesh},
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			EyeMesh,
		},
	},
};

/// Semantic Brenal data attached to the character root entity.
#[derive(Component, Clone, PartialEq)]
pub struct Brenal {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub horns: BrenalHornMesh,
	pub eye: EyeMesh,
	pub colors: BrenalColors,
	pub sliders: BrenalSliders,
}

impl Brenal {
	pub fn from_config(config: &BrenalConfig) -> Self {
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

impl Default for Brenal {
	fn default() -> Self {
		Self::from_config(&BrenalConfig::default_preview())
	}
}

impl BrenalConfig {
	pub fn data_scene(&self) -> impl Scene {
		let brenal = Brenal::from_config(self);
		bsn! { template_value(brenal) }
	}

	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = BrenalAssets::resolve(self);
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

fn part_color(colors: &BrenalColors, part: &ResolvedCharacterPart) -> Color {
	let item = match part.slot {
		CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => colors.head,
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes,
		CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => colors.ears,
		CharacterPartSlot::Horns => colors.horns,
		CharacterPartSlot::Tail => colors.tail,
		CharacterPartSlot::BodyMesh => colors.body,
		_ => colors.body,
	};
	item.color()
}

// Keep fixed mesh enums referenced for compile-time asset wiring checks.
const _: BrenalBodyMesh = BrenalBodyMesh::Gumbus;
const _: BrenalHeadMesh = BrenalHeadMesh::Canine;
