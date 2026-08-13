//! BSN scenes for Spibmom.
//!
//! `data_scene()` carries the semantic [`Spibmom`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::{SpibmomAssets, SpibmomCrownMesh, SpibmomHeadMesh, SpibmomMouthMesh},
	pose::SpibmomPose,
	SpibmomColors, SpibmomConfig,
};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	assets::AssetNormalization,
	components::CharacterComponents,
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{
		bsn::{self as common_bsn, WithBaseColor},
		nodes as humanoid, EyeMesh, HairMesh, EAR_ROUND,
	},
};
use lod::gen::LodSceneLevel;

const HEAD_RIG_SOCKET_SCALE: f32 = 2.0;

/// Semantic Spibmom data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`SpibmomConfig::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Spibmom {
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: SpibmomColors,
}

impl Spibmom {
	pub fn from_config(config: &SpibmomConfig) -> Self {
		Self { eye: config.eye, hair: config.hair, colors: config.colors.clone() }
	}
}

impl Default for Spibmom {
	fn default() -> Self {
		Self::from_config(&SpibmomConfig::default_preview())
	}
}

fn spibmom_eye_local() -> Transform {
	Transform::from_translation(Vec3::new(0.0, -0.15, -0.12))
}

fn spibmom_ear_left_local() -> Transform {
	Transform::from_translation(Vec3::new(0.15, 0.3, -0.05))
		.with_rotation(Quat::from_rotation_y(std::f32::consts::PI / 4.0))
}

fn spibmom_ear_right_local() -> Transform {
	Transform::from_translation(Vec3::new(-0.15, 0.3, -0.05))
		.with_rotation(Quat::from_rotation_y(-std::f32::consts::PI / 4.0))
}

impl CharacterComponents for Spibmom {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(SpibmomPose.resolve()),
			humanoid::orthograde_head_rig_at(
				AssetNormalization::base_y(0.26),
				Transform::from_scale(Vec3::splat(HEAD_RIG_SOCKET_SCALE)),
			),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = Layers::from_labeled(
			"body",
			vec![
				humanoid::body_part("wumbus", "characters/bodies/wumbus_biped_full_body.glb"),
				humanoid::spine(
					"snail-back",
					"characters/spines/snail_back_full_exo.glb",
					"upper_back",
				)
				.with_normalization(AssetNormalization::base_y(1.4)),
			],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				SpibmomHeadMesh::Meerkat.label(),
				SpibmomHeadMesh::Meerkat.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::head_feature(
				CharacterPartSlot::EyeLeft,
				self.eye.label(),
				self.eye.path().as_str(),
				AssetNormalization::centroid(0.2),
				"eye_socket.L",
				spibmom_eye_local(),
			),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EyeRight,
				self.eye.label(),
				self.eye.path().as_str(),
				AssetNormalization::centroid(0.2),
				"eye_socket.R",
				spibmom_eye_local(),
			),
			humanoid::head_feature(
				CharacterPartSlot::Nose,
				SpibmomMouthMesh::Trunkish.label(),
				SpibmomMouthMesh::Trunkish.path().as_str(),
				AssetNormalization::centroid(0.2),
				"nose_socket",
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.1)),
			),
			humanoid::head_feature(
				CharacterPartSlot::EarLeft,
				"round",
				EAR_ROUND.as_str(),
				AssetNormalization::centroid(0.4),
				"ear_socket.L",
				spibmom_ear_left_local(),
			),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EarRight,
				"round",
				EAR_ROUND.as_str(),
				AssetNormalization::centroid(0.4),
				"ear_socket.R",
				spibmom_ear_right_local(),
			),
			humanoid::head_feature(
				CharacterPartSlot::Horns,
				SpibmomCrownMesh::Finbone.label(),
				SpibmomCrownMesh::Finbone.path().as_str(),
				AssetNormalization::centroid(1.2),
				"crown_socket",
				humanoid::crown_socket_local(),
			),
		];
		if let Some(hair) = humanoid::hair(self.hair) {
			features.push(hair);
		}
		out.extend_labeled("features", features);
		out
	}
}

impl SpibmomConfig {
	/// Semantic layer: the root [`Spibmom`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let spibmom = Spibmom::from_config(self);
		bsn! { template_value(spibmom) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = SpibmomAssets::resolve(self);
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

fn part_color(colors: &SpibmomColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::Horns => colors.crown.color(),
		CharacterPartSlot::Spine => colors.spine.color(),
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => colors.ears.color(),
		CharacterPartSlot::Mouth => colors.mouth.color(),
		CharacterPartSlot::Hair => colors.hair.color(),
		_ => colors.skin.color(),
	}
}
