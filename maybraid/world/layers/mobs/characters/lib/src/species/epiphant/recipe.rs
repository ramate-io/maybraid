//! LodScene recipe for Epiphant.
//!
//! [`Epiphant`] is the inner [`CharacterComponents`] value. Clothing is
//! [`crate::Clothed`] via [`crate::CharacterRecipe::clothed`].

use bevy::prelude::*;

use super::{
	assets::HEAD_RIG_SOCKET_SCALE, pose::EpiphantPose, sliders::EpiphantSliders, EpiphantColors,
	EpiphantConfig,
};
use crate::{
	assembly::CharacterPartSlot,
	assets::AssetNormalization,
	components::{CharacterComponents, LocomotionCapsule},
	layer::Layers,
	nodes::{PartNode, RigNode},
	presets::{BuildPreset, GenderPreset},
	socket::RigId,
	species::{
		common::{nodes as humanoid, EyeMesh, TAIL_CAT},
		epiphant::assets::{EpiphantBodyMesh, EpiphantEarMesh, EpiphantHeadMesh, EpiphantNoseMesh},
	},
};
use character_shaders::SkinTattoo;
use lod::scene::LodSceneLevel;

/// Semantic Epiphant data attached to the character root entity.
///
/// This species has no clothing catalog; [`crate::CharacterRecipe::clothed`] wraps the
/// inner recipe with an empty clothing layer list.
#[derive(Component, Clone, PartialEq)]
pub struct Epiphant {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub body: EpiphantBodyMesh,
	pub head: EpiphantHeadMesh,
	pub ear: EpiphantEarMesh,
	pub nose: EpiphantNoseMesh,
	pub eye: EyeMesh,
	pub colors: EpiphantColors,
	pub sliders: EpiphantSliders,
	pub tattoo: SkinTattoo,
}

impl Epiphant {
	pub fn from_config(config: &EpiphantConfig) -> Self {
		Self {
			gender: config.gender,
			build: config.build,
			body: config.body,
			head: config.head,
			ear: config.ear,
			nose: config.nose,
			eye: config.eye,
			colors: config.colors.clone(),
			sliders: config.sliders,
			tattoo: config.tattoo,
		}
	}
}

impl Default for Epiphant {
	fn default() -> Self {
		Self::from_config(&EpiphantConfig::default_preview())
	}
}

impl CharacterComponents for Epiphant {
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		let pose = EpiphantPose {
			gender: self.gender,
			build: self.build,
			sliders: self.sliders.clamped(),
		};
		LocomotionCapsule::quadruped_for_limb_length(pose.rest_limb_scale())
			.with_pose_girdle(&pose.resolve())
	}

	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		let pose = EpiphantPose {
			gender: self.gender,
			build: self.build,
			sliders: self.sliders.clamped(),
		};
		Layers::from_free(vec![
			humanoid::quadruped_body_rig(pose.resolve()),
			humanoid::pronograde_head_rig(
				AssetNormalization::base_y(0.3),
				RigId::Body,
				"head_socket",
				Transform::from_scale(Vec3::splat(HEAD_RIG_SOCKET_SCALE)),
			),
		])
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		let sliders = self.sliders.clamped();
		let mut out = Layers::from_labeled(
			"body",
			vec![
				humanoid::body_part(self.body.label(), self.body.path().as_str()),
				humanoid::tail("cat-tail", TAIL_CAT.as_str(), "tailbone"),
			],
		);
		out.extend_labeled(
			"head",
			vec![humanoid::head_mesh(self.head.label(), self.head.path().as_str())],
		);
		out.extend_labeled(
			"features",
			vec![
				humanoid::head_feature(
					CharacterPartSlot::EyeLeft,
					self.eye.label(),
					self.eye.path().as_str(),
					AssetNormalization::centroid(0.35),
					"eye_socket.L",
					Transform::from_translation(Vec3::new(0.0, -0.3, -0.2)),
				)
				.with_feature(sliders.feature_transform(CharacterPartSlot::EyeLeft)),
				humanoid::reflected_head_feature(
					CharacterPartSlot::EyeRight,
					self.eye.label(),
					self.eye.path().as_str(),
					AssetNormalization::centroid(0.3),
					"eye_socket.R",
					Transform::from_translation(Vec3::new(0.0, -0.35, -0.2)),
				)
				.with_feature(sliders.feature_transform(CharacterPartSlot::EyeRight)),
				humanoid::head_feature(
					CharacterPartSlot::Nose,
					self.nose.label(),
					self.nose.path().as_str(),
					AssetNormalization::centroid(0.6),
					"nose_socket",
					Transform::from_translation(Vec3::new(0.0, -0.1, 0.1)),
				)
				.with_feature(sliders.feature_transform(CharacterPartSlot::Nose)),
				humanoid::head_feature(
					CharacterPartSlot::EarLeft,
					self.ear.label(),
					self.ear.path().as_str(),
					AssetNormalization::centroid(0.6),
					"ear_socket.L",
					Transform::from_translation(Vec3::new(-0.4, 0.15, -0.05)),
				)
				.with_feature(sliders.feature_transform(CharacterPartSlot::EarLeft)),
				humanoid::reflected_head_feature(
					CharacterPartSlot::EarRight,
					self.ear.label(),
					self.ear.path().as_str(),
					AssetNormalization::centroid(0.6),
					"ear_socket.R",
					Transform::from_translation(Vec3::new(0.4, 0.15, -0.05)),
				)
				.with_feature(sliders.feature_transform(CharacterPartSlot::EarRight)),
			],
		);
		out.map(|part| {
			let color = self.colors.color_for_slot(part.slot);
			let part = part.with_base_color(color);
			match part.slot {
				CharacterPartSlot::BodyMesh
				| CharacterPartSlot::HeadMesh
				| CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Nose => part.with_skin_tattoo(&self.tattoo),
				_ => part,
			}
		})
	}
}

