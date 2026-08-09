//! Reusable Chico vegetation scene components.
//!
//! Per domain: style + geometry + [`Placement`] → node (`LodScene`).

pub mod assets;
pub mod foliage;
pub mod layer;
pub mod lod_band;
pub mod lod_host;
pub mod placed;
pub mod procedural;
pub mod scene_children;
pub mod sticks;
pub mod structural_lod;

pub use assets::AssetPath;
pub use foliage::{
	update_foliage_host_levels, FoliageGeometry, FoliageLodProbe, FoliageNode, FoliageStyle,
	FrondCollection, FrondKit, FrondMember, FrondRun, FOLIAGE_HIGH_FACTOR, FOLIAGE_LOW_FACTOR,
	FOLIAGE_MEDIUM_FACTOR, FROND_COLLECTION_HIGH_FACTOR, FROND_COLLECTION_HIGH_METERS,
	FROND_COLLECTION_LOW_FACTOR, FROND_COLLECTION_LOW_METERS, FROND_COLLECTION_MEDIUM_FACTOR,
	FROND_COLLECTION_MEDIUM_METERS,
};
pub use layer::{Layer, Layers};
pub use placed::Placement;
pub use procedural::{
	VegetationProceduralAssets, VegetationProceduralPlugin, FROND_KIT_HALF_X, STICK_KIT_HALF,
};
pub use scene_children::{pose, posed_mesh, scene_children, with_pose};
pub use sticks::{
	update_stick_host_levels, StickGeometry, StickLodProbe, StickNode, StickStyle, STICK_HIGH_FACTOR,
	STICK_LOW_FACTOR, STICK_MEDIUM_FACTOR,
};
pub use lod_host::{VegetationFoliageAssetRoot, VegetationFrondAssetRoot};
pub use structural_lod::{
	StructuralLod, STRUCTURAL_HIGH_FACTOR, STRUCTURAL_LOW_FACTOR, STRUCTURAL_MEDIUM_FACTOR,
};

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Commands, CommandsSceneExt, Component, Entity, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::{
	cull_offset_bands_from_factor, lod_host_scene_pending, SceneChunk,
};

/// Domain IR exposed by a tree (or vegetation part) for structural composition.
pub trait VegetationComponents {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::new()
	}

	fn foliage_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FoliageNode> {
		Layers::new()
	}

	/// When set, drives structural [`LodScene`] banding / bounds for [`ComponentsOnly`].
	fn structural_lod(&self) -> Option<StructuralLod> {
		None
	}
}

impl<T: VegetationComponents + ?Sized> VegetationComponents for &T {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		(**self).stick_nodes_for_level(level)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		(**self).foliage_nodes_for_level(level)
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		(**self).structural_lod()
	}
}

/// Newtype: present a [`VegetationComponents`] value as an [`LodScene`] host component.
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

impl<T: VegetationComponents + Send + Sync + 'static> VegetationComponents for ComponentsOnly<T> {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		self.0.stick_nodes_for_level(level)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		self.0.foliage_nodes_for_level(level)
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		self.0.structural_lod()
	}
}

impl<T: VegetationComponents + Send + Sync + 'static> LodScene for ComponentsOnly<T> {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.0
			.structural_lod()
			.map(|p| p.level_for(lod_ref.current_transform))
			.unwrap_or(LodSceneLevel::High)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		match self.0.structural_lod() {
			Some(band) => band.status_for_lod_ref(lod_ref),
			None => LodSceneStatus::Unchanged,
		}
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		match self.0.structural_lod() {
			Some(band) => {
				let factor = lod_ref.current_transform.translation.distance(band.center)
					/ band.tree_radius.max(1e-4);
				cull_offset_bands_from_factor(
					factor,
					band.high_factor,
					band.medium_factor,
					band.low_factor,
				)
			}
			None => LodSceneCulls::None,
		}
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		component_only_scene(&self.0, lod_ref, level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		vegetation_scene_chunks(&self.0, lod_ref, level)
	}

	fn scene_bounds(&self) -> Aabb3d {
		self.0
			.structural_lod()
			.map(|p| p.footprint_aabb())
			.unwrap_or_else(|| vegetation_bounds(&self.0))
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let level = self.scene_lod_level(lod_ref);
		// Pending host: chunk fulfill streams [`Self::scene_chunks_with_level`].
		lod_host_scene_pending(level, self.scene_bounds())
	}
}

/// Weighted chunks for one structural level (stick/foliage primitives + collections).
pub fn vegetation_scene_chunks(
	vegetation: &impl VegetationComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> SceneChunk {
	let mut chunks = Vec::new();
	for node in vegetation.stick_nodes_for_level(level).flatten() {
		chunks.push(node.scene_chunks_with_level(lod_ref, level));
	}
	for node in vegetation.foliage_nodes_for_level(level).flatten() {
		chunks.push(node.scene_chunks_with_level(lod_ref, level));
	}
	if chunks.is_empty() {
		SceneChunk::primitive(scene_children(Vec::new()))
	} else {
		SceneChunk::chunks(chunks)
	}
}

/// Append every domain node from `vegetation` at `level` as flat content (no nested hosts).
pub fn append_component_scenes(
	vegetation: &impl VegetationComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
	children: &mut Vec<Box<dyn Scene>>,
) {
	for node in vegetation.stick_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_level(lod_ref, level)));
	}
	for node in vegetation.foliage_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_level(lod_ref, level)));
	}
}

pub fn component_only_scene(
	vegetation: &impl VegetationComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	let mut children: Vec<Box<dyn Scene>> = Vec::new();
	append_component_scenes(vegetation, lod_ref, level, &mut children);
	scene_children(children)
}

/// Spawn a [`ComponentsOnly`] vegetation host; chunk fulfill streams the first level.
pub fn spawn_vegetation_components<T>(
	commands: &mut Commands,
	vegetation: &T,
	transform: Transform,
	bounds: Aabb3d,
) -> Vec<Entity>
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
{
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let host = ComponentsOnly(vegetation.clone());
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

/// Approximate AABB from stick/foliage placements at High (for adapter LodRef bounds).
pub fn vegetation_bounds(vegetation: &impl VegetationComponents) -> Aabb3d {
	let mut min = bevy::math::Vec3::splat(f32::INFINITY);
	let mut max = bevy::math::Vec3::splat(f32::NEG_INFINITY);
	let mut any = false;
	for node in vegetation.stick_nodes_for_level(LodSceneLevel::High).flatten() {
		let c = crate::lod_band::placement_center(&node.placement);
		let e = crate::lod_band::characteristic_extent_abs(&node.placement);
		min = min.min(c - bevy::math::Vec3::splat(e));
		max = max.max(c + bevy::math::Vec3::splat(e));
		any = true;
	}
	for node in vegetation.foliage_nodes_for_level(LodSceneLevel::High).flatten() {
		if let Some(collection) = node.geometry.as_frond_collection() {
			if let Some((cmin, cmax)) = collection.aabb() {
				min = min.min(cmin);
				max = max.max(cmax);
				any = true;
				continue;
			}
		}
		let c = crate::lod_band::placement_center(&node.placement);
		let e = crate::lod_band::characteristic_extent_abs(&node.placement);
		min = min.min(c - bevy::math::Vec3::splat(e));
		max = max.max(c + bevy::math::Vec3::splat(e));
		any = true;
	}
	if any {
		Aabb3d::from_min_max(min, max)
	} else {
		Aabb3d::from_min_max(bevy::math::Vec3::ZERO, bevy::math::Vec3::ONE)
	}
}
