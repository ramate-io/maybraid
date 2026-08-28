//! Maybraid PBR adapter for [`lod::VisualLodScene`].
//!
//! Stamps a persistent [`VisualLodRoot`] sibling with identifying data so
//! [`lod::VisualLodPolicy`] can select per view. Packed UltraLow/Low/Medium
//! draws are not realized yet — trees stay on the semantic [`lod::SceneChunk`]
//! drain. Do not `spawn_scene` authored grove bands under this root.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::render::sync_world::SyncToRenderWorld;
use lod::{
	HasVisualLodThresholds, LodHostBounds, NamedVisualLevel, ProjectedBoundsPolicy,
	ProjectedBoundsThresholds, VisualLodPolicy, VisualLodRenderContext, VisualLodRenderer,
	VisualLodRoot, VisualLodScene, VisualSceneLodPlugin,
};

/// Visual sibling stamped on a presented forest grove tile.
#[derive(Debug, Clone, Component)]
pub struct ForestGroveVisual {
	pub thresholds: ProjectedBoundsThresholds,
}

impl ForestGroveVisual {
	pub fn new() -> Self {
		Self { thresholds: ProjectedBoundsThresholds::default() }
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
	type Representation = ();
	type Policy = ProjectedBoundsPolicy;
	type Renderer = PbrVisualRenderer;

	fn visual_representations(&self) -> Self::Representation {}
}

/// Records the policy pick in the render world. Packed submit comes later.
pub struct PbrVisualRenderer;

impl VisualLodRenderer<ForestGroveVisual> for PbrVisualRenderer {
	fn queue(
		_scene: &ForestGroveVisual,
		selection: <ProjectedBoundsPolicy as VisualLodPolicy<ForestGroveVisual>>::Selection,
		ctx: &mut VisualLodRenderContext,
	) {
		ctx.insert(SelectedVisualBand(selection));
	}
}

/// Render-world selection recorded by [`PbrVisualRenderer`].
#[derive(Component, Clone, Copy)]
pub struct SelectedVisualBand(pub NamedVisualLevel);

/// Extract + policy select for [`ForestGroveVisual`].
pub struct ForestGroveVisualPlugin;

impl Plugin for ForestGroveVisualPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VisualSceneLodPlugin<ForestGroveVisual>>() {
			app.add_plugins(VisualSceneLodPlugin::<ForestGroveVisual>::default());
		}
	}
}

/// Persistent visual sibling: policy data only. No kit `spawn_scene`.
///
/// Does **not** stamp [`lod::VisualOwnsAppearance`] — semantic fulfill still
/// draws the tile until packed representations exist.
pub fn attach_forest_grove_visual(commands: &mut Commands, host: Entity, bounds: Aabb3d) {
	let visual = commands
		.spawn((
			VisualLodRoot,
			ForestGroveVisual::new(),
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
	use lod::{LodHostBounds, VisualLodView};

	#[test]
	fn policy_selects_without_spawning_bands() {
		let visual = ForestGroveVisual::new();
		let bounds = LodHostBounds(Aabb3d::from_min_max(Vec3::splat(-50.0), Vec3::splat(50.0)));
		let far = VisualLodView::test_perspective(
			Vec3::new(0.0, 1_500.0, 3_000.0),
			Vec3::ZERO,
			Vec2::new(1280.0, 720.0),
			std::f32::consts::FRAC_PI_3,
		);
		let sel = ProjectedBoundsPolicy::select(&visual, &far, &bounds);
		assert!(matches!(sel, NamedVisualLevel::UltraLow | NamedVisualLevel::Low));
		let _ = visual.visual_representations();
	}
}
