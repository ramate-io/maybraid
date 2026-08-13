//! BSN scenes for Braidman.
//!
//! `data_scene()` carries the semantic [`Braidman`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::BraidmanAssets, pose::BraidmanPose, sliders::BraidmanSliders, BraidmanColors,
	BraidmanConfig,
};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	components::CharacterComponents,
	layer::Layers,
	nodes::{PartNode, RigNode},
	presets::{BuildPreset, GenderPreset},
	species::common::{
		bsn::{self as common_bsn, WithBaseColor},
		BodyMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh,
	},
};
use lod::gen::LodSceneLevel;

/// Semantic Braidman data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`BraidmanConfig::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Braidman {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub body: BodyMesh,
	pub head: HeadMesh,
	pub eye: EyeMesh,
	pub nose: NoseMesh,
	pub mouth: MouthMesh,
	pub ear: EarMesh,
	pub hair: HairMesh,
	pub colors: BraidmanColors,
	pub sliders: BraidmanSliders,
}

impl Braidman {
	pub fn from_config(config: &BraidmanConfig) -> Self {
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

impl Default for Braidman {
	fn default() -> Self {
		Self::from_config(&BraidmanConfig::default_preview())
	}
}

impl CharacterComponents for Braidman {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		let pose = BraidmanPose {
			gender: self.gender,
			build: self.build,
			sliders: self.sliders.clamped(),
		}
		.resolve();
		Layers::from_free(vec![
			BraidmanAssets::body_rig_node_with_pose(pose),
			BraidmanAssets::head_rig_node(),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let sliders = self.sliders.clamped();
		let mut out = Layers::from_labeled("body", vec![BraidmanAssets::body_mesh_node(self.body)]);
		out.extend_labeled("head", vec![BraidmanAssets::head_mesh_node(self.head)]);
		let mut features = vec![
			BraidmanAssets::eye_left_node(
				self.eye,
				sliders.feature_transform(CharacterPartSlot::EyeLeft),
			),
			BraidmanAssets::eye_right_node(
				self.eye,
				sliders.feature_transform(CharacterPartSlot::EyeRight),
			),
			BraidmanAssets::nose_node(
				self.nose,
				sliders.feature_transform(CharacterPartSlot::Nose),
			),
			BraidmanAssets::mouth_node(
				self.mouth,
				sliders.feature_transform(CharacterPartSlot::Mouth),
			),
			BraidmanAssets::ear_left_node(
				self.ear,
				sliders.feature_transform(CharacterPartSlot::EarLeft),
			),
			BraidmanAssets::ear_right_node(
				self.ear,
				sliders.feature_transform(CharacterPartSlot::EarRight),
			),
		];
		if let Some(hair) =
			BraidmanAssets::hair_node(self.hair, sliders.feature_transform(CharacterPartSlot::Hair))
		{
			features.push(hair);
		}
		out.extend_labeled("features", features);
		out
	}
}

impl BraidmanConfig {
	/// Semantic layer: the root [`Braidman`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let braidman = Braidman::from_config(self);
		bsn! { template_value(braidman) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = BraidmanAssets::resolve(self);
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

fn part_color(colors: &BraidmanColors, part: &ResolvedCharacterPart) -> Color {
	let item = match part.slot {
		CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh | CharacterPartSlot::Horns => {
			colors.head
		}
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes,
		CharacterPartSlot::Nose => colors.nose,
		CharacterPartSlot::Mouth => colors.mouth,
		CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => colors.ears,
		CharacterPartSlot::Hair => colors.hair,
		_ => colors.body,
	};
	item.color()
}
