//! BSN scenes for Yilter.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::YilterAssets, sliders::YilterSliders, YilterColors, YilterConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	presets::{BuildPreset, GenderPreset},
	species::{
		common::bsn::{self as common_bsn, WithBaseColor},
		ylter::assets::{YilterBodyMesh, YilterHeadMesh, YilterMouthMesh},
	},
};

/// Semantic Yilter data attached to the character root entity.
#[derive(Component, Clone, PartialEq)]
pub struct Yilter {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub mouth: YilterMouthMesh,
	pub colors: YilterColors,
	pub sliders: YilterSliders,
}

impl Yilter {
	pub fn from_config(config: &YilterConfig) -> Self {
		Self {
			gender: config.gender,
			build: config.build,
			mouth: config.mouth,
			colors: config.colors.clone(),
			sliders: config.sliders,
		}
	}
}

impl Default for Yilter {
	fn default() -> Self {
		Self::from_config(&YilterConfig::default_preview())
	}
}

impl YilterConfig {
	pub fn data_scene(&self) -> impl Scene {
		let ylter = Yilter::from_config(self);
		bsn! { template_value(ylter) }
	}

	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = YilterAssets::resolve(self);
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

fn part_color(colors: &YilterColors, part: &ResolvedCharacterPart) -> Color {
	let item = match part.slot {
		CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => colors.head,
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes,
		CharacterPartSlot::Mouth => colors.mouth,
		CharacterPartSlot::Tail => colors.tail,
		CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => colors.neck,
		CharacterPartSlot::BodyMesh => colors.body,
		_ => colors.body,
	};
	item.color()
}

const _: YilterBodyMesh = YilterBodyMesh::Rumbler;
const _: YilterHeadMesh = YilterHeadMesh::BarredBowl;
const _: YilterMouthMesh = YilterMouthMesh::Cow;
