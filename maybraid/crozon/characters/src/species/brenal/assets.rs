//! Brenal asset catalog for the concepts playground.

use bevy::prelude::*;
use clap::ValueEnum;

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget, SocketAttachment, SocketRig,
	},
	assets::AssetNormalization,
	species::{
		brenal::{pose::BrenalPose, BrenalConfig},
		common::{
			BODY_GUMBUS, EAR_FLANK, HEAD_CANINE, HORNS_HARROWED_CROWN, PRONOGRADE_HEAD_RIG,
			QUADRUPED_RIG, TAIL_CAT,
		},
	},
};

pub use crate::species::common::EyeMesh;

/// Species-local resolver for Brenal asset choices.
pub struct BrenalAssets;

impl BrenalAssets {
	pub fn resolve(config: &BrenalConfig) -> ResolvedCharacterAssembly {
		let assembly = ResolvedCharacterAssembly::new(
			"Brenal",
			RigAsset::new("Quadruped", QUADRUPED_RIG),
			BrenalPose::from_config(config).resolve(),
		)
		.with_part(Self::body_mesh())
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh())
		.with_part(Self::eye_left(config.eye))
		.with_part(Self::eye_right(config.eye))
		.with_part(Self::ear_left())
		.with_part(Self::ear_right())
		.with_part(Self::tail());

		if config.horns != BrenalHornMesh::None {
			assembly.with_part(Self::horns(config.horns))
		} else {
			assembly
		}
	}

	fn body_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new(
				BrenalBodyMesh::Gumbus.label(),
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

	fn head_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadMesh,
			CharacterAsset::new(
				BrenalHeadMesh::Canine.label(),
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
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.2)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket_protrusion.L",
				Transform::from_translation(Vec3::new(0.0, 0.0, -0.05)),
			)),
		)
	}

	fn eye_right(eye: EyeMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeRight,
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.2)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket_protrusion.R",
				Self::mirror_x().with_translation(Vec3::new(0.0, 0.0, -0.05)),
			)),
		)
	}

	fn ear_left() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarLeft,
			CharacterAsset::new("flank", EAR_FLANK, AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"temple.L",
				Transform::from_translation(Vec3::new(0.1, 0.0, -0.05))
					.with_rotation(Quat::from_rotation_y(std::f32::consts::PI / 4.0)),
			)),
		)
	}

	fn ear_right() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarRight,
			CharacterAsset::new("flank", EAR_FLANK, AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"temple.R",
				Self::mirror_x()
					.with_translation(Vec3::new(-0.1, 0.0, -0.05))
					.with_rotation(Quat::from_rotation_y(-std::f32::consts::PI / 4.0)),
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

	fn horns(horns: BrenalHornMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Horns,
			CharacterAsset::new(horns.label(), horns.path(), AssetNormalization::centroid(0.7)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"forehead",
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.05)),
			)),
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
pub enum BrenalBodyMesh {
	#[default]
	Gumbus,
}

impl BrenalBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Gumbus];

	pub const fn label(self) -> &'static str {
		"gumbus"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		BODY_GUMBUS
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BrenalHeadMesh {
	#[default]
	Canine,
}

impl BrenalHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Canine];

	pub const fn label(self) -> &'static str {
		"canine-head"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		HEAD_CANINE
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BrenalHornMesh {
	#[default]
	None,
	HarrowedCrown,
}

impl BrenalHornMesh {
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
