//! BSN scenes for Hars.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::HarsAssets, sliders::HarsSliders, HarsColors, HarsConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	presets::{BuildPreset, GenderPreset},
	species::{
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			EyeMesh,
		},
		hars::assets::{HarsBodyMesh, HarsHeadMesh, HarsMouthMesh},
	},
};

/// Semantic Hars data attached to the character root entity.
#[derive(Component, Clone, PartialEq)]
pub struct Hars {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub mouth: HarsMouthMesh,
	pub eye: EyeMesh,
	pub colors: HarsColors,
	pub sliders: HarsSliders,
}

impl Hars {
	pub fn from_config(config: &HarsConfig) -> Self {
		Self {
			gender: config.gender,
			build: config.build,
			mouth: config.mouth,
			eye: config.eye,
			colors: config.colors.clone(),
			sliders: config.sliders,
		}
	}
}

impl Default for Hars {
	fn default() -> Self {
		Self::from_config(&HarsConfig::default_preview())
	}
}

impl HarsConfig {
	pub fn data_scene(&self) -> impl Scene {
		let hars = Hars::from_config(self);
		bsn! { template_value(hars) }
	}

	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = HarsAssets::resolve(self);
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

fn part_color(colors: &HarsColors, part: &ResolvedCharacterPart) -> Color {
	let item = match part.slot {
		CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => colors.head,
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes,
		CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => colors.ears,
		CharacterPartSlot::Mouth => colors.mouth,
		CharacterPartSlot::Tail => colors.tail,
		CharacterPartSlot::BodyMesh => colors.body,
		_ => colors.body,
	};
	item.color()
}

// Keep fixed mesh enums referenced for compile-time asset wiring checks.
const _: HarsBodyMesh = HarsBodyMesh::Rumbler;
const _: HarsHeadMesh = HarsHeadMesh::Cowder;
const _: HarsMouthMesh = HarsMouthMesh::Cow;
