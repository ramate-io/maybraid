//! Dui asset catalog and assembly resolver.

use bevy::prelude::*;
use clap::ValueEnum;

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget, SocketAttachment, SocketRig,
	},
	assets::{AssetNormalization, AssetPath},
	species::{
		common::assets::{BODY_RIG, HEAD_RIG, MOUTH_STANDARD},
		dui::{pose::DuiPose, DuiConfig},
	},
};

const BODY_IGEO: AssetPath = AssetPath::new("characters/bodies/igeo_biped_full_body.glb");
const HEAD_BARRED_BOWL: AssetPath = AssetPath::new("characters/heads/barred_bowl_head.glb");
const EYE_THORN: AssetPath = AssetPath::new("characters/horns/single_thorn_left.glb");
const NOSE_TBAR: AssetPath = AssetPath::new("characters/noses/tbar_nose.glb");

/// Species-local resolver for Dui asset choices.
pub struct DuiAssets;

impl DuiAssets {
	pub fn resolve(config: &DuiConfig) -> ResolvedCharacterAssembly {
		let mut assembly = ResolvedCharacterAssembly::new(
			"Dui",
			RigAsset::new("Humanoid", BODY_RIG),
			DuiPose.resolve(),
		)
		.with_part(Self::body_mesh())
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh())
		.with_part(Self::eye_left())
		.with_part(Self::eye_right())
		.with_part(Self::mouth());

		if config.nose != DuiNoseMesh::None {
			assembly = assembly.with_part(Self::nose(config.nose));
		}

		let assembly = match Self::hair(config.hair) {
			Some(hair) => assembly.with_part(hair),
			None => assembly,
		};
		config
			.clothing
			.iter()
			.fold(assembly, |assembly, clothing| assembly.with_part(Self::clothing(*clothing)))
	}

	fn body_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new("igeo", BODY_IGEO, AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			None,
		)
	}

	fn head_rig() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadRig,
			CharacterAsset::new("OrthogradeHeadRig", HEAD_RIG, AssetNormalization::base_y(0.4)),
			SkinTarget::OwnRig,
			Some(SocketAttachment {
				rig: SocketRig::Body,
				bone: "upper_neck",
				local_transform: Transform::IDENTITY,
			}),
		)
	}

	fn head_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadMesh,
			CharacterAsset::new(
				DuiHeadMesh::BarredBowl.label(),
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
			CharacterAsset::new(
				DuiEyeMesh::Thorn.label(),
				EYE_THORN,
				AssetNormalization::centroid(0.6),
			),
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
			CharacterAsset::new(
				DuiEyeMesh::Thorn.label(),
				EYE_THORN,
				AssetNormalization::centroid(0.6),
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.R",
				Self::mirror_x().with_translation(Vec3::new(0.0, -0.1, 0.05)),
			)),
		)
	}

	fn nose(nose: DuiNoseMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Nose,
			CharacterAsset::new(nose.label(), nose.path(), AssetNormalization::centroid(0.05)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"nose_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
			)),
		)
	}

	fn mouth() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Mouth,
			CharacterAsset::new(
				DuiMouthMesh::SmallCommon.label(),
				MOUTH_STANDARD,
				AssetNormalization::centroid(0.08),
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"mouth_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, -0.01)),
			)),
		)
	}

	fn hair(hair: crate::species::common::HairMesh) -> Option<ResolvedCharacterPart> {
		let path = hair.path()?;
		Some(ResolvedCharacterPart::new(
			CharacterPartSlot::Hair,
			CharacterAsset::new(hair.label(), path, AssetNormalization::centroid(1.0)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"crown_socket",
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.1)),
			)),
		))
	}

	fn clothing(clothing: crate::species::common::ClothingMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Clothing,
			CharacterAsset::new(clothing.label(), clothing.path(), AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			None,
		)
	}

	fn head_socket(bone: &'static str, local_transform: Transform) -> SocketAttachment {
		SocketAttachment { rig: SocketRig::Head, bone, local_transform }
	}

	fn mirror_x() -> Transform {
		Transform::from_scale(Vec3::new(-1.0, 1.0, 1.0))
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum DuiHeadMesh {
	#[default]
	BarredBowl,
}

impl DuiHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::BarredBowl];

	pub const fn label(self) -> &'static str {
		"barred-bowl"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_BARRED_BOWL
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum DuiEyeMesh {
	#[default]
	Thorn,
}

impl DuiEyeMesh {
	pub const VALUES: &'static [Self] = &[Self::Thorn];

	pub const fn label(self) -> &'static str {
		"thorn"
	}

	pub const fn path(self) -> AssetPath {
		EYE_THORN
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum DuiNoseMesh {
	#[default]
	None,
	Tbar,
}

impl DuiNoseMesh {
	pub const VALUES: &'static [Self] = &[Self::None, Self::Tbar];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::Tbar => "tbar",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::None => NOSE_TBAR,
			Self::Tbar => NOSE_TBAR,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum DuiMouthMesh {
	#[default]
	SmallCommon,
}

impl DuiMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::SmallCommon];

	pub const fn label(self) -> &'static str {
		"small-common"
	}

	pub const fn path(self) -> AssetPath {
		MOUTH_STANDARD
	}
}
