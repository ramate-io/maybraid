//! BSN scenes for Mygr.
//!
//! `data_scene()` carries the semantic [`Mygr`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::{MygrAssets, MygrHeadMesh, MygrMouthMesh},
	pose::MygrPose,
	MygrColors, MygrConfig,
};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	assets::AssetNormalization,
	components::CharacterComponents,
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{
		bsn::{self as common_bsn, WithBaseColor},
		nodes as humanoid, EyeMesh, HairMesh, EAR_FLANK, TAIL_CAT,
	},
};
use lod::gen::LodSceneLevel;

/// Semantic Mygr data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`MygrConfig::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Mygr {
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub colors: MygrColors,
}

impl Mygr {
	pub fn from_config(config: &MygrConfig) -> Self {
		Self { eye: config.eye, hair: config.hair, colors: config.colors.clone() }
	}
}

impl Default for Mygr {
	fn default() -> Self {
		Self::from_config(&MygrConfig::default_preview())
	}
}

fn mygr_eye_local() -> Transform {
	Transform::from_translation(Vec3::new(0.0, 0.0, -0.12))
}

fn mygr_ear_left_local() -> Transform {
	Transform::from_translation(Vec3::new(0.15, 0.3, -0.05))
		.with_rotation(Quat::from_rotation_y(std::f32::consts::PI / 4.0))
}

fn mygr_ear_right_local() -> Transform {
	Transform::from_translation(Vec3::new(-0.15, 0.3, -0.05))
		.with_rotation(Quat::from_rotation_y(-std::f32::consts::PI / 4.0))
}

impl CharacterComponents for Mygr {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(MygrPose.resolve()),
			humanoid::orthograde_head_rig(),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = Layers::from_labeled(
			"body",
			vec![
				humanoid::body_part("full", crate::species::common::BODY_FULL.as_str()),
				humanoid::tail("cat-tail", TAIL_CAT.as_str(), "root"),
			],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				MygrHeadMesh::OrthoBear.label(),
				MygrHeadMesh::OrthoBear.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::head_feature(
				CharacterPartSlot::EyeLeft,
				self.eye.label(),
				self.eye.path().as_str(),
				AssetNormalization::centroid(0.2),
				"eye_socket.L",
				mygr_eye_local(),
			),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EyeRight,
				self.eye.label(),
				self.eye.path().as_str(),
				AssetNormalization::centroid(0.2),
				"eye_socket.R",
				mygr_eye_local(),
			),
			humanoid::head_feature(
				CharacterPartSlot::Mouth,
				MygrMouthMesh::CanineSnout.label(),
				MygrMouthMesh::CanineSnout.path().as_str(),
				AssetNormalization::centroid(0.4),
				"mouth_socket",
				humanoid::mouth_socket_local(),
			),
			humanoid::head_feature(
				CharacterPartSlot::EarLeft,
				"flank",
				EAR_FLANK.as_str(),
				AssetNormalization::centroid(0.4),
				"ear_socket.L",
				mygr_ear_left_local(),
			),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EarRight,
				"flank",
				EAR_FLANK.as_str(),
				AssetNormalization::centroid(0.4),
				"ear_socket.R",
				mygr_ear_right_local(),
			),
		];
		if let Some(hair) = humanoid::hair(self.hair) {
			features.push(hair);
		}
		out.extend_labeled("features", features);
		out
	}
}

impl MygrConfig {
	/// Semantic layer: the root [`Mygr`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let mygr = Mygr::from_config(self);
		bsn! { template_value(mygr) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = MygrAssets::resolve(self);
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

fn part_color(colors: &MygrColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		CharacterPartSlot::Mouth => colors.mouth.color(),
		CharacterPartSlot::Hair => colors.hair.color(),
		_ => colors.skin.color(),
	}
}
