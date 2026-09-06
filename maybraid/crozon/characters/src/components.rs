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
use crozon_character_motion::{motion_policy, CharacterHeading};
use rigs::AssemblyRoot;

use crate::scene_children::{maybe_component, scene_children};
use crate::socket::{RigId, SkinRef};

/// Rest-pose locomotion hull. Matches Avian `Collider::capsule(radius, length)`:
/// `length` is the cylinder; total height is `length + 2 * radius`.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct LocomotionCapsule {
	pub radius: f32,
	pub length: f32,
}

impl LocomotionCapsule {
	/// Standing ~1.8 m humanoid (Braidman and other unscaled bipeds).
	pub const HUMANOID: Self = Self { radius: 0.4, length: 1.0 };
	/// Low vertical stand-in for a quadruped (not a horizontal body hull).
	pub const QUADRUPED: Self = Self { radius: 0.35, length: 0.4 };
	const GROUND_CLEARANCE: f32 = 0.15;

	pub fn scaled(self, scale: f32) -> Self {
		Self { radius: self.radius * scale.max(0.0), length: self.length * scale.max(0.0) }
	}

	/// Stretch the cylinder so the capsule bottom sits at `-half_height`.
	pub fn with_half_height(self, half_height: f32) -> Self {
		let half = half_height.max(self.radius);
		Self { radius: self.radius, length: (half - self.radius) * 2.0 }
	}

	/// Rest-pose feet below the visual/capsule origin.
	///
	/// Thigh + shin at rest (`crozon_rigs::quadruped::LegSegmentLengths`), times
	/// the species / slider / lanky length product.
	pub fn quadruped_feet_below_origin(limb_scale: f32) -> f32 {
		let legs = crozon_rigs::quadruped::LegSegmentLengths::default();
		(legs.upper + legs.lower) * limb_scale.max(0.0)
	}

	/// Standing quadruped hull whose bottom matches rest-pose foot depth.
	///
	/// `limb_scale` is the product of species baseline, slider, and lanky length
	/// layers (`1.0` = stock thigh+shin).
	pub fn quadruped_for_limb_length(limb_scale: f32) -> Self {
		Self::QUADRUPED.with_half_height(Self::quadruped_feet_below_origin(limb_scale))
	}

	pub fn half_height(self) -> f32 {
		self.radius + self.length * 0.5
	}

	pub fn spawn_height(self) -> f32 {
		self.half_height() + Self::GROUND_CLEARANCE
	}

	/// Upper half of the top hemisphere, in local Y of the capsule origin.
	pub fn headshot_min_local_y(self) -> f32 {
		self.length * 0.5 + self.radius * 0.5
	}
}

impl Default for LocomotionCapsule {
	fn default() -> Self {
		Self::HUMANOID
	}
}

/// Domain IR exposed by a character (or character wrapper) for structural composition.
pub trait CharacterComponents {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::new()
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::new()
	}

	/// Physics hull baked from rest-pose proportions (species / slider / lanky
	/// limb length), not the animated mesh.
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		LocomotionCapsule::HUMANOID
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

	fn locomotion_capsule(&self) -> LocomotionCapsule {
		self.components().locomotion_capsule()
	}
}

/// Map selected clothing meshes to [`ClothingLayer`]s fitted to `host`.
pub fn clothing_layers(
	clothing: impl IntoIterator<Item = ClothingMesh>,
	host: ClothingHost,
	mut material: impl FnMut(ClothingMesh) -> ClothingMaterial,
	mut color: impl FnMut(ClothingMesh) -> ItemColor,
) -> Vec<ClothingLayer> {
	clothing
		.into_iter()
		.map(|mesh| ClothingLayer::new(mesh, color(mesh), host).with_material(material(mesh)))
		.collect()
}

impl<T: CharacterComponents + ?Sized> CharacterComponents for &T {
	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RigNode> {
		(**self).rig_nodes_for_level(level)
	}

	fn part_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		(**self).part_nodes_for_level(level)
	}

	fn locomotion_capsule(&self) -> LocomotionCapsule {
		(**self).locomotion_capsule()
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
		self.part_node_skinned(true)
	}

	/// Bind-pose mesh with no body armature — used for isolated item previews.
	pub fn preview_part_node(&self) -> PartNode {
		self.part_node_skinned(false)
	}

	fn part_node_skinned(&self, skinned: bool) -> PartNode {
		let node = PartNode::glb(
			CharacterPartSlot::Clothing,
			self.mesh.label(),
			self.mesh.path_on(self.host),
			AssetNormalization::IDENTITY,
		)
		.with_material(
			MaterialRef::named(self.material.recipe_id()).with_palette([self.color.color()]),
		);
		if skinned {
			node.skinned(SkinRef::to(RigId::Body))
		} else {
			node
		}
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

	fn locomotion_capsule(&self) -> LocomotionCapsule {
		self.inner.locomotion_capsule()
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

	fn locomotion_capsule(&self) -> LocomotionCapsule {
		self.0.locomotion_capsule()
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
				AssemblyRoot
				CharacterRoot
				CharacterHeading::default()
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn humanoid_hull_matches_the_legacy_capsule() {
		let hull = LocomotionCapsule::HUMANOID;
		assert!((hull.radius - 0.4).abs() < 1e-5);
		assert!((hull.length - 1.0).abs() < 1e-5);
		assert!((hull.half_height() - 0.9).abs() < 1e-5);
		assert!((hull.spawn_height() - 1.05).abs() < 1e-5);
		assert!((hull.headshot_min_local_y() - 0.7).abs() < 1e-5);
	}

	#[test]
	fn scaled_hull_keeps_proportions() {
		let hull = LocomotionCapsule::HUMANOID.scaled(0.30);
		assert!((hull.radius - 0.12).abs() < 1e-5);
		assert!((hull.length - 0.30).abs() < 1e-5);
	}

	#[test]
	fn quadruped_limb_hull_matches_rest_pose_foot_depth() {
		let hull = LocomotionCapsule::quadruped_for_limb_length(1.35);
		assert!((hull.radius - LocomotionCapsule::QUADRUPED.radius).abs() < 1e-5);
		assert!((hull.half_height() - 1.35).abs() < 1e-5);
		assert!(
			(hull.half_height() - LocomotionCapsule::quadruped_feet_below_origin(1.35)).abs()
				< 1e-5
		);
	}
}
