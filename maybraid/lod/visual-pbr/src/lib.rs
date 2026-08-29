//! Packed UltraLow / Low / Medium forest draws for [`lod::VisualLodScene`].
//!
//! [`GroveVisualAsset`] is the visual representation: mesh + [`MaterialRef`] per
//! band. [`MultiSceneMerge`] is only the cook path that produces [`Handle<Mesh>`].
//! Policy selects a band per view; High stays on the semantic drain. Camera
//! motion does not `spawn_scene` grove `SceneChunk`s.

use bevy::asset::AssetServer;
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::math::bounding::Aabb3d;
use bevy::mesh::Mesh;
use bevy::prelude::*;
use bevy::render::sync_world::SyncToRenderWorld;
use bevy::world_serialization::WorldAsset;
use chico_vegetation_components::{
	pack_vegetation_visual_aliased, VegetationComponents, VisualPackPart,
};
use lod::lod_ref::LodRef;
use lod::{
	HasVisualLodThresholds, LodHostBounds, LodSceneLevel, LodViewer, NamedVisualLevel,
	ProjectedBoundsPolicy, ProjectedBoundsThresholds, SemanticLodScene, VisualLodPolicy,
	VisualLodRenderContext, VisualLodRenderer, VisualLodRoot, VisualLodScene, VisualLodView,
	VisualOwnsAppearance, VisualSceneLodPlugin,
};
use material_ref::MaterialRef;
use material_ref::MaterialRefRoot;
use scene_ref::{
	MultiSceneMerge, MultiSceneMergeHandles, SceneRefAdmitBudget, SceneRefHandles, SceneRefPlugin,
};

/// One PBR payload. `cook` is intern machinery, not the [`VisualLodScene`] contract.
#[derive(Debug, Clone)]
pub struct GrovePbrRep {
	pub material: MaterialRef,
	pub mesh: Option<Handle<Mesh>>,
	cook: MultiSceneMerge,
}

impl GrovePbrRep {
	fn from_part(part: VisualPackPart) -> Self {
		Self { material: part.material, mesh: None, cook: part.merge }
	}
}

/// Per-band mesh/material sets. Empty finer bands alias coarser cook keys.
#[derive(Debug, Clone, Default)]
pub struct GroveVisualAsset {
	pub ultra_low: Vec<GrovePbrRep>,
	pub low: Vec<GrovePbrRep>,
	pub medium: Vec<GrovePbrRep>,
}

impl GroveVisualAsset {
	fn from_aliased(
		ultra_low: Vec<VisualPackPart>,
		low: Vec<VisualPackPart>,
		medium: Vec<VisualPackPart>,
	) -> Self {
		Self {
			ultra_low: ultra_low.into_iter().map(GrovePbrRep::from_part).collect(),
			low: low.into_iter().map(GrovePbrRep::from_part).collect(),
			medium: medium.into_iter().map(GrovePbrRep::from_part).collect(),
		}
	}

	fn band(&self, named: NamedVisualLevel) -> &[GrovePbrRep] {
		match named.clamp_to_packed() {
			NamedVisualLevel::UltraLow => &self.ultra_low,
			NamedVisualLevel::Low => &self.low,
			NamedVisualLevel::Medium | NamedVisualLevel::High => &self.medium,
		}
	}

	fn band_mut(&mut self, named: NamedVisualLevel) -> &mut [GrovePbrRep] {
		match named.clamp_to_packed() {
			NamedVisualLevel::UltraLow => &mut self.ultra_low,
			NamedVisualLevel::Low => &mut self.low,
			NamedVisualLevel::Medium | NamedVisualLevel::High => &mut self.medium,
		}
	}
}

/// Visual sibling stamped on a presented forest grove tile.
#[derive(Debug, Clone, Component)]
pub struct ForestGroveVisual {
	pub thresholds: ProjectedBoundsThresholds,
	pub asset: GroveVisualAsset,
}

impl ForestGroveVisual {
	pub fn new() -> Self {
		Self {
			thresholds: ProjectedBoundsThresholds::default(),
			asset: GroveVisualAsset::default(),
		}
	}

