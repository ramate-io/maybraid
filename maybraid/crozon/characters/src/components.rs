//! Domain IR + structural [`lod::LodScene`] host for characters.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::{Commands, CommandsSceneExt, Component, Entity, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use crozon_character_items::{ClothingMesh, ItemColor};
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::{lod_host_scene_pending, SceneChunk};

use crate::assembly::CharacterPartSlot;
use crate::assets::AssetNormalization;
use crate::layer::Layers;
use crate::nodes::{PartNode, RigNode};
use crate::scene_children::scene_children;
use crate::socket::{RigId, SkinRef};

/// Domain IR exposed by a character (or character wrapper) for structural composition.
pub trait CharacterComponents {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::new()
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::new()
	}
}

impl<T: CharacterComponents + ?Sized> CharacterComponents for &T {
	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RigNode> {
		(**self).rig_nodes_for_level(level)
	}

	fn part_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		(**self).part_nodes_for_level(level)
	}
}

/// Clothing layers composed over an inner [`CharacterComponents`] recipe.
#[derive(Debug, Clone, PartialEq)]
pub struct ClothingLayer {
	pub mesh: ClothingMesh,
	pub color: ItemColor,
}

impl ClothingLayer {
	pub fn new(mesh: ClothingMesh, color: ItemColor) -> Self {
		Self { mesh, color }
	}

	pub fn part_node(&self) -> PartNode {
		PartNode::glb(
			CharacterPartSlot::Clothing,
			self.mesh.label(),
			self.mesh.path(),
			AssetNormalization::IDENTITY,
		)
		.skinned(SkinRef::to(RigId::Body))
	}
}

/// Higher-order character: inner recipe plus clothing parts under `"clothing"`.
#[derive(Debug, Clone, PartialEq)]
pub struct Clothed<T> {
	pub inner: T,
	pub clothing: Vec<ClothingLayer>,
}

impl<T> Clothed<T> {
	pub fn new(inner: T, clothing: Vec<ClothingLayer>) -> Self {
		Self { inner, clothing }
	}
}

impl<T: CharacterComponents> CharacterComponents for Clothed<T> {
	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RigNode> {
		self.inner.rig_nodes_for_level(level)
	}

	fn part_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = self.inner.part_nodes_for_level(level);
		if !self.clothing.is_empty() {
			out.extend_under(
				"clothing",
				Layers::from_free(self.clothing.iter().map(ClothingLayer::part_node).collect()),
			);
		}
		out
	}
}

/// Newtype: present a [`CharacterComponents`] value as a structural [`LodScene`] host.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct ComponentsOnly<T: Send + Sync + 'static>(pub T);

impl<T: Send + Sync + 'static> ComponentsOnly<T> {
	pub fn into_inner(self) -> T {
		self.0
	}
}

impl<T: Send + Sync + 'static> From<T> for ComponentsOnly<T> {
	fn from(value: T) -> Self {
		Self(value)
	}
}

impl<T: Send + Sync + 'static> std::ops::Deref for ComponentsOnly<T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.0
	}
}

impl<T: CharacterComponents + Send + Sync + 'static> CharacterComponents for ComponentsOnly<T> {
	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RigNode> {
		self.0.rig_nodes_for_level(level)
	}

	fn part_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.0.part_nodes_for_level(level)
	}
}

impl<T: CharacterComponents + Send + Sync + 'static> LodScene for ComponentsOnly<T> {
	fn scene_lod_level(&self, _lod_ref: &LodRef) -> LodSceneLevel {
		LodSceneLevel::High
	}

	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		LodSceneCulls::None
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		component_only_scene(&self.0, lod_ref, level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		character_scene_chunks(&self.0, lod_ref, level)
	}

	fn scene_bounds(&self) -> Aabb3d {
		character_bounds(&self.0)
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let level = self.scene_lod_level(lod_ref);
		lod_host_scene_pending(level, self.scene_bounds())
	}
}

/// Weighted chunks for one structural level: each rig/part node is a nested LOD host.
pub fn character_scene_chunks(
	character: &impl CharacterComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> SceneChunk {
	let mut chunks = Vec::new();
	for node in character.rig_nodes_for_level(level).flatten() {
		chunks.push(SceneChunk::weighted(1, node.host(lod_ref)));
	}
	for node in character.part_nodes_for_level(level).flatten() {
		chunks.push(SceneChunk::weighted(1, node.host(lod_ref)));
	}
	if chunks.is_empty() {
		SceneChunk::primitive(scene_children(Vec::new()))
	} else {
		SceneChunk::chunks(chunks)
	}
}

pub fn append_component_scenes(
	character: &impl CharacterComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
	children: &mut Vec<Box<dyn Scene>>,
) {
	for node in character.rig_nodes_for_level(level).flatten() {
		children.push(Box::new(node.host(lod_ref)));
	}
	for node in character.part_nodes_for_level(level).flatten() {
		children.push(Box::new(node.host(lod_ref)));
	}
}

pub fn component_only_scene(
	character: &impl CharacterComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	let mut children: Vec<Box<dyn Scene>> = Vec::new();
	append_component_scenes(character, lod_ref, level, &mut children);
	scene_children(children)
}

/// Spawn a [`ComponentsOnly`] character host; chunk fulfill streams the first level.
pub fn spawn_character_components<T>(
	commands: &mut Commands,
	character: &T,
	transform: Transform,
	bounds: Aabb3d,
) -> Vec<Entity>
where
	T: CharacterComponents + Clone + Send + Sync + 'static,
{
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let host = ComponentsOnly(character.clone());
	let level = host.scene_lod_level(&lod_ref);
	let pending = lod_host_scene_pending(level, bounds);
	let entity = commands
		.spawn_scene((
			pending,
			bsn! {
				template_value(transform)
				Visibility::default()
			},
		))
		.id();
	commands.entity(entity).insert(host);
	vec![entity]
}

/// Approximate AABB for a standing humanoid (High can be large; bands are identical).
pub fn character_bounds(_character: &impl CharacterComponents) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(-1.5, -0.25, -1.5), Vec3::new(1.5, 2.75, 1.5))
}
