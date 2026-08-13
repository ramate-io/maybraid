//! BSN scenes for Sonyak.
//!
//! `data_scene()` carries the semantic [`Sonyak`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::{SonyakAssets, EYE_THORN},
	pose::SonyakPose,
	sliders::SonyakSliders,
	SonyakColors, SonyakConfig,
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
		common::{
			bsn::{self as common_bsn, WithBaseColor},
			nodes as humanoid, HairMesh, TAIL_CAT,
		},
		sonyak::assets::{SonyakBodyMesh, SonyakHeadMesh, SonyakMouthMesh},
	},
};
use lod::gen::LodSceneLevel;

/// Semantic Sonyak data attached to the character root entity.
///
/// This species has no clothing catalog; [`SonyakConfig::clothed`] wraps the
/// inner recipe with an empty clothing layer list.
#[derive(Component, Clone, PartialEq)]
pub struct Sonyak {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub mouth: SonyakMouthMesh,
	pub colors: SonyakColors,
	pub sliders: SonyakSliders,
}

impl Sonyak {
	pub fn from_config(config: &SonyakConfig) -> Self {
		Self {
			gender: config.gender,
			build: config.build,
			mouth: config.mouth,
			colors: config.colors.clone(),
			sliders: config.sliders,
		}
	}
}

impl Default for Sonyak {
	fn default() -> Self {
		Self::from_config(&SonyakConfig::default_preview())
	}
}

impl CharacterComponents for Sonyak {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		let pose =
			SonyakPose { gender: self.gender, build: self.build, sliders: self.sliders.clamped() };
		Layers::from_free(vec![
			humanoid::quadruped_body_rig(pose.resolve()),
			humanoid::orthograde_head_rig_at(AssetNormalization::base_y(0.6), Transform::IDENTITY)
				.socketed(SocketRef::on(RigId::Body, "head_socket")),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let sliders = self.sliders.clamped();
		let mut out = Layers::from_labeled(
			"body",
			vec![
				humanoid::body_part(
					SonyakBodyMesh::Gumbus.label(),
					SonyakBodyMesh::Gumbus.path().as_str(),
				),
				humanoid::tail("cat-tail", TAIL_CAT.as_str(), "tailbone"),
			],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(
				SonyakHeadMesh::BarredBowl.label(),
				SonyakHeadMesh::BarredBowl.path().as_str(),
			)],
		);
		let mut features = vec![
			humanoid::head_feature(
				CharacterPartSlot::EyeLeft,
				"thorn",
				EYE_THORN.as_str(),
				AssetNormalization::centroid(0.6),
				"eye_socket.L",
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.05)),
			)
			.with_feature(sliders.feature_transform(CharacterPartSlot::EyeLeft)),
			humanoid::reflected_head_feature(
				CharacterPartSlot::EyeRight,
				"thorn",
				EYE_THORN.as_str(),
				AssetNormalization::centroid(0.6),
				"eye_socket.R",
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.05)),
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
		];
		if let Some(mane) = humanoid::hair(HairMesh::ThickBraids) {
			features.push(mane.with_feature(sliders.feature_transform(CharacterPartSlot::Hair)));
		}
		out.extend_labeled("features", features);
		out
	}
}

impl SonyakConfig {
	/// Semantic layer: the root [`Sonyak`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let sonyak = Sonyak::from_config(self);
		bsn! { template_value(sonyak) }
	}

	/// Visual layer: body/head rigs plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = SonyakAssets::resolve(self);
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

fn part_color(colors: &SonyakColors, part: &ResolvedCharacterPart) -> Color {
	let item = match part.slot {
		CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => colors.head,
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes,
		CharacterPartSlot::Mouth => colors.mouth,
		CharacterPartSlot::Hair => colors.hair,
		CharacterPartSlot::Tail => colors.tail,
		CharacterPartSlot::BodyMesh => colors.body,
		_ => colors.body,
	};
	item.color()
}

const _: SonyakBodyMesh = SonyakBodyMesh::Gumbus;
const _: SonyakHeadMesh = SonyakHeadMesh::BarredBowl;
const _: SonyakMouthMesh = SonyakMouthMesh::Cow;
