//! BSN scenes for Croconot.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::CroconotAssets, sliders::CroconotSliders, CroconotColors, CroconotConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	presets::{BuildPreset, GenderPreset},
	species::{
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			EyeMesh,
		},
		croconot::assets::{CroconotBodyMesh, CroconotHeadMesh, CroconotHornMesh},
	},
};

/// Semantic Croconot data attached to the character root entity.
#[derive(Component, Clone, PartialEq)]
pub struct Croconot {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub horns: CroconotHornMesh,
	pub eye: EyeMesh,
	pub colors: CroconotColors,
	pub sliders: CroconotSliders,
}

impl Croconot {
	pub fn from_config(config: &CroconotConfig) -> Self {
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

impl Default for Croconot {
	fn default() -> Self {
		Self::from_config(&CroconotConfig::default_preview())
	}
}

impl CroconotConfig {
	pub fn data_scene(&self) -> impl Scene {
		let croconot = Croconot::from_config(self);
		bsn! { template_value(croconot) }
	}

	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = CroconotAssets::resolve(self);
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

fn part_color(colors: &CroconotColors, part: &ResolvedCharacterPart) -> Color {
	let item = match part.slot {
		CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => colors.head,
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes,
		CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => colors.ears,
		CharacterPartSlot::Mouth => colors.mouth,
		CharacterPartSlot::Horns => colors.horns,
		CharacterPartSlot::Tail => colors.tail,
		CharacterPartSlot::BodyMesh => colors.body,
		_ => colors.body,
	};
	item.color()
}

// Keep fixed mesh enums referenced for compile-time asset wiring checks.
const _: CroconotBodyMesh = CroconotBodyMesh::Dragloon;
const _: CroconotHeadMesh = CroconotHeadMesh::Canine;