	fn from_asset(asset: GroveVisualAsset) -> Self {
		Self { thresholds: ProjectedBoundsThresholds::default(), asset }
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
	type Representation = GroveVisualAsset;
	type Policy = ProjectedBoundsPolicy;
	type Renderer = PbrVisualRenderer;

	fn visual_representations(&self) -> Self::Representation {
		self.asset.clone()
	}
}

/// Records the policy pick in the render world. Draw still uses main-world
/// [`Mesh3d`] until a custom `Opaque3d` item exists (Bevy extract reads main
/// world meshes).
pub struct PbrVisualRenderer;

impl VisualLodRenderer<ForestGroveVisual> for PbrVisualRenderer {
	fn queue(
		_scene: &ForestGroveVisual,
		selection: <ProjectedBoundsPolicy as VisualLodPolicy<ForestGroveVisual>>::Selection,
		ctx: &mut VisualLodRenderContext,
	) {
		ctx.insert(SelectedVisualBand(selection.clamp_to_packed()));
	}
}

/// Render-world selection recorded by [`PbrVisualRenderer`].
#[derive(Component, Clone, Copy)]
pub struct SelectedVisualBand(pub NamedVisualLevel);

#[derive(Component)]
struct GrovePbrDraw;

/// Cook selected-band merges and apply one PBR proxy. No band-child `Visibility` flips.
pub struct ForestGroveVisualPlugin;

impl Plugin for ForestGroveVisualPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SceneRefPlugin>() {
			app.add_plugins(SceneRefPlugin);
		}
		if !app.is_plugin_added::<VisualSceneLodPlugin<ForestGroveVisual>>() {
			app.add_plugins(VisualSceneLodPlugin::<ForestGroveVisual>::default());
		}
		app.add_systems(Update, (cook_selected_grove_meshes, apply_grove_pbr_draw).chain());
	}
}

/// Persistent visual sibling: interned UltraLow / Low / Medium mesh payloads.
///
/// Stamps [`VisualOwnsAppearance`] so non-High semantic fulfill does not spawn
/// plant hosts. High still uses the exclusive drain. Only the selected band is
/// cooked and instantiated as [`Mesh3d`].
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
	let asset = GroveVisualAsset::from_aliased(packed.ultra_low, packed.low, packed.medium);
	let visual = commands
		.spawn((
			VisualLodRoot,
			ForestGroveVisual::from_asset(asset),
			LodHostBounds(bounds),
			Transform::IDENTITY,
			Visibility::Inherited,
			SyncToRenderWorld,
		))
		.id();
	commands.entity(host).add_child(visual);
}

fn selected_packed_band(
	visual: &ForestGroveVisual,
	bounds: &LodHostBounds,
	views: &Query<(&Camera, &GlobalTransform), With<LodViewer>>,
) -> NamedVisualLevel {
	views
		.iter()
		.filter_map(|(camera, transform)| VisualLodView::from_camera(camera, transform))
		.map(|view| ProjectedBoundsPolicy::select(visual, &view, bounds).clamp_to_packed())
		.max()
		.unwrap_or(NamedVisualLevel::Low)
}

fn cook_selected_grove_meshes(
	mut visuals: Query<(&mut ForestGroveVisual, &LodHostBounds, &ChildOf), With<VisualLodRoot>>,
	views: Query<(&Camera, &GlobalTransform), With<LodViewer>>,
	host_levels: Query<&LodSceneLevel>,
	mut scene_handles: ResMut<SceneRefHandles>,
	mut merge_handles: ResMut<MultiSceneMergeHandles>,
	asset_server: Res<AssetServer>,
	mut world_assets: ResMut<Assets<WorldAsset>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	type_registry: Res<AppTypeRegistry>,
	budget: Res<SceneRefAdmitBudget>,
) {
	let mut miss_budget = budget.new_merge_meshes_per_frame;
	for (mut visual, bounds, child_of) in &mut visuals {
		if host_levels
			.get(child_of.parent())
			.is_ok_and(|level| *level == LodSceneLevel::High)
		{
			continue;
		}
		let band = selected_packed_band(&visual, bounds, &views);
		for part in visual.asset.band_mut(band) {
			if part.mesh.is_some() {
				continue;
			}
			let Some(world_asset) = merge_handles.try_resolve(
				&part.cook,
				&mut scene_handles,
				&asset_server,
				&mut world_assets,
				&mut meshes,
				&mut materials,
				&type_registry,
				&mut miss_budget,
			) else {
				merge_handles.preload(&part.cook, &mut scene_handles, &asset_server);
				continue;
			};
			let Some(asset) = world_assets.get(&world_asset) else {
				continue;
			};
			if let Some(mesh) = mesh_handle_from_world_asset(asset) {
				part.mesh = Some(mesh);
			}
		}
	}
}

