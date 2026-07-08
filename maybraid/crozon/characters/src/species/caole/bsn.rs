//! BSN scenes for Caole.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::CaoleAssets, sliders::CaoleSliders, CaoleColors, CaoleConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	presets::{BuildPreset, GenderPreset},
	species::{
		caole::assets::{CaoleBodyMesh, CaoleHeadMesh, CaoleMouthMesh},
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			EyeMesh,
		},
	},
};

/// Semantic Caole data attached to the character root entity.
#[derive(Component, Clone, PartialEq)]
pub struct Caole {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub body: CaoleBodyMesh,
	pub head: CaoleHeadMesh,
	pub mouth: CaoleMouthMesh,
	pub eye: EyeMesh,
	pub colors: CaoleColors,
	pub sliders: CaoleSliders,
}

impl Caole {
	pub fn from_config(config: &CaoleConfig) -> Self {
		Self {
			gender: config.gender,
			build: config.build,
			body: config.body,
			head: config.head,
			mouth: config.mouth,
			eye: config.eye,
			colors: config.colors.clone(),
			sliders: config.sliders,
		}
	}
}

impl Default for Caole {
	fn default() -> Self {
		Self::from_config(&CaoleConfig::default_preview())
	}
}

impl CaoleConfig {
	pub fn data_scene(&self) -> impl Scene {
		let caole = Caole::from_config(self);
		bsn! { template_value(caole) }
	}

	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = CaoleAssets::resolve(self);
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

fn part_color(colors: &CaoleColors, part: &ResolvedCharacterPart) -> Color {
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
const _: CaoleBodyMesh = CaoleBodyMesh::Gumbus;
const _: CaoleBodyMesh = CaoleBodyMesh::Rumbler;
const _: CaoleHeadMesh = CaoleHeadMesh::Caole;
const _: CaoleMouthMesh = CaoleMouthMesh::Cow;
