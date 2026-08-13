//! Braidman asset catalog for the concepts playground.
//!
//! Phase 2 adds hair and clothing through the same resolved-part path as body
//! and head features: hair socketed on the head rig `crown` bone, clothing
//! remapped to the body rig. Clothing is multi-select via `BraidmanConfig::clothing`.

use bevy::prelude::*;

use crate::{
	assembly::{
		CharacterAsset, CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart,
		RigAsset, SkinTarget, SocketAttachment, SocketRig,
	},
	assets::AssetNormalization,
	nodes::{PartNode, RigNode},
	socket::{RigId, SkinRef, SocketRef},
	species::{
		braidman::{pose::BraidmanPose, BraidmanConfig},
		common::{BODY_RIG, HEAD_RIG},
	},
};
use scene_ref::MirrorAxis;

pub use crate::species::common::{
	BodyMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh,
};

/// Species-local resolver for Braidman asset choices.
pub struct BraidmanAssets;

impl BraidmanAssets {
	pub fn resolve(config: &BraidmanConfig) -> ResolvedCharacterAssembly {
		let assembly = ResolvedCharacterAssembly::new(
			"Braidman",
			RigAsset::new("Humanoid", BODY_RIG),
			BraidmanPose::from_config(config).resolve(),
		)
		.with_part(Self::body_mesh(config.body))
		// Head rig is an armature scene, not a head mesh variant selector.
		.with_part(Self::head_rig())
		.with_part(Self::head_mesh(config.head))
		.with_part(Self::eye_left(config.eye))
		.with_part(Self::eye_right(config.eye))
		.with_part(Self::nose(config.nose))
		.with_part(Self::mouth(config.mouth))
		.with_part(Self::ear_left(config.ear))
		.with_part(Self::ear_right(config.ear));

		let assembly = match Self::hair(config.hair) {
			Some(hair) => assembly.with_part(hair),
			None => assembly,
		};
		config.clothing.iter().fold(assembly, |assembly, clothing| {
			assembly.with_part(ResolvedCharacterPart::clothing(*clothing))
		})
	}

	pub fn body_rig_node(config: &BraidmanConfig) -> RigNode {
		Self::body_rig_node_with_pose(BraidmanPose::from_config(config).resolve())
	}

	pub fn body_rig_node_with_pose(pose: crate::ResolvedRigPose) -> RigNode {
		RigNode::body("Humanoid", BODY_RIG.as_str()).with_pose(pose)
	}

	pub fn head_rig_node() -> RigNode {
		RigNode::head("OrthogradeHeadRig", HEAD_RIG.as_str())
			.with_normalization(AssetNormalization::base_y(0.26))
			.socketed(SocketRef::on(RigId::Body, "upper_neck"))
	}

	pub fn body_mesh_node(body: BodyMesh) -> PartNode {
		PartNode::glb(
			CharacterPartSlot::BodyMesh,
			body.label(),
			body.path().as_str(),
			AssetNormalization::IDENTITY,
		)
		.skinned(SkinRef::to(RigId::Body))
	}

	pub fn head_mesh_node(head: HeadMesh) -> PartNode {
		PartNode::glb(
			CharacterPartSlot::HeadMesh,
			head.label(),
			head.path().as_str(),
			AssetNormalization::IDENTITY,
		)
		.socketed(SocketRef::on(RigId::Head, "root"))
		.skinned(SkinRef::to(RigId::Head))
	}

	pub fn eye_left_node(eye: EyeMesh, feature: Transform) -> PartNode {
		PartNode::glb(
			CharacterPartSlot::EyeLeft,
			eye.label(),
			eye.path().as_str(),
			AssetNormalization::centroid(0.16),
		)
		.with_feature(feature)
		.socketed(
			SocketRef::on(RigId::Head, "eye_socket.L")
				.with_local(Transform::from_translation(Vec3::new(0.0, -0.1, -0.075))),
		)
		.skinned(SkinRef::to(RigId::Head))
	}

