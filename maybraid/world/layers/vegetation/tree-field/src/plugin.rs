//! Main-world plugin: patch upload, presenter AABB, and render-app wiring.

use std::sync::Arc;

use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{self, VisibilityClass};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::extract_component::ExtractComponentPlugin;
use bevy::render::extract_resource::ExtractResource;
use bevy::render::extract_resource::ExtractResourcePlugin;
use bevy::render::RenderApp;
use vegetation_components::CanopyShadowMode;
use vegetation_grass_field::GrassGpuCapability;
use vegetation_shaders::VegetationWeslPlugin;

use crate::diagnostics::TreeFieldStats;
use crate::field::{TreeField, TreeFieldSettings};
use crate::gpu::{requested_world_tree_mode, TreeGpuPlugin};
use crate::patch::TreePatch;
use crate::shaders::TreeShadersPlugin;

/// Marker extracted into the render world. CPU frustum cull uses the conservative AABB.
#[derive(Component, Clone, Copy, Debug, Eq, ExtractComponent, PartialEq)]
#[require(VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<TreeFieldPresenter>)]
#[extract_app(RenderApp)]
pub(crate) enum TreeFieldPresenter {
	Wood,
	Foliage,
}

/// Frame inputs plus an immutable descriptor snapshot shared with the render world.
#[derive(Resource, Clone, ExtractResource, Default)]
#[extract_app(RenderApp)]
pub struct ExtractedTreeField {
	pub field: Arc<TreeField>,
	pub settings: TreeFieldSettings,
	pub time_s: f32,
	pub prev_time_s: f32,
	pub light_direction: Vec3,
	pub shadow_mode: CanopyShadowMode,
}

/// Registers GPU tree-canopy field rendering. Existing kit trees stay available.
pub struct TreeFieldPlugin;

impl Plugin for TreeFieldPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VegetationWeslPlugin>() {
			app.add_plugins(VegetationWeslPlugin);
		}
		app.add_plugins(TreeShadersPlugin);
		app.insert_resource(TreeFieldSettings::from_env());
		app.init_resource::<TreeField>();
		app.init_resource::<TreeFieldStats>();
		app.init_resource::<GrassGpuCapability>();
		app.init_resource::<PrevTreeTime>();
		app.init_resource::<ExtractedTreeField>();
		app.add_plugins(ExtractResourcePlugin::<ExtractedTreeField>::default());
		app.add_plugins(ExtractComponentPlugin::<TreeFieldPresenter>::default());
		app.add_systems(Startup, spawn_presenter);
		app.add_systems(
			Update,
			(
				rebuild_field,
				snapshot_extracted_field,
				sync_presenter_bounds,
				sync_presenter_shadows,
			)
				.chain(),
		);
		app.add_plugins(TreeGpuPlugin);
	}
}

#[derive(Resource, Default)]
struct PrevTreeTime(f32);

fn spawn_presenter(mut commands: Commands) {
	// The retained opaque phase has one cached bin per MainEntity.
	for (name, kind) in [
		("TreeFieldWood", TreeFieldPresenter::Wood),
		("TreeFieldFoliage", TreeFieldPresenter::Foliage),
	] {
		commands.spawn((
			Name::new(name),
			kind,
			Visibility::default(),
			Transform::default(),
			Aabb::default(),
			NotShadowCaster,
		));
	}
}

fn sync_presenter_shadows(
	mode: Option<Res<CanopyShadowMode>>,
	mut commands: Commands,
	casters: Query<Entity, (With<TreeFieldPresenter>, Without<NotShadowCaster>)>,
	non_casters: Query<Entity, (With<TreeFieldPresenter>, With<NotShadowCaster>)>,
) {
	let casts = mode.is_some_and(|mode| mode.canopy_casts());
	if casts {
		for entity in &non_casters {
			commands.entity(entity).remove::<NotShadowCaster>();
		}
	} else {
		for entity in &casters {
			commands.entity(entity).insert(NotShadowCaster);
		}
	}
}

fn rebuild_field(
	changed: Query<(), Changed<TreePatch>>,
	mut removed: RemovedComponents<TreePatch>,
	all_patches: Query<&TreePatch>,
	settings: Res<TreeFieldSettings>,
	mut field: ResMut<TreeField>,
) {
	let patch_removed = removed.read().count() != 0;
	if changed.is_empty() && !patch_removed && !settings.is_changed() {
		return;
	}
	let generation = field.generation.wrapping_add(1);
	*field = TreeField::rebuild(all_patches.iter(), &settings).with_generation(generation);
	if requested_world_tree_mode().is_some() {
		let requested = all_patches.iter().map(|patch| patch.samples.len()).sum::<usize>();
		let leafy_plants = field.plants.iter().filter(|plant| plant.leaf_count > 0).count();
		info!(
			target: "maybraid.tree_field",
			generation, requested, plants = field.plants.len(), leafy_plants,
			branches = field.branches.len(), leaves = field.leaves.len(),
			"world descriptor inventory"
		);
	}
}

