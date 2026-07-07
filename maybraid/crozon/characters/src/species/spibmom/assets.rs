//! Spibmom asset catalog and assembly resolver.

use bevy::prelude::*;
use clap::ValueEnum;

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget, SocketAttachment, SocketRig,
	},
	assets::{AssetNormalization, AssetPath},
	species::{
		common::assets::{BODY_RIG, EAR_ROUND, HEAD_RIG, HEAD_STANDARD},
		spibmom::{pose::SpibmomPose, SpibmomConfig},
	},
};

const BODY_WUMBUS: AssetPath = AssetPath::new("characters/bodies/wumbus_biped_full_body.glb");
const SPINE_SNAIL_BACK: AssetPath = AssetPath::new("characters/spines/snail_back_full_exo.glb");
const HORNS_FINBONE_CROWN: AssetPath = AssetPath::new("characters/horns/finbone_crown.glb");
const NOSE_TRUNKISH: AssetPath = AssetPath::new("characters/noses/trunkish_nose.glb");

const HEAD_RIG_SOCKET_SCALE: f32 = 2.0;
const EAR_SOCKET_SCALE: f32 = 1.0;

/// Species-local resolver for Spibmom asset choices.
pub struct SpibmomAssets;

impl SpibmomAssets {
	pub fn resolve(config: &SpibmomConfig) -> ResolvedCharacterAssembly {
		let assembly = ResolvedCharacterAssembly::new(
			"Spibmom",
			RigAsset::new("Humanoid", BODY_RIG),
			SpibmomPose.resolve(),
		)
		.with_part(Self::body_mesh())
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh())
		.with_part(Self::eye_left(config.eye))
		.with_part(Self::eye_right(config.eye))
		.with_part(Self::nose())
		.with_part(Self::ear_left())
		.with_part(Self::ear_right())
		.with_part(Self::horns())
		.with_part(Self::spine());

		let assembly = match Self::hair(config.hair) {
			Some(hair) => assembly.with_part(hair),
			None => assembly,
		};
		config.clothing.iter().fold(assembly, |assembly, clothing| {
			assembly.with_part(ResolvedCharacterPart::clothing(*clothing))
		})
	}

	fn body_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::BodyMesh,
			CharacterAsset::new("wumbus", BODY_WUMBUS, AssetNormalization::IDENTITY),
			SkinTarget::BodyRig,
			None,
		)
	}

	fn head_rig() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadRig,
			CharacterAsset::new("OrthogradeHeadRig", HEAD_RIG, AssetNormalization::base_y(0.26)),
			SkinTarget::OwnRig,
			Some(SocketAttachment {
				rig: SocketRig::Body,
				bone: "upper_neck",
				local_transform: Transform::from_scale(Vec3::splat(HEAD_RIG_SOCKET_SCALE)),
			}),
		)
	}

	fn head_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadMesh,
			CharacterAsset::new(
				SpibmomHeadMesh::Meerkat.label(),
				HEAD_STANDARD,
				AssetNormalization::IDENTITY,
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket("root", Transform::IDENTITY)),
		)
	}

	fn eye_left(eye: crate::species::common::EyeMesh) -> ResolvedCharacterPart {
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

	fn eye_right(eye: crate::species::common::EyeMesh) -> ResolvedCharacterPart {
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

	fn nose() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Nose,
			CharacterAsset::new(
				SpibmomMouthMesh::Trunkish.label(),
				NOSE_TRUNKISH,
				AssetNormalization::centroid(0.2),
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"nose_socket",
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.1)),
			)),
		)
	}

	fn ear_left() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarLeft,
			CharacterAsset::new("round", EAR_ROUND, AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"ear_socket.L",
				Transform::from_translation(Vec3::new(0.15, 0.3, -0.05))
					.with_rotation(Quat::from_rotation_y(std::f32::consts::PI / 4.0))
					.with_scale(Vec3::splat(EAR_SOCKET_SCALE)),
			)),
		)
	}

	fn ear_right() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarRight,
			CharacterAsset::new("round", EAR_ROUND, AssetNormalization::centroid(0.4)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"ear_socket.R",
				Self::mirror_x()
					.with_translation(Vec3::new(-0.15, 0.3, -0.05))
					.with_rotation(Quat::from_rotation_y(-std::f32::consts::PI / 4.0))
					.with_scale(Vec3::splat(EAR_SOCKET_SCALE)),
			)),
		)
	}

	fn horns() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Horns,
			CharacterAsset::new(
				SpibmomCrownMesh::Finbone.label(),
				HORNS_FINBONE_CROWN,
				AssetNormalization::centroid(1.2),
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"crown_socket",
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.1)),
			)),
		)
	}

	fn spine() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Spine,
			CharacterAsset::new("snail-back", SPINE_SNAIL_BACK, AssetNormalization::base_y(1.4)),
			SkinTarget::BodyRig,
			Some(Self::body_socket("upper_back", Transform::IDENTITY)),
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
pub enum SpibmomHeadMesh {
	#[default]
	Meerkat,
}

impl SpibmomHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Meerkat];

	pub const fn label(self) -> &'static str {
		"meerkat"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_STANDARD
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SpibmomMouthMesh {
	#[default]
	Trunkish,
}

impl SpibmomMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Trunkish];

	pub const fn label(self) -> &'static str {
		"trunkish"
	}

	pub const fn path(self) -> AssetPath {
		NOSE_TRUNKISH
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SpibmomCrownMesh {
	#[default]
	Finbone,
}

impl SpibmomCrownMesh {
	pub const VALUES: &'static [Self] = &[Self::Finbone];

	pub const fn label(self) -> &'static str {
		"finbone"
	}

	pub const fn path(self) -> AssetPath {
		HORNS_FINBONE_CROWN
	}
}
