//! Packed UltraLow / Low / Medium forest draws for [`lod::VisualLodScene`].
//!
//! [`ForestGroveVisual`] stores [`VisualInstance`]s. Geometry is cached by
//! [`scene_ref::SceneRef`]; material by [`MaterialRef`]. Policy selects a band
//! per view; [`InstancePbrRenderer`] buckets `(prototype, material)` and submits
//! instanced draws. Camera motion does not cook posed grove meshes.

mod instance_pbr;

use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::render::sync_world::SyncToRenderWorld;
use chico_vegetation_components::{pack_vegetation_visual_aliased, VegetationComponents};
pub use instance_pbr::{InstancePbrPlugin, InstancePbrRenderer};
use lod::lod_ref::LodRef;
use lod::{
	HasVisualLodThresholds, LodHostBounds, NamedVisualLevel, ProjectedBoundsPolicy,
	ProjectedBoundsThresholds, SemanticLodScene, VisualInstanceList, VisualLodRoot, VisualLodScene,
	VisualOwnsAppearance, VisualSceneLodPlugin,
};

/// Visual sibling stamped on a presented forest grove tile.
#[derive(Debug, Clone, Component)]
pub struct ForestGroveVisual {
	pub thresholds: ProjectedBoundsThresholds,
	pub representation: VisualInstanceList,
}

impl ForestGroveVisual {
	pub fn new() -> Self {
		Self {
			thresholds: ProjectedBoundsThresholds::default(),
			representation: VisualInstanceList::default(),
		}
	}

	fn from_instances(instances: VisualInstanceList) -> Self {
		Self { thresholds: ProjectedBoundsThresholds::default(), representation: instances }
	}
}

impl Default for ForestGroveVisual {
	fn default() -> Self {
		Self::new()
	}
}

impl HasVisualLodThresholds for ForestGroveVisual {
	fn visual_lod_thresholds(&self) -> ProjectedBoundsThresholds {
		self.thresholds
	}
}

impl VisualLodScene for ForestGroveVisual {
	type Representation = VisualInstanceList;
	type Policy = ProjectedBoundsPolicy;
	type Renderer = InstancePbrRenderer;

	fn visual_representations(&self) -> Self::Representation {
		self.representation.clone()
	}
}

/// Render-world selection recorded by [`InstancePbrRenderer`].
#[derive(Component, Clone, Copy)]
pub struct SelectedVisualBand(pub NamedVisualLevel);

/// Packed instance list + instanced PBR submit.
pub struct ForestGroveVisualPlugin;

impl Plugin for ForestGroveVisualPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<scene_ref::SceneRefPlugin>() {
			app.add_plugins(scene_ref::SceneRefPlugin);
		}
		if !app.is_plugin_added::<VisualSceneLodPlugin<ForestGroveVisual>>() {
			app.add_plugins(VisualSceneLodPlugin::<ForestGroveVisual>::default());
		}
		if !app.is_plugin_added::<InstancePbrPlugin>() {
			app.add_plugins(InstancePbrPlugin);
		}
	}
}

/// Persistent visual sibling: interned UltraLow / Low / Medium instances.
///
/// Stamps [`VisualOwnsAppearance`] so non-High semantic fulfill does not spawn
/// plant hosts. High still uses the exclusive drain.
pub fn attach_forest_grove_visual<T>(
	commands: &mut Commands,
	host: Entity,
	grove: &T,
	bounds: Aabb3d,
	_lod_ref: &LodRef,
) where
	T: SemanticLodScene + VegetationComponents + Component,
{
	commands.entity(host).insert(VisualOwnsAppearance);
	let packed = pack_vegetation_visual_aliased(grove);
	let visual = commands
		.spawn((
			VisualLodRoot,
			ForestGroveVisual::from_instances(VisualInstanceList::new(Arc::<[_]>::from(
				packed.instances,
			))),
			LodHostBounds(bounds),
			Transform::IDENTITY,
			Visibility::Inherited,
			SyncToRenderWorld,
		))
		.id();
	commands.entity(host).add_child(visual);
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::bounding::Aabb3d;
	use lod::{LodHostBounds, VisualLodPolicy, VisualLodView};

	#[test]
	fn policy_selects_packed_far_band() {
		let visual = ForestGroveVisual::new();
		let bounds = LodHostBounds(Aabb3d::from_min_max(Vec3::splat(-50.0), Vec3::splat(50.0)));
		let far = VisualLodView::test_perspective(
			Vec3::new(0.0, 1_500.0, 3_000.0),
			Vec3::ZERO,
			Vec2::new(1280.0, 720.0),
			std::f32::consts::FRAC_PI_3,
		);
		let sel = ProjectedBoundsPolicy::select(&visual, &far, &bounds).clamp_to_packed();
		assert!(matches!(sel, NamedVisualLevel::UltraLow | NamedVisualLevel::Low));
		assert!(visual.visual_representations().is_empty());
	}

	#[test]
	fn high_policy_clamps_to_medium_pack() {
		assert_eq!(NamedVisualLevel::High.clamp_to_packed(), NamedVisualLevel::Medium);
		assert_eq!(NamedVisualLevel::Medium.clamp_to_packed(), NamedVisualLevel::Medium);
	}

	#[test]
	fn representation_is_instance_list() {
		let _ = std::any::type_name::<<ForestGroveVisual as VisualLodScene>::Representation>();
	}
}
