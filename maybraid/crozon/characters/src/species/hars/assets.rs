//! Hars asset catalog for the concepts playground.

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
			quadruped::{self, head_socket_attachment},
			BODY_RUMBLER, EAR_FLANK, HEAD_COWDER, MOUTH_COW_SNOUT, PRONOGRADE_HEAD_RIG,
			QUADRUPED_RIG, TAIL_CAT,
		},
		hars::{pose::{HarsPose, NECK_PITCH}, HarsConfig},
	},
};

pub use crate::species::common::EyeMesh;

/// Species-local resolver for Hars asset choices.
pub struct HarsAssets;

impl HarsAssets {
	pub fn resolve(config: &HarsConfig) -> ResolvedCharacterAssembly {
		ResolvedCharacterAssembly::new(
			"Hars",
			RigAsset::new("Quadruped", QUADRUPED_RIG),
			HarsPose::from_config(config).resolve(),
		)
		.with_part(Self::body_mesh())
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh())
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
				HarsBodyMesh::Rumbler.label(),
				BODY_RUMBLER,
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
				AssetNormalization::base_y(0.4),
			),
			SkinTarget::OwnRig,
			// Counterpose the pitched neck so the head stays level.
			Some(head_socket_attachment(NECK_PITCH)),
		)
	}

	fn head_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadMesh,
			CharacterAsset::new(
				HarsHeadMesh::Cowder.label(),
				HEAD_COWDER,
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
				Transform::from_translation(Vec3::new(0.2, -0.25, -0.25)),
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
				Self::mirror_x().with_translation(Vec3::new(-0.2, -0.25, -0.25)),
			)),
		)
	}

	fn mouth(mouth: HarsMouthMesh) -> ResolvedCharacterPart {
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

	fn ear_left() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarLeft,
			CharacterAsset::new("flank", EAR_FLANK, AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"ear_socket.L",
				Transform::from_translation(Vec3::new(-0.2, 0.0, 0.0)),
			)),
		)
	}

	fn ear_right() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarRight,
			CharacterAsset::new("flank", EAR_FLANK, AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"ear_socket.R",
				Self::mirror_x().with_translation(Vec3::new(0.2, 0.0, 0.0)),
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

// Keep the shared helper referenced so refactors don't drop the pairing.
const _: fn(f32) -> crozon_rigs::BoneRotation = quadruped::neck_pitch_rotation;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum HarsBodyMesh {
	#[default]
	Rumbler,
}

impl HarsBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Rumbler];

	pub const fn label(self) -> &'static str {
		"rumbler"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		BODY_RUMBLER
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum HarsHeadMesh {
	#[default]
	Cowder,
}

impl HarsHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Cowder];

	pub const fn label(self) -> &'static str {
		"cowder"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		HEAD_COWDER
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum HarsMouthMesh {
	#[default]
	Cow,
}

impl HarsMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Cow];

	pub const fn label(self) -> &'static str {
		"cow"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		MOUTH_COW_SNOUT
	}
}
