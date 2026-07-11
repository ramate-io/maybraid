//! Croconot asset catalog for the concepts playground.

use bevy::prelude::*;
use clap::ValueEnum;

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget, SocketAttachment, SocketRig,
	},
	assets::AssetNormalization,
	species::{
		common::{
			BODY_DRAGLOON, EAR_FLANK, HEAD_CANINE, HORNS_HARROWED_CROWN, MOUTH_LERODON_SNOUT,
			PRONOGRADE_HEAD_RIG, QUADRUPED_RIG, TAIL_LERODON_QUADRUPED,
		},
		croconot::{pose::CroconotPose, CroconotConfig},
	},
};

pub use crate::species::common::EyeMesh;

/// Lerodon snout scale on the pronograde mouth socket (from Lero, enlarged for Croconot).
const SNOUT_XY_SCALE: f32 = 2.25;
const SNOUT_Z_SCALE: f32 = 6.2;

/// Species-local resolver for Croconot asset choices.
pub struct CroconotAssets;

impl CroconotAssets {
	pub fn resolve(config: &CroconotConfig) -> ResolvedCharacterAssembly {
		let assembly = ResolvedCharacterAssembly::new(
			"Croconot",
			RigAsset::new("Quadruped", QUADRUPED_RIG),
			CroconotPose::from_config(config).resolve(),
		)
		.with_part(Self::body_mesh())
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh())
		.with_part(Self::eye_left(config.eye))
		.with_part(Self::eye_right(config.eye))
		.with_part(Self::mouth())
		.with_part(Self::ear_left())
		.with_part(Self::ear_right())
		.with_part(Self::tail());

		if config.horns != CroconotHornMesh::None {
			assembly.with_part(Self::horns(config.horns))
		} else {
			assembly
		}
	}

	fn body_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new(
				CroconotBodyMesh::Dragloon.label(),
				BODY_DRAGLOON,
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
				"PronogradeHeadRig",
				PRONOGRADE_HEAD_RIG,
				AssetNormalization::base_y(0.2),
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
				CroconotHeadMesh::Canine.label(),
				HEAD_CANINE,
				AssetNormalization::IDENTITY,
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket("root", Transform::IDENTITY)),
		)
	}

	fn eye_left(eye: EyeMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeLeft,
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.L",
				Transform::from_translation(Vec3::new(0.0, -0.05, -0.12)),
			)),
		)
	}

	fn eye_right(eye: EyeMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeRight,
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.R",
				Self::mirror_x().with_translation(Vec3::new(0.0, -0.05, -0.12)),
			)),
		)
	}

	fn mouth() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Mouth,
			CharacterAsset::new(
				CroconotMouthMesh::Lerodon.label(),
				MOUTH_LERODON_SNOUT,
				AssetNormalization::centroid(0.4),
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"mouth_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)).with_scale(Vec3::new(
					SNOUT_XY_SCALE,
					SNOUT_XY_SCALE,
					SNOUT_Z_SCALE,
				)),
			)),
		)
	}

	fn ear_left() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarLeft,
			CharacterAsset::new("flank", EAR_FLANK, AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket("ear_socket.L", Transform::IDENTITY)),
		)
	}

	fn ear_right() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarRight,
			CharacterAsset::new("flank", EAR_FLANK, AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket("ear_socket.R", Self::mirror_x())),
		)
	}

	fn tail() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Tail,
			CharacterAsset::new(
				"lerodon-tail",
				TAIL_LERODON_QUADRUPED,
				AssetNormalization::IDENTITY,
			),
			SkinTarget::BodyRig,
			Some(Self::body_socket(
				"tailbone",
				Transform::from_translation(Vec3::new(0.0, -0.5, 0.0)),
			)),
		)
	}

	fn horns(horns: CroconotHornMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Horns,
			CharacterAsset::new(horns.label(), horns.path(), AssetNormalization::centroid(0.7)),
			SkinTarget::HeadRig,
			Some(Self::head_socket("crown_socket", Transform::IDENTITY)),
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
pub enum CroconotBodyMesh {
	#[default]
	Dragloon,
}

impl CroconotBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Dragloon];

	pub const fn label(self) -> &'static str {
		"dragloon"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		BODY_DRAGLOON
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum CroconotHeadMesh {
	#[default]
	Canine,
}

impl CroconotHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Canine];

	pub const fn label(self) -> &'static str {
		"canine-head"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		HEAD_CANINE
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum CroconotMouthMesh {
	#[default]
	Lerodon,
}

impl CroconotMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Lerodon];

	pub const fn label(self) -> &'static str {
		"lerodon"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		MOUTH_LERODON_SNOUT
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum CroconotHornMesh {
	#[default]
	None,
	HarrowedCrown,
}

impl CroconotHornMesh {
	pub const VALUES: &'static [Self] = &[Self::None, Self::HarrowedCrown];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::HarrowedCrown => "harrowed-crown",
		}
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		match self {
			Self::None => HORNS_HARROWED_CROWN,
			Self::HarrowedCrown => HORNS_HARROWED_CROWN,
		}
	}
}
