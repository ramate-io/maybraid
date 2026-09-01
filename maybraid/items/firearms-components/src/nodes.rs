//! Firearm domain nodes that implement [`lod::LodScene`].

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Component, Transform, Vec3};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::SceneChunk;
use scene_ref::SceneRef;

use crate::scene_children::maybe_component;
use rigs::{
	ActiveRigPose, AssemblyHost, BindPose, BoneMap, RigKey, RigRoot, SocketRef, SocketRefRoot,
};

/// Semantic slot for a firearm mesh or armature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Component)]
pub enum FirearmPartSlot {
	#[default]
	Body,
	Barrel,
	Grip,
	/// Baked one-mesh concept (skips kit assembly).
	Concept,
}

impl FirearmPartSlot {
	/// Bone name a kit part sockets onto when a receiver rig is present.
	pub const fn socket_bone(self) -> Option<&'static str> {
		match self {
			Self::Barrel => Some("barrel"),
			Self::Grip => Some("grip"),
			Self::Body | Self::Concept => None,
		}
	}
}

/// Authoring IR for a firearm mesh — also the fine-phase host component.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct PartNode {
	pub slot: FirearmPartSlot,
	pub label: &'static str,
	pub scene: SceneRef,
	pub socket: Option<SocketRef>,
}

impl Default for PartNode {
	fn default() -> Self {
		Self { slot: FirearmPartSlot::Body, label: "", scene: SceneRef::default(), socket: None }
	}
}

impl PartNode {
	pub fn glb(slot: FirearmPartSlot, label: &'static str, path: impl Into<String>) -> Self {
		Self {
			slot,
			label,
			scene: SceneRef::glb(path),
			socket: slot.socket_bone().map(SocketRef::bone),
		}
	}

	pub fn body(label: &'static str, path: impl Into<String>) -> Self {
		Self::glb(FirearmPartSlot::Body, label, path)
	}

	pub fn barrel(label: &'static str, path: impl Into<String>) -> Self {
		Self::glb(FirearmPartSlot::Barrel, label, path)
	}

	pub fn grip(label: &'static str, path: impl Into<String>) -> Self {
		Self::glb(FirearmPartSlot::Grip, label, path)
	}

	pub fn concept(label: &'static str, path: impl Into<String>) -> Self {
		Self::glb(FirearmPartSlot::Concept, label, path)
	}

	pub fn socketed(mut self, socket: SocketRef) -> Self {
		self.socket = Some(socket);
		self
	}
}

impl LodScene for PartNode {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		LodSceneCulls::None
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		self.scene.clone().scene()
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(-0.4, -0.2, -0.8), Vec3::new(0.4, 0.4, 0.4))
	}

	fn host_contents(&self, lod_ref: &LodRef) -> impl Scene + 'static
	where
		Self: Component + Clone + Default + Unpin + Sized,
	{
		let _ = lod_ref;
		let node = self.clone();
		let socket = node.socket.map(SocketRefRoot);
		(
			bsn! {
				template_value(node)
				Transform::IDENTITY
				AssemblyHost
			},
			maybe_component(socket),
		)
	}
}

/// Authoring IR for a firearm receiver armature — also the fine-phase host.
///
/// Kit parts socket onto named bones (`barrel`, `grip`, …). When no armature GLB
/// is present yet, omit this node and socket fulfill parents under [`crate::FirearmRoot`].
#[derive(Debug, Clone, PartialEq, Component)]
pub struct RigNode {
	pub label: &'static str,
	pub scene: SceneRef,
}

impl Default for RigNode {
	fn default() -> Self {
		Self { label: "", scene: SceneRef::default() }
	}
}

impl RigNode {
	pub fn receiver(label: &'static str, path: impl Into<String>) -> Self {
		Self { label, scene: SceneRef::glb(path) }
	}
}

impl LodScene for RigNode {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		LodSceneCulls::None
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		self.scene.clone().scene()
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(-0.4, -0.2, -0.8), Vec3::new(0.4, 0.4, 0.4))
	}

	fn host_contents(&self, lod_ref: &LodRef) -> impl Scene + 'static
	where
		Self: Component + Clone + Default + Unpin + Sized,
	{
		let _ = lod_ref;
		let node = self.clone();
		bsn! {
			template_value(node)
			Transform::IDENTITY
			AssemblyHost
			template_value(RigRoot::new(RigKey::named("receiver")))
			template_value(BoneMap::default())
			template_value(ActiveRigPose::default())
			template_value(BindPose::default())
		}
	}
}
