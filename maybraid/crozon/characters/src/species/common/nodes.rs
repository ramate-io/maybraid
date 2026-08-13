//! Shared LodScene node builders for orthograde humanoid head/body layouts.
//!
//! Socket locals are bone-local placement. Left-authored GLBs on `.R` sockets
//! use [`scene_ref::SceneRef::reflected`] for hierarchy handedness — do not put
//! `scale.x = -1` on the socket local.

use bevy::prelude::*;
use scene_ref::MirrorAxis;

use crate::{
	assembly::CharacterPartSlot,
	assets::AssetNormalization,
	nodes::{PartNode, RigNode},
	socket::{RigId, SocketRef},
	ResolvedRigPose,
};

use super::{BodyMesh, EarMesh, EyeMesh, HairMesh, MouthMesh, NoseMesh, BODY_RIG, HEAD_RIG};

pub fn eye_socket_local() -> Transform {
	Transform::from_translation(Vec3::new(0.0, -0.1, -0.075))
}

pub fn nose_socket_local() -> Transform {
	Transform::from_translation(Vec3::new(0.0, 0.0, 0.1))
}

pub fn mouth_socket_local() -> Transform {
	Transform::from_translation(Vec3::new(0.0, 0.0, 0.1))
}

pub fn ear_left_socket_local() -> Transform {
	Transform::from_translation(Vec3::new(0.1, -0.1, 0.00))
		.with_rotation(Quat::from_rotation_y(std::f32::consts::PI / 4.0))
}

/// Right-socket placement (`+X` on `ear_socket.R` points inward).
pub fn ear_right_socket_local() -> Transform {
	Transform::from_translation(Vec3::new(-0.1, -0.1, 0.00))
		.with_rotation(Quat::from_rotation_y(-std::f32::consts::PI / 4.0))
}

pub fn crown_socket_local() -> Transform {
	Transform::from_translation(Vec3::new(0.0, -0.1, 0.1))
}

pub fn humanoid_body_rig(pose: ResolvedRigPose) -> RigNode {
	RigNode::body("Humanoid", BODY_RIG.as_str()).with_pose(pose)
}

pub fn orthograde_head_rig() -> RigNode {
	RigNode::head("OrthogradeHeadRig", HEAD_RIG.as_str())
		.with_normalization(AssetNormalization::base_y(0.26))
		.socketed(SocketRef::on(RigId::Body, "upper_neck"))
}

pub fn body_mesh(body: BodyMesh) -> PartNode {
	PartNode::glb(
		CharacterPartSlot::BodyMesh,
		body.label(),
		body.path().as_str(),
		AssetNormalization::IDENTITY,
	)
	.on_body()
}

pub fn head_mesh(label: &'static str, path: impl Into<String>) -> PartNode {
	PartNode::glb(CharacterPartSlot::HeadMesh, label, path, AssetNormalization::IDENTITY)
		.on_head("root", Transform::IDENTITY)
}

pub fn head_feature(
	slot: CharacterPartSlot,
	label: &'static str,
	path: impl Into<String>,
	normalization: AssetNormalization,
	bone: &'static str,
	local: Transform,
) -> PartNode {
	PartNode::glb(slot, label, path, normalization).on_head(bone, local)
}

/// Left-authored GLB on a right socket: vertex mirror plus conjugated instance TRS.
pub fn reflected_head_feature(
	slot: CharacterPartSlot,
	label: &'static str,
	path: impl Into<String>,
	normalization: AssetNormalization,
	bone: &'static str,
	local: Transform,
) -> PartNode {
	head_feature(slot, label, path, normalization, bone, local).reflected(MirrorAxis::X)
}

pub fn eye_left(eye: EyeMesh) -> PartNode {
	head_feature(
		CharacterPartSlot::EyeLeft,
		eye.label(),
		eye.path().as_str(),
		AssetNormalization::centroid(0.16),
		"eye_socket.L",
		eye_socket_local(),
	)
}

pub fn eye_right(eye: EyeMesh) -> PartNode {
	reflected_head_feature(
		CharacterPartSlot::EyeRight,
		eye.label(),
		eye.path().as_str(),
		AssetNormalization::centroid(0.16),
		"eye_socket.R",
		eye_socket_local(),
	)
}

pub fn nose(nose: NoseMesh) -> PartNode {
	head_feature(
		CharacterPartSlot::Nose,
		nose.label(),
		nose.path().as_str(),
		nose.normalization(),
		"nose_socket",
		nose_socket_local(),
	)
}

pub fn mouth(mouth: MouthMesh) -> PartNode {
	head_feature(
		CharacterPartSlot::Mouth,
		mouth.label(),
		mouth.path().as_str(),
		AssetNormalization::centroid(0.12),
		"mouth_socket",
		mouth_socket_local(),
	)
}

pub fn ear_left(ear: EarMesh) -> PartNode {
	head_feature(
		CharacterPartSlot::EarLeft,
		ear.label(),
		ear.path().as_str(),
		AssetNormalization::centroid(0.15),
		"ear_socket.L",
		ear_left_socket_local(),
	)
}

pub fn ear_right(ear: EarMesh) -> PartNode {
	reflected_head_feature(
		CharacterPartSlot::EarRight,
		ear.label(),
		ear.path().as_str(),
		AssetNormalization::centroid(0.15),
		"ear_socket.R",
		ear_right_socket_local(),
	)
}

pub fn hair(hair: HairMesh) -> Option<PartNode> {
	let path = hair.path()?;
	Some(head_feature(
		CharacterPartSlot::Hair,
		hair.label(),
		path.as_str(),
		AssetNormalization::centroid(1.0),
		"crown_socket",
		crown_socket_local(),
	))
}

pub fn horns(label: &'static str, path: impl Into<String>) -> PartNode {
	head_feature(
		CharacterPartSlot::Horns,
		label,
		path,
		AssetNormalization::centroid(0.7),
		"crown_socket",
		crown_socket_local(),
	)
}
