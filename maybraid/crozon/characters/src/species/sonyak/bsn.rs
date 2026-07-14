//! BSN scenes for Sonyak.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::SonyakAssets, sliders::SonyakSliders, SonyakColors, SonyakConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	presets::{BuildPreset, GenderPreset},
	species::{
		common::bsn::{self as common_bsn, WithBaseColor},
		sonyak::assets::{SonyakBodyMesh, SonyakHeadMesh, SonyakMouthMesh},
	},
};

/// Semantic Sonyak data attached to the character root entity.
#[derive(Component, Clone, PartialEq)]
pub struct Sonyak {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub mouth: SonyakMouthMesh,
	pub colors: SonyakColors,
	pub sliders: SonyakSliders,
}

impl Sonyak {
	pub fn from_config(config: &SonyakConfig) -> Self {
		Self {
			gender: config.gender,
			build: config.build,
			mouth: config.mouth,
			colors: config.colors.clone(),
			sliders: config.sliders,
		}
	}
}

impl Default for Sonyak {
	fn default() -> Self {
		Self::from_config(&SonyakConfig::default_preview())
	}
}

impl SonyakConfig {
	pub fn data_scene(&self) -> impl Scene {
		let sonyak = Sonyak::from_config(self);
		bsn! { template_value(sonyak) }
	}

	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = SonyakAssets::resolve(self);
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

fn part_color(colors: &SonyakColors, part: &ResolvedCharacterPart) -> Color {
	let item = match part.slot {
		CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => colors.head,
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes,
		CharacterPartSlot::Mouth => colors.mouth,
		CharacterPartSlot::Hair => colors.hair,
		CharacterPartSlot::Tail => colors.tail,
		CharacterPartSlot::BodyMesh => colors.body,
		_ => colors.body,
	};
	item.color()
}

const _: SonyakBodyMesh = SonyakBodyMesh::Gumbus;
const _: SonyakHeadMesh = SonyakHeadMesh::BarredBowl;
const _: SonyakMouthMesh = SonyakMouthMesh::Cow;
