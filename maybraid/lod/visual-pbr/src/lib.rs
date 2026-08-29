//! Packed UltraLow / Low / Medium forest draws for [`lod::VisualLodScene`].
//!
//! Present interned [`scene_ref::MultiSceneMerge`] keys on a [`VisualLodRoot`]
//! sibling. Policy picks a band per view; High stays on the semantic drain.
//! Camera motion does not `spawn_scene` grove `SceneChunk`s.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::render::sync_world::SyncToRenderWorld;
use chico_vegetation_components::{pack_vegetation_visual, VegetationComponents, VisualPackPart};
use lod::lod_ref::LodRef;
use lod::{
	HasVisualLodThresholds, LodHostBounds, LodSceneLevel, LodViewer, NamedVisualLevel,
	ProjectedBoundsPolicy, ProjectedBoundsThresholds, SemanticLodScene, VisualLodBand,
	VisualLodPolicy, VisualLodRenderContext, VisualLodRenderer, VisualLodRoot, VisualLodScene,
	VisualLodView, VisualOwnsAppearance, VisualSceneLodPlugin,
};
use material_ref::{MaterialRefRoot, PropagateToDescendants};
use scene_ref::{MultiSceneMergeRoot, SceneRefPlugin};

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

/// Records the policy pick in the render world.
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

/// Extract + band visibility for packed grove draws.
pub struct ForestGroveVisualPlugin;

impl Plugin for ForestGroveVisualPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SceneRefPlugin>() {
			app.add_plugins(SceneRefPlugin);
		}
		if !app.is_plugin_added::<VisualSceneLodPlugin<ForestGroveVisual>>() {
			app.add_plugins(VisualSceneLodPlugin::<ForestGroveVisual>::default());
		}
		app.add_systems(Update, show_packed_grove_band);
	}
}

/// Persistent visual sibling: packed UltraLow / Low / Medium merges.
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

	let ultra_low = pack_vegetation_visual(grove, LodSceneLevel::UltraLow);
	let mut low = pack_vegetation_visual(grove, LodSceneLevel::Low);
	let mut medium = pack_vegetation_visual(grove, LodSceneLevel::Medium);
	if low.is_empty() {
		low.clone_from(&ultra_low);
	}
	if medium.is_empty() {
		medium.clone_from(&low);
	}

	spawn_packed_band(commands, visual, NamedVisualLevel::UltraLow, &ultra_low);
	spawn_packed_band(commands, visual, NamedVisualLevel::Low, &low);
	spawn_packed_band(commands, visual, NamedVisualLevel::Medium, &medium);
}

fn spawn_packed_band(
	commands: &mut Commands,
	visual: Entity,
	named: NamedVisualLevel,
	parts: &[VisualPackPart],
) {
	let band = commands
		.spawn((VisualLodBand(named), Transform::IDENTITY, Visibility::Hidden, SyncToRenderWorld))
		.id();
	commands.entity(visual).add_child(band);
	for part in parts {
		let child = commands
			.spawn((
				MultiSceneMergeRoot(part.merge.clone()),
				MaterialRefRoot(part.material.clone()),
				PropagateToDescendants,
				Transform::IDENTITY,
				Visibility::Inherited,
			))
			.id();
		commands.entity(band).add_child(child);
	}
}

/// Show the policy band while the tile is not semantically High.
fn show_packed_grove_band(
	views: Query<(&Camera, &GlobalTransform), With<LodViewer>>,
	visuals: Query<
		(Entity, &ForestGroveVisual, &LodHostBounds, &ChildOf, &Children),
		With<VisualLodRoot>,
	>,
	host_levels: Query<&LodSceneLevel>,
	mut bands: Query<(&VisualLodBand, &mut Visibility)>,
) {
	let view = views
		.iter()
		.find_map(|(camera, transform)| VisualLodView::from_camera(camera, transform));
	for (_entity, visual, bounds, child_of, children) in &visuals {
		let semantic_high = host_levels
			.get(child_of.parent())
			.is_ok_and(|level| *level == LodSceneLevel::High);
		let selected = if semantic_high {
			None
		} else {
			Some(
				view.as_ref()
					.map(|view| ProjectedBoundsPolicy::select(visual, view, bounds))
					.unwrap_or(NamedVisualLevel::Low)
					.clamp_packed(),
			)
		};
		for child in children.iter() {
			let Ok((band, mut visibility)) = bands.get_mut(child) else {
				continue;
			};
			let want = match selected {
				Some(sel) if band.0 == sel => Visibility::Inherited,
				_ => Visibility::Hidden,
			};
			if *visibility != want {
				*visibility = want;
			}
		}
	}
}

trait ClampPacked {
	fn clamp_packed(self) -> NamedVisualLevel;
}

impl ClampPacked for NamedVisualLevel {
	fn clamp_packed(self) -> NamedVisualLevel {
		match self {
			NamedVisualLevel::High => NamedVisualLevel::Medium,
			other => other,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::bounding::Aabb3d;
	use lod::{LodHostBounds, VisualLodView};

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
		let sel = ProjectedBoundsPolicy::select(&visual, &far, &bounds).clamp_packed();
		assert!(matches!(sel, NamedVisualLevel::UltraLow | NamedVisualLevel::Low));
		let _ = visual.visual_representations();
	}

	#[test]
	fn high_policy_clamps_to_medium_pack() {
		assert_eq!(NamedVisualLevel::High.clamp_packed(), NamedVisualLevel::Medium);
		assert_eq!(NamedVisualLevel::Medium.clamp_packed(), NamedVisualLevel::Medium);
	}
}
