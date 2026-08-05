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

pub use assets::AssetPath;
pub use foliage::{FoliageGeometry, FoliageNode, FoliageStyle};
pub use layer::{Layer, Layers};
pub use placed::Placement;
pub use procedural::{VegetationProceduralAssets, VegetationProceduralPlugin, STICK_KIT_HALF};
pub use scene_children::{pose, posed_mesh, scene_children, with_pose};
pub use sticks::{
	update_stick_host_levels, StickGeometry, StickLodProbe, StickNode, StickStyle, STICK_HIGH_FACTOR,
	STICK_LOW_FACTOR, STICK_MEDIUM_FACTOR,
};
pub use foliage::{
	update_foliage_host_levels, FoliageLodProbe, FOLIAGE_HIGH_FACTOR, FOLIAGE_LOW_FACTOR,
	FOLIAGE_MEDIUM_FACTOR,
};

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Commands, CommandsSceneExt, Entity, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::{LodScene, LodSceneLevel};
use lod::lod_ref::LodRef;

/// Domain IR exposed by a tree (or vegetation part) for structural composition.
pub trait VegetationComponents {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::new()
	}

	fn foliage_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FoliageNode> {
		Layers::new()
	}
}

impl<T: VegetationComponents + ?Sized> VegetationComponents for &T {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		(**self).stick_nodes_for_level(level)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		(**self).foliage_nodes_for_level(level)
	}
}

/// Newtype: present a [`VegetationComponents`] value as an [`LodScene`].
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentsOnly<T>(pub T);

impl<T> ComponentsOnly<T> {
	pub fn into_inner(self) -> T {
		self.0
	}
}

impl<T> From<T> for ComponentsOnly<T> {
	fn from(value: T) -> Self {
		Self(value)
	}
}

impl<T> std::ops::Deref for ComponentsOnly<T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.0
	}
}

impl<T: VegetationComponents> VegetationComponents for ComponentsOnly<T> {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		self.0.stick_nodes_for_level(level)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		self.0.foliage_nodes_for_level(level)
	}
}

impl<T: VegetationComponents> LodScene for ComponentsOnly<T> {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		component_only_scene(&self.0, lod_ref, level)
	}
}

/// Append every domain node from `vegetation` at `level` as nested [`LodScene`] children.
pub fn append_component_scenes(
	vegetation: &impl VegetationComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
	children: &mut Vec<Box<dyn Scene>>,
) {
	for node in vegetation.stick_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_lod(lod_ref)));
	}
	for node in vegetation.foliage_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_lod(lod_ref)));
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

/// Thin adapter: spawn a [`VegetationComponents`] tree via [`LodScene`] under `transform`.
///
/// Used by groves (and playground `/render`) until those layers own LodScene presentation.
pub fn spawn_vegetation_components(
	commands: &mut Commands,
	vegetation: &impl VegetationComponents,
	transform: Transform,
	bounds: Aabb3d,
) -> Vec<Entity> {
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let scene = ComponentsOnly(vegetation).scene_with_lod(&lod_ref);
	let entity = commands
		.spawn_scene((
			scene,
			bsn! {
				template_value(transform)
				Visibility::default()
			},
		))
		.id();
	vec![entity]
}

/// Approximate AABB from stick/foliage placements at High (for adapter LodRef bounds).
pub fn vegetation_bounds(vegetation: &impl VegetationComponents) -> Aabb3d {
	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	let mut any = false;
	for node in vegetation.stick_nodes_for_level(LodSceneLevel::High).flatten() {
		let c = crate::lod_band::placement_center(&node.placement);
		let e = crate::lod_band::characteristic_extent_abs(&node.placement);
		min = min.min(c - Vec3::splat(e));
		max = max.max(c + Vec3::splat(e));
		any = true;
	}
	for node in vegetation.foliage_nodes_for_level(LodSceneLevel::High).flatten() {
		let c = node.placement.translation;
		let e = crate::lod_band::characteristic_extent_abs(&node.placement);
		min = min.min(c - Vec3::splat(e));
		max = max.max(c + Vec3::splat(e));
		any = true;
	}
	if !any {
		return Aabb3d::from_min_max(Vec3::splat(-1.0), Vec3::splat(1.0));
	}
	Aabb3d::from_min_max(min, max)
}

use bevy_math::Vec3;