fn snapshot_extracted_field(
	mut extracted: ResMut<ExtractedTreeField>,
	field: Res<TreeField>,
	settings: Res<TreeFieldSettings>,
	time: Res<Time>,
	mut prev: ResMut<PrevTreeTime>,
	lights: Query<&GlobalTransform, With<DirectionalLight>>,
	shadow_mode: Option<Res<CanopyShadowMode>>,
) {
	let time_s = settings.freeze_time_s.unwrap_or(time.elapsed_secs());
	let prev_time_s = settings.freeze_time_s.unwrap_or(prev.0);
	prev.0 = time_s;
	let light_direction = lights
		.iter()
		.next()
		.map(|transform| *transform.back())
		.unwrap_or(Vec3::new(0.4, 0.8, 0.3));
	if extracted.field.generation != field.generation {
		extracted.field = Arc::new(field.clone());
	}
	extracted.settings = settings.clone();
	extracted.time_s = time_s;
	extracted.prev_time_s = prev_time_s;
	extracted.light_direction = light_direction.normalize_or(Vec3::Y);
	extracted.shadow_mode = shadow_mode.map(|mode| *mode).unwrap_or_default();
}

fn sync_presenter_bounds(
	field: Res<TreeField>,
	mut presenters: Query<&mut Aabb, With<TreeFieldPresenter>>,
) {
	if !field.is_changed() || field.is_empty() {
		return;
	}
	let aabb = field.bounds.aabb();
	for mut existing in &mut presenters {
		*existing = aabb;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::patch::TreePatch;
	use crate::recipe::TreeRecipe;

	#[test]
	fn wood_and_foliage_have_separate_presenter_entities() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_systems(Startup, spawn_presenter);
		app.update();
		let world = app.world_mut();
		let mut query = world.query::<(Entity, &TreeFieldPresenter)>();
		let presenters: Vec<_> = query.iter(world).map(|(id, kind)| (id, *kind)).collect();
		assert_eq!(presenters.len(), 2);
		let (wood, _) = presenters
			.iter()
			.find(|(_, kind)| *kind == TreeFieldPresenter::Wood)
			.ok_or_else(|| anyhow::anyhow!("expected a wood presenter"))?;
		world.despawn(*wood);
		assert_eq!(
			query.iter(world).map(|(_, kind)| *kind).collect::<Vec<_>>(),
			[TreeFieldPresenter::Foliage]
		);
		Ok(())
	}

	#[test]
	fn extraction_reuses_descriptors_until_generation_changes() {
		let mut app = App::new();
		app.init_resource::<TreeField>()
			.init_resource::<TreeFieldSettings>()
			.init_resource::<Time>()
			.init_resource::<PrevTreeTime>()
			.init_resource::<ExtractedTreeField>()
			.add_systems(Update, snapshot_extracted_field);
		app.update();
		let initial = app.world().resource::<ExtractedTreeField>().field.clone();
		app.update();
		assert!(Arc::ptr_eq(&initial, &app.world().resource::<ExtractedTreeField>().field));
		app.world_mut().resource_mut::<TreeField>().set_changed();
		app.update();
		assert!(Arc::ptr_eq(&initial, &app.world().resource::<ExtractedTreeField>().field));
		app.world_mut().resource_mut::<TreeField>().generation = 1;
		app.update();
		let next = &app.world().resource::<ExtractedTreeField>().field;
		assert!(!Arc::ptr_eq(&initial, next));
		assert_eq!(initial.generation, 0);
		assert_eq!(next.generation, 1);
	}

	#[test]
	fn rebuild_uploads_compact_descriptors_not_meshes() -> anyhow::Result<()> {
		let patch = TreePatch::isolated(1, Vec3::ZERO, TreeRecipe::storybook());
		let settings = TreeFieldSettings::default();
		let field = TreeField::rebuild([&patch], &settings);
		assert!(!field.plants.is_empty());
		assert!(!field.branches.is_empty());
		assert_eq!(field.patches.len(), 1);
		Ok(())
	}
}
