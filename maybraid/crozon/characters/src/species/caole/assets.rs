//! Caole asset catalog for the concepts playground.

use bevy::prelude::*;
use clap::ValueEnum;

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget, SocketAttachment, SocketRig,
	},
	assets::AssetNormalization,
	species::{
		caole::{pose::CaolePose, CaoleConfig},
		common::{
			BODY_GUMBUS, EAR_FLANK, HEAD_CAOLE, HEAD_COWDER, MOUTH_COW_SNOUT,
			PRONOGRADE_HEAD_RIG, QUADRUPED_RIG, TAIL_CAT,
		},
	},
};

pub use crate::species::common::EyeMesh;

/// Species-local resolver for Caole asset choices.
pub struct CaoleAssets;

impl CaoleAssets {
	pub fn resolve(config: &CaoleConfig) -> ResolvedCharacterAssembly {
		ResolvedCharacterAssembly::new(
			"Caole",
			RigAsset::new("Quadruped", QUADRUPED_RIG),
			CaolePose::from_config(config).resolve(),
		)
		.with_part(Self::body_mesh())
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh(config.head))
		.with_part(Self::eye_left(config.eye))
		.with_part(Self::eye_right(config.eye))
		.with_part(Self::mouth(config.mouth))
		.with_part(Self::ear_left())
		.with_part(Self::ear_right())
		.with_part(Self::tail())
	}

	fn body_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new(
				CaoleBodyMesh::Gumbus.label(),
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

	fn head_mesh(head: CaoleHeadMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadMesh,
			CharacterAsset::new(head.label(), head.path(), AssetNormalization::IDENTITY),
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
				Transform::from_translation(Vec3::new(0.0, 0.0, -0.25)),
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
				Self::mirror_x().with_translation(Vec3::new(0.0, 0.0, -0.25)),
			)),
		)
	}

	fn mouth(mouth: CaoleMouthMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Mouth,
			CharacterAsset::new(mouth.label(), mouth.path(), AssetNormalization::centroid(0.8)),
			SkinTarget::HeadRig,
			Some(Self::head_socket("mouth_socket", Transform::IDENTITY)),
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
pub enum CaoleBodyMesh {
	#[default]
	Gumbus,
}

impl CaoleBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Gumbus];

	pub const fn label(self) -> &'static str {
		"gumbus"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		BODY_GUMBUS
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum CaoleHeadMesh {
	#[default]
	Caole,
	Cowder,
}

impl CaoleHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Caole, Self::Cowder];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Caole => "caole",
			Self::Cowder => "cowder",
		}
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		match self {
			Self::Caole => HEAD_CAOLE,
			Self::Cowder => HEAD_COWDER,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum CaoleMouthMesh {
	#[default]
	Cow,
}

impl CaoleMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Cow];

	pub const fn label(self) -> &'static str {
		"cow"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		MOUTH_COW_SNOUT
	}
}
