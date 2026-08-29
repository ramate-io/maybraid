//! Generic instanced PBR draws for [`lod::VisualLodScene`].
//!
//! [`InstancePbrVisual`] stores [`VisualInstance`]s. Geometry is cached by
//! [`scene_ref::SceneRef`]; material by [`MaterialRef`]. Policy selects a band
//! per view; [`InstancePbrRenderer`] buckets per visual root, then `(mesh, material)`,
//! and submits instanced draws. Camera motion does not cook posed grove meshes.

mod instance_pbr;

use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::render::sync_world::SyncToRenderWorld;
use chico_vegetation_components::{pack_vegetation_visual_aliased, VegetationVisualPack};
pub use instance_pbr::{InstancePbrCompileBudget, InstancePbrPlugin, InstancePbrRenderer};
use lod::lod_ref::LodRef;
use lod::{
	HasVisualLodThresholds, LodHostBounds, ProjectedBoundsPolicy, ProjectedBoundsThresholds,
	SemanticLodScene, VisualInstanceList, VisualLodRoot, VisualLodScene, VisualOwnsAppearance,
	VisualSceneLodPlugin,
};

/// Domain-neutral banded instance scene rendered by [`InstancePbrRenderer`].
#[derive(Debug, Clone, Component)]
pub struct InstancePbrVisual {
	pub thresholds: ProjectedBoundsThresholds,
	pub representation: VisualInstanceList,
}

impl InstancePbrVisual {
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

impl Default for InstancePbrVisual {
	fn default() -> Self {
		Self::new()
	}
}

impl HasVisualLodThresholds for InstancePbrVisual {
	fn visual_lod_thresholds(&self) -> ProjectedBoundsThresholds {
		self.thresholds
	}
}

impl VisualLodScene for InstancePbrVisual {
	type Representation = VisualInstanceList;
	type Policy = ProjectedBoundsPolicy;
	type Renderer = InstancePbrRenderer;

	fn visual_representations(&self) -> Self::Representation {
		self.representation.clone()
	}
}

/// Banded instance list + instanced PBR submit.
pub struct InstancePbrVisualPlugin;

impl Plugin for InstancePbrVisualPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<scene_ref::SceneRefPlugin>() {
			app.add_plugins(scene_ref::SceneRefPlugin);
		}
		if !app.is_plugin_added::<VisualSceneLodPlugin<InstancePbrVisual>>() {
			app.add_plugins(VisualSceneLodPlugin::<InstancePbrVisual>::default());
		}
	}
}

/// Attach a persistent visual sibling to a presented forest grove tile.
///
/// Stamps [`VisualOwnsAppearance`] so ordinary appearance is always supplied by
/// the visual scene; semantic High is reserved for gameplay ECS.
pub fn attach_forest_grove_visual<T>(
	commands: &mut Commands,
	host: Entity,
	grove: &T,
	bounds: Aabb3d,
	_lod_ref: &LodRef,
) where
	T: SemanticLodScene + VegetationVisualPack + Component,
{
	commands.entity(host).insert(VisualOwnsAppearance);
	let packed = pack_vegetation_visual_aliased(grove);
	let visual = commands
		.spawn((
			VisualLodRoot,
			InstancePbrVisual::from_instances(VisualInstanceList::new(Arc::<[_]>::from(
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
	use lod::{LodHostBounds, NamedVisualLevel, VisualLodPolicy, VisualLodView};

	#[test]
	fn policy_selects_packed_far_band() {
		let visual = InstancePbrVisual::new();
		let bounds = LodHostBounds(Aabb3d::from_min_max(Vec3::splat(-50.0), Vec3::splat(50.0)));
		let far = VisualLodView::test_perspective(
			Vec3::new(0.0, 1_500.0, 3_000.0),
			Vec3::ZERO,
			Vec2::new(1280.0, 720.0),
			std::f32::consts::FRAC_PI_3,
		);
		let sel = ProjectedBoundsPolicy::select(&visual, &far, &bounds);
		assert!(matches!(sel, NamedVisualLevel::UltraLow | NamedVisualLevel::Low));
		assert!(visual.visual_representations().is_empty());
	}

	#[test]
	fn high_is_an_ordinary_visual_band() {
		assert_eq!(NamedVisualLevel::High.and_coarser().next(), Some(NamedVisualLevel::High));
	}

	#[test]
	fn representation_is_instance_list() {
		let _ = std::any::type_name::<<InstancePbrVisual as VisualLodScene>::Representation>();
	}
}
