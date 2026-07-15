//! Kaller asset catalog and assembly resolver.

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
			HairMesh, BODY_RIG, HEAD_RIG, HEAD_STANDARD, HORNS_HARROWED_CROWN, MOUTH_ROBREK_SNOUT,
		},
		kaller::{
			pose::{KallerPose, KALLER_OVERALL_SCALE},
			KallerConfig,
		},
	},
};

const BODY_SPARROW: AssetPath = AssetPath::new("characters/bodies/sparrow_body.glb");

/// Species-local resolver for Kaller asset choices.
pub struct KallerAssets;

impl KallerAssets {
	pub fn resolve(config: &KallerConfig) -> ResolvedCharacterAssembly {
		let assembly = ResolvedCharacterAssembly::new(
			"Kaller",
			RigAsset::new("Humanoid", BODY_RIG)
				.with_normalization(AssetNormalization::centroid(KALLER_OVERALL_SCALE)),
			KallerPose.resolve(),
		)
		.with_part(Self::body_mesh())
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh())
		.with_part(Self::eye_left(config.eye))
		.with_part(Self::eye_right(config.eye))
		.with_part(Self::snout())
		.with_part(Self::horns());

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
			CharacterAsset::new("sparrow", BODY_SPARROW, AssetNormalization::IDENTITY),
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
				local_transform: Transform::IDENTITY,
			}),
		)
	}

	fn head_mesh() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::HeadMesh,
			CharacterAsset::new(
				KallerHeadMesh::Meerkat.label(),
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
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.16)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.L",
				Transform::from_translation(Vec3::new(0.0, -0.1, -0.075)),
			)),
		)
	}

	fn eye_right(eye: crate::species::common::EyeMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EyeRight,
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.16)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.R",
				Self::mirror_x().with_translation(Vec3::new(0.0, -0.1, -0.075)),
			)),
		)
	}

	fn snout() -> ResolvedCharacterPart {
		// Mild forward scale so the robrek snout reads on the meerkat head.
		ResolvedCharacterPart::new(
			CharacterPartSlot::Mouth,
			CharacterAsset::new(
				KallerSnoutMesh::Robrek.label(),
				MOUTH_ROBREK_SNOUT,
				AssetNormalization::centroid(0.35),
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"mouth_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.1))
					.with_scale(Vec3::new(1.0, 1.0, 1.15)),
			)),
		)
	}

	fn horns() -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Horns,
			CharacterAsset::new(
				KallerHornMesh::HarrowedCrown.label(),
				HORNS_HARROWED_CROWN,
				AssetNormalization::centroid(0.7),
			),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"crown_socket",
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.1)),
			)),
		)
	}

	fn hair(hair: HairMesh) -> Option<ResolvedCharacterPart> {
		let path = hair.path()?;
		let scale = match hair {
			HairMesh::FeatherHawk => 0.4,
			_ => 1.0,
		};
		Some(ResolvedCharacterPart::new(
			CharacterPartSlot::Hair,
			CharacterAsset::new(hair.label(), path, AssetNormalization::centroid(scale)),
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

	fn mirror_x() -> Transform {
		Transform::from_scale(Vec3::new(-1.0, 1.0, 1.0))
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum KallerHeadMesh {
	#[default]
	Meerkat,
}

impl KallerHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Meerkat];

	pub const fn label(self) -> &'static str {
		"meerkat"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_STANDARD
	}
}

/// Fixed robrek snout — always attached; kept as an enum for menu identity traits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum KallerSnoutMesh {
	#[default]
	Robrek,
}

impl KallerSnoutMesh {
	pub const VALUES: &'static [Self] = &[Self::Robrek];

	pub const fn label(self) -> &'static str {
		"robrek"
	}

	pub const fn path(self) -> AssetPath {
		MOUTH_ROBREK_SNOUT
	}
}

/// Fixed harrowed crown — always attached; kept as an enum for menu identity traits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum KallerHornMesh {
	#[default]
	HarrowedCrown,
}

impl KallerHornMesh {
	pub const VALUES: &'static [Self] = &[Self::HarrowedCrown];

	pub const fn label(self) -> &'static str {
		"harrowed-crown"
	}

	pub const fn path(self) -> AssetPath {
		HORNS_HARROWED_CROWN
	}
}