	pub fn eye_right_node(eye: EyeMesh, feature: Transform) -> PartNode {
		PartNode::glb(
			CharacterPartSlot::EyeRight,
			eye.label(),
			eye.path().as_str(),
			AssetNormalization::centroid(0.16),
		)
		.with_feature(feature)
		.mirrored(MirrorAxis::X)
		.socketed(
			SocketRef::on(RigId::Head, "eye_socket.R")
				.with_local(Transform::from_translation(Vec3::new(0.0, -0.1, -0.075))),
		)
		.skinned(SkinRef::to(RigId::Head))
	}

	pub fn nose_node(nose: NoseMesh, feature: Transform) -> PartNode {
		PartNode::glb(
			CharacterPartSlot::Nose,
			nose.label(),
			nose.path().as_str(),
			nose.normalization(),
		)
		.with_feature(feature)
		.socketed(
			SocketRef::on(RigId::Head, "nose_socket")
				.with_local(Transform::from_translation(Vec3::new(0.0, 0.0, 0.1))),
		)
		.skinned(SkinRef::to(RigId::Head))
	}

	pub fn mouth_node(mouth: MouthMesh, feature: Transform) -> PartNode {
		PartNode::glb(
			CharacterPartSlot::Mouth,
			mouth.label(),
			mouth.path().as_str(),
			AssetNormalization::centroid(0.12),
		)
		.with_feature(feature)
		.socketed(
			SocketRef::on(RigId::Head, "mouth_socket")
				.with_local(Transform::from_translation(Vec3::new(0.0, 0.0, 0.1))),
		)
		.skinned(SkinRef::to(RigId::Head))
	}

	pub fn ear_left_node(ear: EarMesh, feature: Transform) -> PartNode {
		PartNode::glb(
			CharacterPartSlot::EarLeft,
			ear.label(),
			ear.path().as_str(),
			AssetNormalization::centroid(0.15),
		)
		.with_feature(feature)
		.socketed(
			SocketRef::on(RigId::Head, "ear_socket.L").with_local(
				Transform::from_translation(Vec3::new(0.1, -0.1, 0.00))
					.with_rotation(Quat::from_rotation_y(std::f32::consts::PI / 4.0)),
			),
		)
		.skinned(SkinRef::to(RigId::Head))
	}

	pub fn ear_right_node(ear: EarMesh, feature: Transform) -> PartNode {
		PartNode::glb(
			CharacterPartSlot::EarRight,
			ear.label(),
			ear.path().as_str(),
			AssetNormalization::centroid(0.15),
		)
		.with_feature(feature)
		.mirrored(MirrorAxis::X)
		.socketed(
			SocketRef::on(RigId::Head, "ear_socket.R").with_local(
				Transform::from_translation(Vec3::new(-0.1, -0.1, 0.00))
					.with_rotation(Quat::from_rotation_y(-std::f32::consts::PI / 4.0)),
			),
		)
		.skinned(SkinRef::to(RigId::Head))
	}

	pub fn hair_node(hair: HairMesh, feature: Transform) -> Option<PartNode> {
		let path = hair.path()?;
		Some(
			PartNode::glb(
				CharacterPartSlot::Hair,
				hair.label(),
				path.as_str(),
				AssetNormalization::centroid(1.0),
			)
			.with_feature(feature)
			.socketed(
				SocketRef::on(RigId::Head, "crown_socket")
					.with_local(Transform::from_translation(Vec3::new(0.0, -0.1, 0.1))),
			)
			.skinned(SkinRef::to(RigId::Head)),
		)
	}

	fn body_mesh(body: BodyMesh) -> ResolvedCharacterPart {
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
			CharacterAsset::new("OrthogradeHeadRig", HEAD_RIG, AssetNormalization::base_y(0.26)),
			SkinTarget::OwnRig,
			Some(SocketAttachment {
				rig: SocketRig::Body,
				bone: "upper_neck",
				local_transform: Transform::IDENTITY,
			}),
		)
	}

	fn head_mesh(head: HeadMesh) -> ResolvedCharacterPart {
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

	fn head_socket(bone: &'static str, local_transform: Transform) -> SocketAttachment {
		SocketAttachment { rig: SocketRig::Head, bone, local_transform }
	}

	fn mirror_x() -> Transform {
		Transform::from_scale(Vec3::new(-1.0, 1.0, 1.0))
	}
}
