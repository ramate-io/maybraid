//! Fine-phase armature [`lod::LodScene`] host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Component, Vec3};
use bevy::scene::prelude::Scene;
use crozon_rigs::ResolvedRigPose;
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::SceneChunk;
use scene_ref::SceneRef;

use crate::assets::AssetNormalization;
use crate::rig::RigSkeletonKind;
use crate::socket::{RigId, SocketRef};

/// Authoring IR for a character armature — also the fine-phase host component.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct RigNode {
	pub id: RigId,
	pub label: &'static str,
	pub scene: SceneRef,
	pub normalization: AssetNormalization,
	pub socket: Option<SocketRef>,
	pub pose: ResolvedRigPose,
	pub skeleton: RigSkeletonKind,
}

impl Default for RigNode {
	fn default() -> Self {
		Self {
			id: RigId::Body,
			label: "",
			scene: SceneRef::default(),
			normalization: AssetNormalization::IDENTITY,
			socket: None,
			pose: ResolvedRigPose::new(),
			skeleton: RigSkeletonKind::Humanoid,
		}
	}
}

impl RigNode {
	pub fn body(label: &'static str, path: impl Into<String>) -> Self {
		Self {
			id: RigId::Body,
			label,
			scene: SceneRef::glb(path),
			normalization: AssetNormalization::IDENTITY,
			socket: None,
			pose: ResolvedRigPose::new(),
			skeleton: RigSkeletonKind::from_body_rig_label(label),
		}
	}

	pub fn head(label: &'static str, path: impl Into<String>) -> Self {
		Self {
			id: RigId::Head,
			label,
			scene: SceneRef::glb(path),
			normalization: AssetNormalization::IDENTITY,
			socket: None,
			pose: ResolvedRigPose::new(),
			skeleton: RigSkeletonKind::Humanoid,
		}
	}

	pub fn neck(label: &'static str, path: impl Into<String>) -> Self {
		Self {
			id: RigId::Neck,
			label,
			scene: SceneRef::glb(path),
			normalization: AssetNormalization::IDENTITY,
			socket: None,
			pose: ResolvedRigPose::new(),
			skeleton: RigSkeletonKind::Neck,
		}
	}

	pub fn with_normalization(mut self, normalization: AssetNormalization) -> Self {
		self.normalization = normalization;
		self
	}

	pub fn with_pose(mut self, pose: ResolvedRigPose) -> Self {
		self.pose = pose;
		self
	}

	pub fn with_skeleton(mut self, skeleton: RigSkeletonKind) -> Self {
		self.skeleton = skeleton;
		self
	}

	pub fn socketed(mut self, socket: SocketRef) -> Self {
		self.socket = Some(socket);
		self
	}

	fn content_for_level(&self, _level: LodSceneLevel) -> impl Scene + 'static {
		self.scene.clone().scene()
	}
}

impl LodScene for RigNode {
	fn scene_lod_level(&self, _lod_ref: &LodRef) -> LodSceneLevel {
		LodSceneLevel::High
	}

	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		LodSceneCulls::None
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.content_for_level(level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(-1.0, 0.0, -1.0), Vec3::new(1.0, 2.5, 1.0))
	}
}
