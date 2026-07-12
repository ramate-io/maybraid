//! Claber asset catalog for the concepts playground.

use bevy::prelude::*;
use clap::ValueEnum;

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget, SocketAttachment, SocketRig,
	},
	assets::AssetNormalization,
	species::{
		claber::{pose::ClaberPose, ClaberConfig},
		common::{
			BODY_GUMBUS, EAR_FLANK, HEAD_CAOLE, HORNS_HARROWED_CROWN, MOUTH_ROBREK_SNOUT,
			PRONOGRADE_HEAD_RIG, QUADRUPED_RIG, TAIL_LERODON_QUADRUPED,
		},
	},
};

pub use crate::species::common::EyeMesh;

/// Robrek snout on the pronograde mouth socket: wider XY, shorter Z than Croconot's Lerodon.
const SNOUT_XY_SCALE: f32 = 2.9;
const SNOUT_Z_SCALE: f32 = 4.4;

/// Enlarged harrowed crown on the pronograde crown socket.
const CROWN_SCALE: f32 = 1.75;

/// Species-local resolver for Claber asset choices.
pub struct ClaberAssets;

impl ClaberAssets {
	pub fn resolve(config: &ClaberConfig) -> ResolvedCharacterAssembly {
		let assembly = ResolvedCharacterAssembly::new(
			"Claber",
			RigAsset::new("Quadruped", QUADRUPED_RIG),
			ClaberPose::from_config(config).resolve(),
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

		if config.horns != ClaberHornMesh::None {
			assembly.with_part(Self::horns(config.horns))
		} else {
			assembly
		}
	}

	fn body_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new(
				ClaberBodyMesh::Gumbus.label(),
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
				AssetNormalization::base_y(0.35),
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
				ClaberHeadMesh::Caole.label(),
				HEAD_CAOLE,
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
				Transform::from_translation(Vec3::new(0.2, -0.05, -0.3)),
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
				Self::mirror_x().with_translation(Vec3::new(-0.2, -0.05, -0.3)),
			)),
		)
	}

	fn mouth() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Mouth,
			CharacterAsset::new(
				ClaberMouthMesh::Robrek.label(),
				MOUTH_ROBREK_SNOUT,
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
				"haunch_vertical_thickness",
				Transform::from_translation(Vec3::new(0.0, -0.05, -0.05)),
			)),
		)
	}

	fn horns(horns: ClaberHornMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Horns,
			CharacterAsset::new(horns.label(), horns.path(), AssetNormalization::centroid(0.7)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"crown_socket",
				Transform::from_scale(Vec3::splat(CROWN_SCALE))
					.with_translation(Vec3::new(0.0, -0.2, 0.05)),
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
pub enum ClaberBodyMesh {
	#[default]
	Gumbus,
}

impl ClaberBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Gumbus];

	pub const fn label(self) -> &'static str {
		"gumbus"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		BODY_GUMBUS
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ClaberHeadMesh {
	#[default]
	Caole,
}

impl ClaberHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Caole];

	pub const fn label(self) -> &'static str {
		"caole-head"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		HEAD_CAOLE
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ClaberMouthMesh {
	#[default]
	Robrek,
}

impl ClaberMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Robrek];

	pub const fn label(self) -> &'static str {
		"robrek"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		MOUTH_ROBREK_SNOUT
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ClaberHornMesh {
	None,
	#[default]
	HarrowedCrown,
}

impl ClaberHornMesh {
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
