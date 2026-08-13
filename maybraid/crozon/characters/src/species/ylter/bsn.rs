//! BSN scenes for Yilter.
//!
//! `data_scene()` carries the semantic [`Yilter`] root component (including
//! colors), `visual_scene()` composes the rig/part scenes, and `scene()`
//! layers the two for higher-order consumers.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use super::{
	assets::{YilterAssets, EYE_THORN},
	pose::YilterPose,
	sliders::YilterSliders,
	YilterColors, YilterConfig,
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
			nodes as humanoid, TAIL_CAT,
		},
		ylter::assets::{YilterBodyMesh, YilterHeadMesh, YilterMouthMesh},
	},
};
use lod::gen::LodSceneLevel;

/// Semantic Yilter data attached to the character root entity.
///
/// This species has no clothing catalog; [`YilterConfig::clothed`] wraps the
/// inner recipe with an empty clothing layer list.
#[derive(Component, Clone, PartialEq)]
pub struct Yilter {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub mouth: YilterMouthMesh,
	pub colors: YilterColors,
	pub sliders: YilterSliders,
}

impl Yilter {
	pub fn from_config(config: &YilterConfig) -> Self {
		Self {
			gender: config.gender,
			build: config.build,
			mouth: config.mouth,
			colors: config.colors.clone(),
			sliders: config.sliders,
		}
	}
}

impl Default for Yilter {
	fn default() -> Self {
		Self::from_config(&YilterConfig::default_preview())
	}
}

impl CharacterComponents for Yilter {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		let pose =
			YilterPose { gender: self.gender, build: self.build, sliders: self.sliders.clamped() };
		Layers::from_free(vec![
			humanoid::quadruped_body_rig(pose.resolve()),
			humanoid::triple_join_neck_rig(
				pose.neck_pose(),
				Transform::from_translation(Vec3::new(0.0, 0.2, -0.2)),
			),
			humanoid::orthograde_head_rig_at(AssetNormalization::base_y(1.2), Transform::IDENTITY)
				.socketed(SocketRef::on(RigId::Neck, "head_socket")),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let sliders = self.sliders.clamped();
		let mut out = Layers::from_labeled(
			"body",
			vec![
				humanoid::body_part(
					YilterBodyMesh::Rumbler.label(),
					YilterBodyMesh::Rumbler.path().as_str(),
				),
				humanoid::tail("cat-tail", TAIL_CAT.as_str(), "tailbone"),
			],
		);
		out.extend_labeled("neck", vec![humanoid::neck_mesh()]);
		out.extend_labeled(
			"head",
			vec![PartNode::glb(
				CharacterPartSlot::HeadMesh,
				YilterHeadMesh::BarredBowl.label(),
				YilterHeadMesh::BarredBowl.path().as_str(),
				AssetNormalization::base_y(1.2),
			)
			.on_head("root", Transform::IDENTITY)],
		);
		out.extend_labeled(
			"features",
			vec![
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
						.with_translation(Vec3::new(0.0, -0.15, -0.1))
						.with_scale(Vec3::new(4.0, 2.0, 2.0)),
				)
				.with_feature(sliders.feature_transform(CharacterPartSlot::Mouth)),
			],
		);
		out
	}
}

impl YilterConfig {
	/// Semantic layer: the root [`Yilter`] component only.
	pub fn data_scene(&self) -> impl Scene {
		let ylter = Yilter::from_config(self);
		bsn! { template_value(ylter) }
	}

	/// Visual layer: body/neck/head rigs plus resolved parts, colored for material family `M`.
	pub fn visual_scene<M: WithBaseColor>(&self) -> impl Scene {
		let assembly = YilterAssets::resolve(self);
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

fn part_color(colors: &YilterColors, part: &ResolvedCharacterPart) -> Color {
	let item = match part.slot {
		CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => colors.head,
		CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => colors.eyes,
		CharacterPartSlot::Mouth => colors.mouth,
		CharacterPartSlot::Tail => colors.tail,
		CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => colors.neck,
		CharacterPartSlot::BodyMesh => colors.body,
		_ => colors.body,
	};
	item.color()
}

const _: YilterBodyMesh = YilterBodyMesh::Rumbler;
const _: YilterHeadMesh = YilterHeadMesh::BarredBowl;
const _: YilterMouthMesh = YilterMouthMesh::Cow;
