//! BSN scenes for Claber.
//!
//! `data_scene()` carries the semantic [`Claber`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::{ClaberAssets, CROWN_SCALE, SNOUT_XY_SCALE, SNOUT_Z_SCALE},
	pose::ClaberPose,
	sliders::ClaberSliders,
	ClaberColors, ClaberConfig,
};
use crate::{
	assembly::{CharacterPartSlot, ResolvedCharacterPart},
	assets::AssetNormalization,
	components::CharacterComponents,
	layer::Layers,
	nodes::{PartNode, RigNode},
	presets::{BuildPreset, GenderPreset},
	socket::{RigId, SocketRef},
	species::{
		claber::assets::{ClaberBodyMesh, ClaberHeadMesh, ClaberHornMesh, ClaberMouthMesh},
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			nodes as humanoid, EarMesh, EyeMesh, EAR_FLANK, TAIL_LERODON_QUADRUPED,
		},
	},
};
use lod::gen::LodSceneLevel;

/// Semantic Claber data attached to the character root entity.
///
/// This species has no clothing catalog; [`ClaberConfig::clothed`] wraps the
/// inner recipe with an empty clothing layer list.
#[derive(Component, Clone, PartialEq)]
pub struct Claber {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub horns: ClaberHornMesh,
	pub eye: EyeMesh,
	pub colors: ClaberColors,
	pub sliders: ClaberSliders,
}

impl Claber {
	pub fn from_config(config: &ClaberConfig) -> Self {
		Self {
			gender: config.gender,
			build: config.build,
			horns: config.horns,
			eye: config.eye,
			colors: config.colors.clone(),
			sliders: config.sliders,
		}
	}
}

impl Default for Claber {
	fn default() -> Self {
		Self::from_config(&ClaberConfig::default_preview())
	}
}

impl CharacterComponents for Claber {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		let pose =
			ClaberPose { gender: self.gender, build: self.build, sliders: self.sliders.clamped() };
		Layers::from_free(vec![
			humanoid::quadruped_body_rig(pose.resolve()),
			humanoid::pronograde_head_rig(
				AssetNormalization::base_y(0.35),
				RigId::Body,
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
					ClaberBodyMesh::Gumbus.label(),
					ClaberBodyMesh::Gumbus.path().as_str(),
				),
				humanoid::tail(
					"lerodon-tail",
					TAIL_LERODON_QUADRUPED.as_str(),
					"haunch_vertical_thickness",
				)
				.socketed(
					SocketRef::on(RigId::Body, "haunch_vertical_thickness")
						.with_local(Transform::from_translation(Vec3::new(0.0, -0.05, -0.05))),
				),
			],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				ClaberHeadMesh::Caole.label(),
				ClaberHeadMesh::Caole.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::head_feature(
				CharacterPartSlot::EyeLeft,
				self.eye.label(),
				self.eye.path().as_str(),
				AssetNormalization::centroid(0.4),
				"eye_socket.L",
				Transform::from_translation(Vec3::new(0.2, -0.05, -0.3)),
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::EyeLeft)),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EyeRight,
				self.eye.label(),
				self.eye.path().as_str(),
				AssetNormalization::centroid(0.4),
				"eye_socket.R",
				Transform::from_translation(Vec3::new(-0.2, -0.05, -0.3)),
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::EyeRight)),
			humanoid::head_feature(
				CharacterPartSlot::Mouth,
				ClaberMouthMesh::Robrek.label(),
				ClaberMouthMesh::Robrek.path().as_str(),
				AssetNormalization::centroid(0.4),
				"mouth_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)).with_scale(Vec3::new(
					SNOUT_XY_SCALE,
					SNOUT_XY_SCALE,
					SNOUT_Z_SCALE,
				)),
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::Mouth)),
			humanoid::head_feature(
				CharacterPartSlot::EarLeft,
				EarMesh::Flank.label(),
				EAR_FLANK.as_str(),
				AssetNormalization::centroid(0.4),
				"ear_socket.L",
				Transform::IDENTITY,
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::EarLeft)),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EarRight,
				EarMesh::Flank.label(),
				EAR_FLANK.as_str(),
				AssetNormalization::centroid(0.4),
				"ear_socket.R",
				Transform::IDENTITY,
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::EarRight)),
		];
		if self.horns != ClaberHornMesh::None {
			features.push(
				humanoid::head_feature(
					CharacterPartSlot::Horns,
					self.horns.label(),
					self.horns.path().as_str(),
					AssetNormalization::centroid(0.7),
					"crown_socket",
					Transform::from_scale(Vec3::splat(CROWN_SCALE))
						.with_translation(Vec3::new(0.0, -0.2, 0.05)),
				)
				.with_feature(sliders.feature_transform(CharacterPartSlot::Horns)),
			);
		}
		out.extend_labeled("features", features);
		out
	}
}

impl ClaberConfig {
	/// Semantic layer: the root [`Claber`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let claber = Claber::from_config(self);
		bsn! { template_value(claber) }
	}

	/// Visual layer: body/head rigs plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = ClaberAssets::resolve(self);
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

fn part_color(colors: &ClaberColors, part: &ResolvedCharacterPart) -> Color {
	let tone = match part.slot {
		CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => colors.head,
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes,
		CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => colors.ears,
		CharacterPartSlot::Mouth => colors.mouth,
		CharacterPartSlot::Horns => colors.horns,
		CharacterPartSlot::Tail => colors.tail,
		CharacterPartSlot::BodyMesh => colors.body,
		_ => colors.body,
	};
	tone.color()
}

// Keep fixed mesh enums referenced for compile-time asset wiring checks.
const _: ClaberBodyMesh = ClaberBodyMesh::Gumbus;
const _: ClaberHeadMesh = ClaberHeadMesh::Caole;
const _: ClaberHornMesh = ClaberHornMesh::HarrowedCrown;
