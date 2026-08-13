//! BSN scenes for Hars.
//!
//! `data_scene()` carries the semantic [`Hars`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{assets::HarsAssets, pose::HarsPose, sliders::HarsSliders, HarsColors, HarsConfig};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	assets::AssetNormalization,
	components::CharacterComponents,
	layer::Layers,
	nodes::{PartNode, RigNode},
	presets::{BuildPreset, GenderPreset},
	socket::RigId,
	species::{
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			nodes as humanoid, EarMesh, EyeMesh, EAR_FLANK, TAIL_CAT,
		},
		hars::assets::{HarsBodyMesh, HarsHeadMesh, HarsMouthMesh},
	},
};
use lod::gen::LodSceneLevel;

/// Semantic Hars data attached to the character root entity.
///
/// This species has no clothing catalog; [`HarsConfig::clothed`] wraps the
/// inner recipe with an empty clothing layer list.
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

impl CharacterComponents for Hars {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		let pose =
			HarsPose { gender: self.gender, build: self.build, sliders: self.sliders.clamped() };
		Layers::from_free(vec![
			humanoid::quadruped_body_rig(pose.resolve()),
			humanoid::triple_join_neck_rig(
				pose.neck_pose(),
				Transform::from_translation(Vec3::new(0.0, 0.2, -0.2)),
			),
			humanoid::pronograde_head_rig(
				AssetNormalization::base_y(0.7),
				RigId::Neck,
				"head_socket",
				Transform::IDENTITY,
			),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let sliders = self.sliders.clamped();
		let mut out = Layers::from_labeled(
			"body",
			vec![
				humanoid::body_part(
					HarsBodyMesh::Rumbler.label(),
					HarsBodyMesh::Rumbler.path().as_str(),
				),
				humanoid::tail("cat-tail", TAIL_CAT.as_str(), "tailbone"),
			],
		);
		out.extend_labeled("neck", vec![humanoid::neck_mesh()]);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				HarsHeadMesh::Cowder.label(),
				HarsHeadMesh::Cowder.path().as_str(),
			)],
		);
		out.extend_labeled(
			"features",
			vec![
				humanoid::head_feature(
					CharacterPartSlot::EyeLeft,
					self.eye.label(),
					self.eye.path().as_str(),
					AssetNormalization::centroid(0.4),
					"eye_socket.L",
					Transform::from_translation(Vec3::new(0.2, -0.25, -0.25)),
				)
				.with_feature(sliders.feature_transform(CharacterPartSlot::EyeLeft)),
				humanoid::reflected_head_feature(
					CharacterPartSlot::EyeRight,
					self.eye.label(),
					self.eye.path().as_str(),
					AssetNormalization::centroid(0.4),
					"eye_socket.R",
					Transform::from_translation(Vec3::new(-0.2, -0.25, -0.25)),
				)
				.with_feature(sliders.feature_transform(CharacterPartSlot::EyeRight)),
				humanoid::head_feature(
					CharacterPartSlot::Mouth,
					self.mouth.label(),
					self.mouth.path().as_str(),
					AssetNormalization::centroid(0.3),
					"mouth_socket",
					Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_4))
						.with_translation(Vec3::new(0.0, -0.15, 0.05))
						.with_scale(Vec3::new(4.0, 2.0, 2.0)),
				)
				.with_feature(sliders.feature_transform(CharacterPartSlot::Mouth)),
				humanoid::head_feature(
					CharacterPartSlot::EarLeft,
					EarMesh::Flank.label(),
					EAR_FLANK.as_str(),
					AssetNormalization::centroid(0.4),
					"ear_socket.L",
					Transform::from_translation(Vec3::new(-0.2, 0.0, 0.0)),
				)
				.with_feature(sliders.feature_transform(CharacterPartSlot::EarLeft)),
				humanoid::reflected_head_feature(
					CharacterPartSlot::EarRight,
					EarMesh::Flank.label(),
					EAR_FLANK.as_str(),
					AssetNormalization::centroid(0.4),
					"ear_socket.R",
					Transform::from_translation(Vec3::new(0.2, 0.0, 0.0)),
				)
				.with_feature(sliders.feature_transform(CharacterPartSlot::EarRight)),
			],
		);
		out
	}
}

impl HarsConfig {
	/// Semantic layer: the root [`Hars`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let hars = Hars::from_config(self);
		bsn! { template_value(hars) }
	}

	/// Visual layer: body/neck/head rigs plus resolved parts, colored for material family `M`.
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
