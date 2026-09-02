//! Domain IR + structural [`lod::LodScene`] host for characters.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::{Component, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use crozon_character_items::{ClothingHost, ClothingMaterial, ClothingMesh, ItemColor};
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::SceneChunk;
use material_ref::MaterialRef;

use crate::assembly::CharacterPartSlot;
use crate::assets::AssetNormalization;
use crate::layer::Layers;
use crate::member::CharacterRoot;
use crate::nodes::{PartNode, RigNode};
use crozon_character_motion::motion_policy;

use crate::scene_children::{maybe_component, scene_children};
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

/// Config → inner recipe plus clothing. Clothing is never part of the inner species.
pub trait CharacterRecipe {
	type Components: CharacterComponents + Clone + Send + Sync + 'static;

	fn components(&self) -> Self::Components;

	fn clothing_layers(&self) -> Vec<ClothingLayer>;

	fn clothed(&self) -> Clothed<Self::Components> {
		Clothed::new(self.components(), self.clothing_layers())
	}
}

/// Map selected clothing meshes to [`ClothingLayer`]s fitted to `host`.
pub fn clothing_layers(
	clothing: impl IntoIterator<Item = ClothingMesh>,
	host: ClothingHost,
	material: ClothingMaterial,
	mut color: impl FnMut(ClothingMesh) -> ItemColor,
) -> Vec<ClothingLayer> {
	clothing
		.into_iter()
		.map(|mesh| ClothingLayer::new(mesh, color(mesh), host).with_material(material))
		.collect()
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
	pub material: ClothingMaterial,
	pub host: ClothingHost,
}

impl ClothingLayer {
	pub fn new(mesh: ClothingMesh, color: ItemColor, host: ClothingHost) -> Self {
		Self { mesh, color, material: ClothingMaterial::Cloth, host }
	}

	pub fn with_material(mut self, material: ClothingMaterial) -> Self {
		self.material = material;
		self
	}

	pub fn part_node(&self) -> PartNode {
		PartNode::glb(
			CharacterPartSlot::Clothing,
			self.mesh.label(),
			self.mesh.path_on(self.host),
			AssetNormalization::IDENTITY,
		)
		.skinned(SkinRef::to(RigId::Body))
		.with_material(
			MaterialRef::named(self.material.recipe_id()).with_palette([self.color.color()]),
		)
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

impl<T: Default + Send + Sync + 'static> Default for ComponentsOnly<T> {
	fn default() -> Self {
		Self(T::default())
	}
}

impl<T: Default> Default for Clothed<T> {
	fn default() -> Self {
		Self { inner: T::default(), clothing: Vec::new() }
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

	fn host_contents(&self, lod_ref: &LodRef) -> impl Scene + 'static
	where
		Self: Component + Clone + Default + Unpin + Sized,
	{
		let level = self.scene_lod_level(lod_ref);
		let policy = motion_policy(level);
		let host = self.clone();
		(
			bsn! {
				template_value(host)
				CharacterRoot
				Visibility::default()
			},
			maybe_component(policy.apply_terrain_pitch()),
		)
	}
}

/// Weighted chunks for one structural level: nested rig/part hosts only.
///
/// Motion markers live on the character / body **host** and are synced from the
/// shown LOD band — not stamped into these chunks.
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

/// Approximate AABB for a standing humanoid (High can be large; bands are identical).
pub fn character_bounds(_character: &impl CharacterComponents) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(-1.5, -0.25, -1.5), Vec3::new(1.5, 2.75, 1.5))
}
