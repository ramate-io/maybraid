//! Maybraid PBR adapter for [`lod::VisualLodScene`].
//!
//! Grove visual bands are the same authored kit / canopy scenes the semantic
//! path used to spawn. They live under a persistent [`VisualLodRoot`]; Policy
//! shows one band per view. The tile host keeps [`lod::VisualOwnsAppearance`]
//! so `SceneChunk` no longer draws trees.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponentPlugin;
use bevy::render::sync_world::SyncToRenderWorld;
use bevy::render::{Render, RenderApp, RenderSystems};
use lod::{
	HasVisualLodThresholds, LodHostBounds, LodSceneLevel, LodViewer, NamedVisualLevel,
	ProjectedBoundsPolicy, ProjectedBoundsThresholds, SceneChunk, SemanticLodScene, VisualLodBand,
	VisualLodPolicy, VisualLodRenderContext, VisualLodRenderer, VisualLodRoot, VisualLodScene,
	VisualLodView, VisualOwnsAppearance, VisualSceneLodPlugin,
};
use lod::lod_ref::LodRef;

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

/// Queues the selected named band. Band visibility is applied in the render world.
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

/// Render-world selection: show this [`VisualLodBand`], hide siblings.
#[derive(Component, Clone, Copy)]
pub struct SelectedVisualBand(pub NamedVisualLevel);

/// Prepare + extract + band visibility for [`ForestGroveVisual`].
pub struct ForestGroveVisualPlugin;

impl Plugin for ForestGroveVisualPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VisualSceneLodPlugin<ForestGroveVisual>>() {
			app.add_plugins(VisualSceneLodPlugin::<ForestGroveVisual>::default());
		}
		if !app.is_plugin_added::<ExtractComponentPlugin<VisualLodBand>>() {
			app.add_plugins(ExtractComponentPlugin::<VisualLodBand>::default());
		}
		app.add_systems(Update, show_selected_visual_band);
		if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
			render_app.add_systems(
				Render,
				apply_selected_visual_band.in_set(RenderSystems::PrepareMeshes),
			);
		}
	}
}

fn band_visibility(band: NamedVisualLevel, selected: NamedVisualLevel) -> Visibility {
	if band == selected {
		Visibility::Inherited
	} else {
		Visibility::Hidden
	}
}

/// Main-world visibility so Bevy `ViewVisibility` can extract the selected band.
///
/// Writes only [`VisualLodBand`] children, not the semantic root.
fn show_selected_visual_band(
	views: Query<(&Camera, &GlobalTransform), With<LodViewer>>,
	visuals: Query<(&ForestGroveVisual, &LodHostBounds, &Children), With<VisualLodRoot>>,
	mut bands: Query<(&VisualLodBand, &mut Visibility)>,
) {
	let view = views.iter().find_map(|(camera, transform)| VisualLodView::from_camera(camera, transform));
	for (visual, bounds, children) in &visuals {
		let selected = view
			.as_ref()
			.map(|view| ProjectedBoundsPolicy::select(visual, view, bounds))
			.unwrap_or(NamedVisualLevel::High);
		for child in children.iter() {
			let Ok((band, mut visibility)) = bands.get_mut(child) else {
				continue;
			};
			let want = band_visibility(band.0, selected);
			if *visibility != want {
				*visibility = want;
			}
		}
	}
}

fn apply_selected_visual_band(
	mut commands: Commands,
	selected: Query<&SelectedVisualBand>,
	bands: Query<(Entity, &VisualLodBand, &ChildOf)>,
) {
	for (entity, band, child_of) in &bands {
		let Ok(selected) = selected.get(child_of.parent()) else {
			continue;
		};
		let visibility = if band.0 == selected.0 {
			Visibility::Inherited
		} else {
			Visibility::Hidden
		};
		commands.entity(entity).insert(visibility);
	}
}

/// Persistent visual sibling: one child per named band, authored grove scenes.
pub fn attach_forest_grove_visual<T>(
	commands: &mut Commands,
	host: Entity,
	grove: &T,
	bounds: Aabb3d,
	lod_ref: &LodRef,
) where
	T: SemanticLodScene + Component,
{
	commands.entity(host).insert(VisualOwnsAppearance);
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
	for named in NamedVisualLevel::ALL {
		let level = named.to_scene_level();
		let band = commands
			.spawn((
				VisualLodBand(named),
				Transform::IDENTITY,
				band_visibility(named, NamedVisualLevel::High),
				SyncToRenderWorld,
			))
			.id();
		commands.entity(visual).add_child(band);
		spawn_band_scenes(commands, band, grove.scene_chunks_with_level(lod_ref, level), level);
	}
}

fn spawn_band_scenes(
	commands: &mut Commands,
	band: Entity,
	chunk: SceneChunk,
	level: LodSceneLevel,
) {
	for (_weight, scene) in chunk.into_primitives() {
		let child = commands.spawn_scene(scene).id();
		commands.entity(band).add_child(child);
		commands.entity(child).insert(level);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::bounding::Aabb3d;
	use lod::{LodHostBounds, VisualLodView};

	#[test]
	fn policy_selects_without_semantic_chunks() {
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
		assert_eq!(band_visibility(sel, sel), Visibility::Inherited);
		assert_eq!(band_visibility(NamedVisualLevel::High, sel), Visibility::Hidden);
		let _ = visual.visual_representations();
	}
}
