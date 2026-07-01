//! Brodler asset catalog and assembly resolver.

use bevy::prelude::*;
use clap::ValueEnum;

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget, SocketAttachment, SocketRig,
	},
	assets::AssetNormalization,
	species::{
		brodler::{pose::BrodlerPose, BrodlerConfig, BrodlerHeadMesh},
		common::{
			BodyMesh, ClothingMesh, EarMesh, EyeMesh, HairMesh, MouthMesh, NoseMesh, BODY_RIG,
			BODY_STANDARD, HEAD_RIG, HORNS_HARROWED_CROWN, HORNS_LORKEN_CROWN,
		},
	},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum HornMesh {
	#[default]
	HarrowedCrown,
	LorkenCrown,
}

impl HornMesh {
	pub const fn label(self) -> &'static str {
		match self {
			Self::HarrowedCrown => "harrowed-crown",
			Self::LorkenCrown => "lorken-crown",
		}
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		match self {
			Self::HarrowedCrown => HORNS_HARROWED_CROWN,
			Self::LorkenCrown => HORNS_LORKEN_CROWN,
		}
	}
}

/// Species-local resolver for Brodler asset choices.
pub struct BrodlerAssets;

impl BrodlerAssets {
	pub fn resolve(config: &BrodlerConfig) -> ResolvedCharacterAssembly {
		let assembly = ResolvedCharacterAssembly::new(
			"Brodler",
			RigAsset::new("Humanoid", BODY_RIG),
			BrodlerPose.resolve(),
		)
		.with_part(Self::body_mesh())
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh(config.head))
		.with_part(Self::eye_left(config.eye))
		.with_part(Self::eye_right(config.eye))
		.with_part(Self::nose(config.nose))
		.with_part(Self::mouth(config.mouth))
		.with_part(Self::ear_left(config.ear))
		.with_part(Self::ear_right(config.ear))
		.with_part(Self::horns(config.horns));

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
			CharacterAsset::new(
				BodyMesh::Standard.label(),
				BODY_STANDARD,
				AssetNormalization::IDENTITY,
			),
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

	fn head_mesh(head: BrodlerHeadMesh) -> ResolvedCharacterPart {
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
			CharacterAsset::new(eye.label(), eye.path(), AssetNormalization::centroid(0.16)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"eye_socket.L",
				Transform::from_translation(Vec3::new(0.0, -0.1, -0.075)),
			)),
		)
	}

	fn eye_right(eye: EyeMesh) -> ResolvedCharacterPart {
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

	fn nose(nose: NoseMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Nose,
			CharacterAsset::new(nose.label(), nose.path(), nose.normalization()),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"nose_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
			)),
		)
	}

	fn mouth(mouth: MouthMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Mouth,
			CharacterAsset::new(mouth.label(), mouth.path(), AssetNormalization::centroid(0.12)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"mouth_socket",
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
			)),
		)
	}

	fn ear_left(ear: EarMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarLeft,
			CharacterAsset::new(ear.label(), ear.path(), AssetNormalization::centroid(0.15)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"ear_socket.L",
				Transform::from_translation(Vec3::new(0.1, -0.1, 0.00))
					.with_rotation(Quat::from_rotation_y(std::f32::consts::PI / 4.0)),
			)),
		)
	}

	fn ear_right(ear: EarMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::EarRight,
			CharacterAsset::new(ear.label(), ear.path(), AssetNormalization::centroid(0.15)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"ear_socket.R",
				Self::mirror_x()
					.with_translation(Vec3::new(-0.1, -0.1, 0.00))
					.with_rotation(Quat::from_rotation_y(-std::f32::consts::PI / 4.0)),
			)),
		)
	}

	fn horns(horns: HornMesh) -> ResolvedCharacterPart {
		ResolvedCharacterPart::new(
			CharacterPartSlot::Horns,
			CharacterAsset::new(horns.label(), horns.path(), AssetNormalization::centroid(0.7)),
			SkinTarget::HeadRig,
			Some(Self::head_socket(
				"crown_socket",
				Transform::from_translation(Vec3::new(0.0, -0.1, 0.1)),
			)),
		)
	}

	fn hair(hair: HairMesh) -> Option<ResolvedCharacterPart> {
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

	fn clothing(clothing: ClothingMesh) -> ResolvedCharacterPart {
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
