//! Fine-phase mesh / feature [`lod::LodScene`] host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Component, Transform, Vec3};
use bevy::scene::prelude::Scene;
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::SceneChunk;
use scene_ref::{MirrorAxis, SceneRef};

use crate::assembly::CharacterPartSlot;
use crate::assets::AssetNormalization;
use crate::socket::{SkinRef, SocketRef};

/// Authoring IR for a character mesh or feature — also the fine-phase host component.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct PartNode {
	pub slot: CharacterPartSlot,
	pub label: &'static str,
	pub scene: SceneRef,
	pub normalization: AssetNormalization,
	/// Extra local TRS composed with [`Self::normalization`] (feature sliders).
	pub feature: Transform,
	pub socket: Option<SocketRef>,
	pub skin: Option<SkinRef>,
}

impl Default for PartNode {
	fn default() -> Self {
		Self {
			slot: CharacterPartSlot::BodyMesh,
			label: "",
			scene: SceneRef::default(),
			normalization: AssetNormalization::IDENTITY,
			feature: Transform::IDENTITY,
			socket: None,
			skin: None,
		}
	}
}

impl PartNode {
	pub fn glb(
		slot: CharacterPartSlot,
		label: &'static str,
		path: impl Into<String>,
		normalization: AssetNormalization,
	) -> Self {
		Self {
			slot,
			label,
			scene: SceneRef::glb(path),
			normalization,
			feature: Transform::IDENTITY,
			socket: None,
			skin: None,
		}
	}

	pub fn socketed(mut self, socket: SocketRef) -> Self {
		self.socket = Some(socket);
		self
	}

	pub fn skinned(mut self, skin: SkinRef) -> Self {
		self.skin = Some(skin);
		self
	}

	pub fn with_feature(mut self, feature: Transform) -> Self {
		self.feature = feature;
		self
	}

	pub fn mirrored(mut self, axis: MirrorAxis) -> Self {
		self.scene = self.scene.mirrored(axis);
		self
	}

	/// Normalization × feature, applied on the host before socket fulfill.
	pub fn authored_transform(&self) -> Transform {
		self.normalization.transform().mul_transform(self.feature)
	}

	fn content_for_level(&self, _level: LodSceneLevel) -> impl Scene + 'static {
		self.scene.clone().scene()
	}
}

impl LodScene for PartNode {
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
		Aabb3d::from_min_max(Vec3::splat(-0.5), Vec3::splat(0.5))
	}
}
