//! BSN scenes for Dui.
//!
//! `data_scene()` carries the semantic [`Dui`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::{DuiAssets, DuiEyeMesh, DuiHeadMesh, DuiMouthMesh, DuiNoseMesh},
	pose::DuiPose,
	DuiColors, DuiConfig,
};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	assets::AssetNormalization,
	components::CharacterComponents,
	layer::Layers,
	nodes::{PartNode, RigNode},
	species::common::{
		bsn::{self as common_bsn, WithBaseColor},
		nodes as humanoid, HairMesh,
	},
};
use lod::gen::LodSceneLevel;

/// Semantic Dui data attached to the character root entity.
///
/// Clothing is a higher-order wrapper ([`crate::Clothed`]) via
/// [`DuiConfig::clothed`]. The inner recipe does not emit clothing parts.
#[derive(Component, Clone, PartialEq)]
pub struct Dui {
	pub nose: DuiNoseMesh,
	pub hair: HairMesh,
	pub colors: DuiColors,
}

impl Dui {
	pub fn from_config(config: &DuiConfig) -> Self {
		Self { nose: config.nose, hair: config.hair, colors: config.colors.clone() }
	}
}

impl Default for Dui {
	fn default() -> Self {
		Self::from_config(&DuiConfig::default_preview())
	}
}

fn dui_eye_local() -> Transform {
	Transform::from_translation(Vec3::new(0.0, -0.1, 0.05))
}

impl CharacterComponents for Dui {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_free(vec![
			humanoid::humanoid_body_rig(DuiPose.resolve()),
			humanoid::orthograde_head_rig_at(AssetNormalization::base_y(0.4), Transform::IDENTITY),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = Layers::from_labeled(
			"body",
			vec![humanoid::body_part("igeo", "characters/bodies/igeo_biped_full_body.glb")],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				DuiHeadMesh::BarredBowl.label(),
				DuiHeadMesh::BarredBowl.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::head_feature(
				CharacterPartSlot::EyeLeft,
				DuiEyeMesh::Thorn.label(),
				DuiEyeMesh::Thorn.path().as_str(),
				AssetNormalization::centroid(0.6),
				"eye_socket.L",
				dui_eye_local(),
			),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EyeRight,
				DuiEyeMesh::Thorn.label(),
				DuiEyeMesh::Thorn.path().as_str(),
				AssetNormalization::centroid(0.6),
				"eye_socket.R",
				dui_eye_local(),
			),
			humanoid::head_feature(
				CharacterPartSlot::Mouth,
				DuiMouthMesh::SmallCommon.label(),
				DuiMouthMesh::SmallCommon.path().as_str(),
				AssetNormalization::centroid(0.08),
				"mouth_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, -0.01)),
			),
		];
		if self.nose != DuiNoseMesh::None {
			features.push(humanoid::head_feature(
				CharacterPartSlot::Nose,
				self.nose.label(),
				self.nose.path().as_str(),
				AssetNormalization::centroid(0.05),
				"nose_socket",
				humanoid::nose_socket_local(),
			));
		}
		if let Some(hair) = humanoid::hair(self.hair) {
			features.push(hair);
		}
		out.extend_labeled("features", features);
		out
	}
}

impl DuiConfig {
	/// Semantic layer: the root [`Dui`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let dui = Dui::from_config(self);
		bsn! { template_value(dui) }
	}

	/// Visual layer: body rig plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = DuiAssets::resolve(self);
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

fn part_color(colors: &DuiColors, part: &ResolvedCharacterPart) -> Color {
	match part.slot {
		CharacterPartSlot::Nose => colors.nose_color.color(),
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes.color(),
		CharacterPartSlot::Mouth => colors.mouth.color(),
		CharacterPartSlot::Hair => colors.hair.color(),
		_ => colors.skin.color(),
	}
}
