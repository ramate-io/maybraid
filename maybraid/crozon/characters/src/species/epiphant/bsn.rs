//! BSN scenes for Epiphant.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::EpiphantAssets, sliders::EpiphantSliders, EpiphantColors, EpiphantConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	presets::{BuildPreset, GenderPreset},
	species::{
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			EyeMesh,
		},
		epiphant::assets::{EpiphantBodyMesh, EpiphantEarMesh, EpiphantHeadMesh, EpiphantNoseMesh},
	},
};

/// Semantic Epiphant data attached to the character root entity.
#[derive(Component, Clone, PartialEq)]
pub struct Epiphant {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub body: EpiphantBodyMesh,
	pub head: EpiphantHeadMesh,
	pub ear: EpiphantEarMesh,
	pub nose: EpiphantNoseMesh,
	pub eye: EyeMesh,
	pub colors: EpiphantColors,
	pub sliders: EpiphantSliders,
}

impl Epiphant {
	pub fn from_config(config: &EpiphantConfig) -> Self {
		Self {
			gender: config.gender,
			build: config.build,
			body: config.body,
			head: config.head,
			ear: config.ear,
			nose: config.nose,
			eye: config.eye,
			colors: config.colors.clone(),
			sliders: config.sliders,
		}
	}
}

impl Default for Epiphant {
	fn default() -> Self {
		Self::from_config(&EpiphantConfig::default_preview())
	}
}

impl EpiphantConfig {
	pub fn data_scene(&self) -> impl Scene {
		let epiphant = Epiphant::from_config(self);
		bsn! { template_value(epiphant) }
	}

	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = EpiphantAssets::resolve(self);
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

fn part_color(colors: &EpiphantColors, part: &ResolvedCharacterPart) -> Color {
	let item = match part.slot {
		CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => colors.head,
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes,
		CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => colors.ears,
		CharacterPartSlot::Nose => colors.nose,
		CharacterPartSlot::Tail => colors.tail,
		CharacterPartSlot::BodyMesh => colors.body,
		_ => colors.body,
	};
	item.color()
}

const _: EpiphantBodyMesh = EpiphantBodyMesh::Epiphant;
const _: EpiphantHeadMesh = EpiphantHeadMesh::Meerkat;
const _: EpiphantEarMesh = EpiphantEarMesh::Epiphant;
const _: EpiphantNoseMesh = EpiphantNoseMesh::Trunkish;