fn mesh_handle_from_world_asset(asset: &WorldAsset) -> Option<Handle<Mesh>> {
	for entity in asset.world.iter_entities() {
		if let Some(mesh) = asset.world.get::<Mesh3d>(entity.id()) {
			return Some(mesh.0.clone());
		}
	}
	None
}

/// Stamp [`Mesh3d`] children for the selected band only. Writes when the payload
/// or High mute changes, not on every camera tick inside a band.
fn apply_grove_pbr_draw(
	mut commands: Commands,
	visuals: Query<
		(Entity, &ForestGroveVisual, &LodHostBounds, &ChildOf, Option<&Children>),
		With<VisualLodRoot>,
	>,
	views: Query<(&Camera, &GlobalTransform), With<LodViewer>>,
	host_levels: Query<&LodSceneLevel>,
	draws: Query<(Entity, &Mesh3d, &MaterialRefRoot, &Visibility), With<GrovePbrDraw>>,
) {
	for (entity, visual, bounds, child_of, children) in &visuals {
		let semantic_high = host_levels
			.get(child_of.parent())
			.is_ok_and(|level| *level == LodSceneLevel::High);
		let selected = selected_packed_band(visual, bounds, &views);
		let ready: Vec<(MaterialRef, Handle<Mesh>)> = if semantic_high {
			Vec::new()
		} else {
			visual
				.asset
				.band(selected)
				.iter()
				.filter_map(|part| part.mesh.clone().map(|mesh| (part.material.clone(), mesh)))
				.collect()
		};
		let show =
			!semantic_high && ready.len() == visual.asset.band(selected).len() && !ready.is_empty();
		let existing: Vec<Entity> = children
			.map(|c| c.iter().filter(|child| draws.contains(*child)).collect())
			.unwrap_or_default();

		if !show {
			for child in existing {
				if draws.get(child).is_ok_and(|(_, _, _, vis)| *vis != Visibility::Hidden) {
					commands.entity(child).insert(Visibility::Hidden);
				}
			}
			continue;
		}

		if existing.len() != ready.len() {
			for child in &existing {
				commands.entity(*child).despawn();
			}
			for (material, mesh) in ready {
				let child = commands
					.spawn((
						GrovePbrDraw,
						Mesh3d(mesh),
						MaterialRefRoot(material),
						Transform::IDENTITY,
						Visibility::Inherited,
						SyncToRenderWorld,
					))
					.id();
				commands.entity(entity).add_child(child);
			}
			continue;
		}

		for (child, (material, mesh)) in existing.iter().zip(ready) {
			let Ok((_, have_mesh, have_mat, vis)) = draws.get(*child) else {
				continue;
			};
			if have_mesh.0 != mesh {
				commands.entity(*child).insert(Mesh3d(mesh));
			}
			if have_mat.0 != material {
				commands.entity(*child).insert(MaterialRefRoot(material));
			}
			if *vis != Visibility::Inherited {
				commands.entity(*child).insert(Visibility::Inherited);
			}
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
		let sel = ProjectedBoundsPolicy::select(&visual, &far, &bounds).clamp_to_packed();
		assert!(matches!(sel, NamedVisualLevel::UltraLow | NamedVisualLevel::Low));
		assert!(visual.visual_representations().low.is_empty());
	}

	#[test]
	fn high_policy_clamps_to_medium_pack() {
		assert_eq!(NamedVisualLevel::High.clamp_to_packed(), NamedVisualLevel::Medium);
		assert_eq!(NamedVisualLevel::Medium.clamp_to_packed(), NamedVisualLevel::Medium);
	}

	#[test]
	fn representation_is_grove_visual_asset() {
		let _ = std::any::type_name::<<ForestGroveVisual as VisualLodScene>::Representation>();
	}
}
