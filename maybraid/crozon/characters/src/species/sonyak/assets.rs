//! Sonyak asset catalog: Gumbus body + Yilter/Dui head stack + thick-braid mane.

use bevy::prelude::*;
use clap::ValueEnum;

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget, SocketAttachment, SocketRig,
	},
	assets::{AssetNormalization, AssetPath},
	species::{
		common::{
			BODY_GUMBUS, HAIR_THICK_BRAIDS, HEAD_RIG, MOUTH_COW_SNOUT, QUADRUPED_RIG, TAIL_CAT,
		},
		sonyak::{pose::SonyakPose, SonyakConfig},
	},
};

const HEAD_BARRED_BOWL: AssetPath = AssetPath::new("characters/heads/barred_bowl_head.glb");
const EYE_THORN: AssetPath = AssetPath::new("characters/horns/single_thorn_left.glb");

/// Species-local resolver for Sonyak asset choices.
pub struct SonyakAssets;

impl SonyakAssets {
	pub fn resolve(config: &SonyakConfig) -> ResolvedCharacterAssembly {
		ResolvedCharacterAssembly::new(
			"Sonyak",
			RigAsset::new("Quadruped", QUADRUPED_RIG),
			SonyakPose::from_config(config).resolve(),
		)
		.with_part(Self::body_mesh())
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh())
		.with_part(Self::eye_left())
		.with_part(Self::eye_right())
		.with_part(Self::mouth(config.mouth))
		.with_part(Self::mane())
		.with_part(Self::tail())
	}

	fn body_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new(
				SonyakBodyMesh::Gumbus.label(),
				BODY_GUMBUS,
				AssetNormalization::IDENTITY,
			),
			SkinTarget::BodyRig,
			None,
		)
	}

	fn head_rig() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadRig,
			CharacterAsset::new(
				"OrthogradeHeadRig",
				HEAD_RIG,
				AssetNormalization::base_y(0.6),
			),
			SkinTarget::OwnRig,
			Some(SocketAttachment {
				rig: SocketRig::Body,
				bone: "head_socket",
				local_transform: Transform::IDENTITY,
			}),
		)
	}

	fn head_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadMesh,
			CharacterAsset::new(
				SonyakHeadMesh::BarredBowl.label(),
				HEAD_BARRED_BOWL,
				AssetNormalization::IDENTITY,
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket("root", Transform::IDENTITY)),
		)
	}

	fn eye_left() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeLeft,
			CharacterAsset::new("thorn", EYE_THORN, AssetNormalization::centroid(0.6)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.L",
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.05)),
			)),
		)
	}

	fn eye_right() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeRight,
			CharacterAsset::new("thorn", EYE_THORN, AssetNormalization::centroid(0.6)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.R",
				Self::mirror_x().with_translation(Vec3::new(0.0, -0.1, 0.05)),
			)),
		)
	}

	fn mouth(mouth: SonyakMouthMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Mouth,
			CharacterAsset::new(mouth.label(), mouth.path(), AssetNormalization::centroid(0.3)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"mouth_socket",
				Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_4))
					.with_translation(Vec3::new(0.0, -0.15, 0.05))
					.with_scale(Vec3::new(4.0, 2.0, 2.0)),
			)),
		)
	}

	fn mane() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Hair,
			CharacterAsset::new("thick-braids", HAIR_THICK_BRAIDS, AssetNormalization::centroid(1.0)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"crown_socket",
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.1)),
			)),
		)
	}

	fn tail() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Tail,
			CharacterAsset::new("cat-tail", TAIL_CAT, AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			Some(Self::body_socket("tailbone", Transform::IDENTITY)),
		)
	}

	fn head_socket(bone: &'static str, local_transform: Transform) -> SocketAttachment {
		SocketAttachment { rig: SocketRig::Head, bone, local_transform }
	}

	fn body_socket(bone: &'static str, local_transform: Transform) -> SocketAttachment {
		SocketAttachment { rig: SocketRig::Body, bone, local_transform }
	}

	fn mirror_x() -> Transform {
		Transform::from_scale(Vec3::new(-1.0, 1.0, 1.0))
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SonyakBodyMesh {
	#[default]
	Gumbus,
}

impl SonyakBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Gumbus];

	pub const fn label(self) -> &'static str {
		"gumbus"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		BODY_GUMBUS
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SonyakHeadMesh {
	#[default]
	BarredBowl,
}

impl SonyakHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::BarredBowl];

	pub const fn label(self) -> &'static str {
		"barred-bowl"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		HEAD_BARRED_BOWL
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SonyakMouthMesh {
	#[default]
	Cow,
}

impl SonyakMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Cow];

	pub const fn label(self) -> &'static str {
		"cow"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		MOUTH_COW_SNOUT
	}
}
