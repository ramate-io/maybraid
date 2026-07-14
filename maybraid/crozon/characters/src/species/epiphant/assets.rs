//! Epiphant asset catalog for the concepts playground.

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
			assets::HEAD_STANDARD_PRONOGRADE, PRONOGRADE_HEAD_RIG, QUADRUPED_RIG, TAIL_CAT,
		},
		epiphant::{pose::EpiphantPose, EpiphantConfig},
	},
};

pub use crate::species::common::EyeMesh;

const BODY_EPIPHANT: AssetPath = AssetPath::new("characters/bodies/epiphant.glb");
const EAR_EPIPHANT: AssetPath = AssetPath::new("characters/ears/epiphant_ear_left.glb");
const NOSE_TRUNKISH: AssetPath = AssetPath::new("characters/noses/trunkish_nose.glb");

/// Enlarge the pronograde head stack so the meerkat head reads as "large" on the body.
const HEAD_RIG_SOCKET_SCALE: f32 = 1.45;
/// Epiphant ears are the silhouette cue; bias them larger than flank/round defaults.
const EAR_SOCKET_SCALE: f32 = 1.75;

/// Species-local resolver for Epiphant asset choices.
pub struct EpiphantAssets;

impl EpiphantAssets {
	pub fn resolve(config: &EpiphantConfig) -> ResolvedCharacterAssembly {
		ResolvedCharacterAssembly::new(
			"Epiphant",
			RigAsset::new("Quadruped", QUADRUPED_RIG),
			EpiphantPose::from_config(config).resolve(),
		)
		.with_part(Self::body_mesh(config.body))
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh(config.head))
		.with_part(Self::eye_left(config.eye))
		.with_part(Self::eye_right(config.eye))
		.with_part(Self::nose(config.nose))
		.with_part(Self::ear_left(config.ear))
		.with_part(Self::ear_right(config.ear))
		.with_part(Self::tail())
	}

	fn body_mesh(body: EpiphantBodyMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new(body.label(), body.path(), AssetNormalization::IDENTITY),
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
			Some(SocketAttachment {
				rig: SocketRig::Body,
				bone: "head_socket",
				local_transform: Transform::from_scale(Vec3::splat(HEAD_RIG_SOCKET_SCALE)),
			}),
		)
	}

	fn head_mesh(head: EpiphantHeadMesh) -> ResolvedCharacterPart {
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
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.2)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.L",
				Transform::from_translation(Vec3::new(0.0, -0.15, -0.12)),
			)),
		)
	}

	fn eye_right(eye: EyeMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeRight,
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.2)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.R",
				Self::mirror_x().with_translation(Vec3::new(0.0, -0.15, -0.12)),
			)),
		)
	}

	fn nose(nose: EpiphantNoseMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Nose,
			CharacterAsset::new(nose.label(), nose.path(), AssetNormalization::centroid(0.2)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"nose_socket",
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.1)),
			)),
		)
	}

	fn ear_left(ear: EpiphantEarMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarLeft,
			CharacterAsset::new(ear.label(), ear.path(), AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"ear_socket.L",
				Transform::from_translation(Vec3::new(0.1, 0.15, -0.05))
					.with_scale(Vec3::splat(EAR_SOCKET_SCALE)),
			)),
		)
	}

	fn ear_right(ear: EpiphantEarMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarRight,
			CharacterAsset::new(ear.label(), ear.path(), AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"ear_socket.R",
				Self::mirror_x()
					.with_translation(Vec3::new(-0.1, 0.15, -0.05))
					.with_scale(Vec3::splat(EAR_SOCKET_SCALE)),
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
pub enum EpiphantBodyMesh {
	#[default]
	Epiphant,
}

impl EpiphantBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Epiphant];

	pub const fn label(self) -> &'static str {
		"epiphant"
	}

	pub const fn path(self) -> AssetPath {
		BODY_EPIPHANT
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum EpiphantHeadMesh {
	#[default]
	Meerkat,
}

impl EpiphantHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Meerkat];

	pub const fn label(self) -> &'static str {
		"meerkat"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_STANDARD_PRONOGRADE
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum EpiphantEarMesh {
	#[default]
	Epiphant,
}

impl EpiphantEarMesh {
	pub const VALUES: &'static [Self] = &[Self::Epiphant];

	pub const fn label(self) -> &'static str {
		"epiphant"
	}

	pub const fn path(self) -> AssetPath {
		EAR_EPIPHANT
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum EpiphantNoseMesh {
	#[default]
	Trunkish,
}

impl EpiphantNoseMesh {
	pub const VALUES: &'static [Self] = &[Self::Trunkish];

	pub const fn label(self) -> &'static str {
		"trunkish"
	}

	pub const fn path(self) -> AssetPath {
		NOSE_TRUNKISH
	}
}
