//! BSN scenes for Brodler.
//!
//! `data_scene()` carries the semantic [`Brodler`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::{BrodlerAssets, HornMesh},
	pose::BrodlerPose,
	BrodlerColors, BrodlerConfig, BrodlerHeadMesh,
};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	components::CharacterComponents,
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{
		bsn::{self as common_bsn, WithBaseColor},
		nodes as humanoid, BodyMesh, EarMesh, EyeMesh, HairMesh, MouthMesh, NoseMesh,
	},
};
use lod::gen::LodSceneLevel;

/// Semantic Brodler data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`BrodlerConfig::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Brodler {
	pub head: BrodlerHeadMesh,
	pub horns: HornMesh,
	pub eye: EyeMesh,
	pub nose: NoseMesh,
	pub mouth: MouthMesh,
	pub ear: EarMesh,
	pub hair: HairMesh,
	pub colors: BrodlerColors,
}

impl Brodler {
	pub fn from_config(config: &BrodlerConfig) -> Self {
		Self {
			head: config.head,
			horns: config.horns,
			eye: config.eye,
			nose: config.nose,
			mouth: config.mouth,
			ear: config.ear,
			hair: config.hair,
			colors: config.colors.clone(),
		}
	}
}

impl Default for Brodler {
	fn default() -> Self {
		Self::from_config(&BrodlerConfig::default_preview())
	}
}

impl CharacterComponents for Brodler {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(BrodlerPose.resolve()),
			humanoid::orthograde_head_rig(),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = Layers::from_labeled("body", vec![humanoid::body_mesh(BodyMesh::Standard)]);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(self.head.label(), self.head.path().as_str())],
		);
		let mut features = vec![
			humanoid::eye_left(self.eye),
			humanoid::eye_right(self.eye),
			humanoid::nose(self.nose),
			humanoid::mouth(self.mouth),
			humanoid::ear_left(self.ear),
			humanoid::ear_right(self.ear),
			humanoid::horns(self.horns.label(), self.horns.path().as_str()),
		];
		if let Some(hair) = humanoid::hair(self.hair) {
			features.push(hair);
		}
		out.extend_labeled("features", features);
		out
	}
}

impl BrodlerConfig {
	/// Semantic layer: the root [`Brodler`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let brodler = Brodler::from_config(self);
		bsn! { template_value(brodler) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = BrodlerAssets::resolve(self);
		let colors = self.colors.clone();
		common_bsn::assembly_visual_scene::<M>(
			&assembly,
			|part| part.asset.normalization.transform(),
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

fn part_color(colors: &BrodlerColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::Horns => colors.horns.color(),
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		CharacterPartSlot::Mouth => colors.mouth.color(),
		CharacterPartSlot::Hair => colors.hair.color(),
		_ => colors.skin.color(),
	}
}