const _: EpiphantBodyMesh = EpiphantBodyMesh::Epiphant;
const _: EpiphantHeadMesh = EpiphantHeadMesh::Meerkat;
const _: EpiphantEarMesh = EpiphantEarMesh::Epiphant;
const _: EpiphantNoseMesh = EpiphantNoseMesh::Trunkish;

#[cfg(test)]
mod tests {
	use character_shaders::{
		RECIPE_SKIN_TATTOO_BODY, RECIPE_SKIN_TATTOO_EAR, RECIPE_SKIN_TATTOO_HEAD,
		RECIPE_SKIN_TATTOO_TRUNK,
	};
	use lod::scene::LodSceneLevel;
	use material_ref::MaterialId;

	use super::*;
	use crate::assembly::CharacterPartSlot;

	#[test]
	fn body_head_ear_trunk_stamp_tattoo_recipes_tail_does_not() -> anyhow::Result<()> {
		let parts = Epiphant::default().part_nodes_for_level(LodSceneLevel::High).flatten();
		let mut saw_body = false;
		let mut saw_head = false;
		let mut saw_ear = false;
		let mut saw_trunk = false;
		for part in parts {
			match part.slot {
				CharacterPartSlot::BodyMesh => {
					saw_body = true;
					assert_eq!(part.material.name, MaterialId::named(RECIPE_SKIN_TATTOO_BODY));
					assert_eq!(part.material.palette.len(), 2);
				}
				CharacterPartSlot::HeadMesh => {
					saw_head = true;
					assert_eq!(part.material.name, MaterialId::named(RECIPE_SKIN_TATTOO_HEAD));
				}
				CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
					saw_ear = true;
					assert_eq!(part.material.name, MaterialId::named(RECIPE_SKIN_TATTOO_EAR));
				}
				CharacterPartSlot::Nose => {
					saw_trunk = true;
					assert_eq!(part.material.name, MaterialId::named(RECIPE_SKIN_TATTOO_TRUNK));
				}
				CharacterPartSlot::Tail => {
					assert_ne!(part.material.name, MaterialId::named(RECIPE_SKIN_TATTOO_BODY));
					assert_ne!(part.material.name, MaterialId::named(RECIPE_SKIN_TATTOO_HEAD));
					assert_ne!(part.material.name, MaterialId::named(RECIPE_SKIN_TATTOO_EAR));
					assert_ne!(part.material.name, MaterialId::named(RECIPE_SKIN_TATTOO_TRUNK));
				}
				_ => {}
			}
		}
		assert!(saw_body && saw_head && saw_ear && saw_trunk, "expected tattooed parts");
		Ok(())
	}
}
