//! BSN scenes for Kaller.
//!
//! `data_scene()` carries the semantic [`Kaller`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::{KallerAssets, KallerHeadMesh, KallerHornMesh, KallerSnoutMesh},
	pose::{KallerPose, KALLER_OVERALL_SCALE},
	KallerColors, KallerConfig,
};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	assets::AssetNormalization,
	components::CharacterComponents,
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{
		bsn::{self as common_bsn, WithBaseColor},
		nodes as humanoid, EyeMesh, HairMesh,
	},
};
use lod::gen::LodSceneLevel;

/// Semantic Kaller data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`KallerConfig::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Kaller {
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: KallerColors,
}

impl Kaller {
	pub fn from_config(config: &KallerConfig) -> Self {
		Self { eye: config.eye, hair: config.hair, colors: config.colors.clone() }
	}
}

impl Default for Kaller {
	fn default() -> Self {
		Self::from_config(&KallerConfig::default_preview())
	}
}

impl CharacterComponents for Kaller {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(KallerPose.resolve())
				.with_normalization(AssetNormalization::centroid(KALLER_OVERALL_SCALE)),
			humanoid::orthograde_head_rig(),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = Layers::from_labeled(
			"body",
			vec![humanoid::body_part("sparrow", "characters/bodies/sparrow_body.glb")],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				KallerHeadMesh::Meerkat.label(),
				KallerHeadMesh::Meerkat.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::eye_left(self.eye),
			humanoid::eye_right(self.eye),
			humanoid::head_feature(
				CharacterPartSlot::Mouth,
				KallerSnoutMesh::Robrek.label(),
				KallerSnoutMesh::Robrek.path().as_str(),
				AssetNormalization::centroid(0.35),
				"mouth_socket",
				humanoid::mouth_socket_local().with_scale(Vec3::new(1.0, 1.0, 1.15)),
			),
			humanoid::horns(
				KallerHornMesh::HarrowedCrown.label(),
				KallerHornMesh::HarrowedCrown.path().as_str(),
			),
		];
		if let Some(hair) = humanoid::hair_scaled(
			self.hair,
			match self.hair {
				HairMesh::FeatherHawk => 0.4,
				_ => 1.0,
			},
		) {
			features.push(hair);
		}
		out.extend_labeled("features", features);
		out
	}
}

impl KallerConfig {
	/// Semantic layer: the root [`Kaller`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let kaller = Kaller::from_config(self);
		bsn! { template_value(kaller) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = KallerAssets::resolve(self);
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

fn part_color(colors: &KallerColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::Mouth => colors.snout.color(),
		CharacterPartSlot::Horns => colors.crown.color(),
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		_ => colors.plumage.color(),
	}
}
