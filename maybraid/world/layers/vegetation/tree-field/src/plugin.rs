//! Main-world plugin: catalog admits, two kind presenters, render-app wiring.

use std::sync::Arc;

use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{self, NoFrustumCulling, VisibilityClass};
use bevy::ecs::{entity_disabling::Disabled, query::Allow};
use bevy::light::NotShadowCaster;
use bevy::math::Vec3A;
use bevy::prelude::*;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::{ExtractSchedule, MainWorld, RenderApp};
use vegetation_components::CanopyShadowMode;
use vegetation_grass_field::{
	CatalogApply, GrassGpuCapability, GroveId, GroveRetainPolicy, ADMISSION_BUDGET,
};
use vegetation_shaders::VegetationWeslPlugin;

use crate::catalog::{TreeCatalogOp, TreeCatalogOps, TreeGroveCatalog};
use crate::diagnostics::TreeFieldStats;
use crate::field::{TreeField, TreeFieldSettings};
use crate::gpu::{requested_world_tree_mode, TreeGpuPlugin};
use crate::patch::TreePatch;
use crate::shaders::TreeShadersPlugin;

/// Marker extracted into the render world. CPU frustum cull uses the catalog AABB.
#[derive(Component, Clone, Copy, Debug, Eq, ExtractComponent, PartialEq)]
#[require(VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<TreeFieldPresenter>)]
#[extract_app(RenderApp)]
pub(crate) enum TreeFieldPresenter {
	Wood,
	Foliage,
}

/// Immutable snapshot used as an admit source. Presenters are not parented here.
#[derive(Component, Clone, Debug)]
pub struct TreeGrove(pub Arc<TreeField>);

/// Frame inputs. Descriptor arenas stay in [`TreeGroveCatalog`]; extract never clones them.
#[derive(Resource, Clone, ExtractResource, Default)]
#[extract_app(RenderApp)]
pub struct ExtractedTreeField {
	pub settings: TreeFieldSettings,
	pub time_s: f32,
	pub prev_time_s: f32,
	pub light_direction: Vec3,
	pub shadow_mode: CanopyShadowMode,
	pub plant_count: u32,
	pub branch_count: u32,
	pub leaf_count: u32,
	pub grove_residency: bool,
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
		app.init_resource::<TreeGroveCatalog>();
		app.init_resource::<TreeCatalogOps>();
		app.init_resource::<GrassGpuCapability>();
		app.init_resource::<PrevTreeTime>();
		app.init_resource::<ExtractedTreeField>();
		app.insert_resource(GroveRetainPolicy::from_env());
		app.add_plugins(ExtractResourcePlugin::<ExtractedTreeField>::default());
		if !app.is_plugin_added::<ExtractResourcePlugin<GroveRetainPolicy>>() {
			app.add_plugins(ExtractResourcePlugin::<GroveRetainPolicy>::default());
		}
		app.add_plugins(ExtractComponentPlugin::<TreeFieldPresenter>::default());
		app.add_systems(Startup, spawn_presenter);
		app.add_systems(
			Update,
			(
				collect_catalog_ops,
				snapshot_extracted_frame,
				sync_presenter_bounds,
				sync_presenter_shadows,
			)
				.chain(),
		);
		app.add_plugins(TreeGpuPlugin);
		if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
			render_app
				.init_resource::<TreeCatalogOps>()
				.insert_resource(GroveRetainPolicy::from_env())
				.add_systems(ExtractSchedule, extract_catalog_ops);
		}
	}
}

#[derive(Resource, Default)]
struct PrevTreeTime(f32);

fn spawn_presenter(mut commands: Commands) {
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
			NoFrustumCulling,
			NotShadowCaster,
		));
	}
}

pub fn collect_catalog_ops(
	patches: Query<(Entity, &TreePatch), Changed<TreePatch>>,
	mut removed_patches: RemovedComponents<TreePatch>,
	groves: Query<(Entity, &TreeGrove), (Changed<TreeGrove>, Allow<Disabled>)>,
	mut removed_groves: RemovedComponents<TreeGrove>,
	settings: Res<TreeFieldSettings>,
	mut ops: ResMut<TreeCatalogOps>,
	mut catalog: ResMut<TreeGroveCatalog>,
) {
	let mut fresh = Vec::new();
	for id in removed_patches.read() {
		fresh.push(TreeCatalogOp::Retire { id: GroveId::from_entity(id) });
	}
	for id in removed_groves.read() {
		fresh.push(TreeCatalogOp::Retire { id: GroveId::from_entity(id) });
	}
	for (entity, patch) in &patches {
		fresh.push(admit_from_field(
			GroveId::from_entity(entity),
			TreeField::rebuild([patch], &settings),
			patch.samples.len(),
		));
	}
	for (entity, grove) in &groves {
		catalog.grove_residency = true;
		fresh.push(admit_from_grove(GroveId::from_entity(entity), Arc::clone(&grove.0)));
	}
	if fresh.is_empty() && catalog.pending.is_empty() {
		let catalog = catalog.bypass_change_detection();
		catalog.last_writes = 0;
		catalog.last_write_bytes = 0;
		catalog.last_uploads.clear();
		return;
	}
	let mut batch = std::mem::take(&mut catalog.pending);
	batch.extend(fresh);
	let CatalogApply { applied, leftover } = catalog.apply(batch, ADMISSION_BUDGET);
	ops.ops.extend(applied);
	catalog.pending = leftover;
}

fn admit_from_field(id: GroveId, field: TreeField, requested: usize) -> TreeCatalogOp {
	admit_from_arc(id, Arc::new(field), requested)
}

fn admit_from_grove(id: GroveId, field: Arc<TreeField>) -> TreeCatalogOp {
	let requested = field.plants.len();
	admit_from_arc(id, field, requested)
}

fn admit_from_arc(id: GroveId, field: Arc<TreeField>, requested: usize) -> TreeCatalogOp {
	let truncated = requested.saturating_sub(field.plants.len()) as u32;
	TreeCatalogOp::Admit { id, field, truncated_plants: truncated }
}

fn extract_catalog_ops(mut main_world: ResMut<MainWorld>, mut render: ResMut<TreeCatalogOps>) {
	let mut main = main_world.resource_mut::<TreeCatalogOps>();
	render.ops.extend(main.take());
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

fn snapshot_extracted_frame(
	mut extracted: ResMut<ExtractedTreeField>,
	catalog: Res<TreeGroveCatalog>,
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
	extracted.settings = settings.clone();
	extracted.time_s = time_s;
	extracted.prev_time_s = prev_time_s;
	extracted.light_direction = light_direction.normalize_or(Vec3::Y);
	extracted.shadow_mode = shadow_mode.map(|mode| *mode).unwrap_or_default();
	extracted.plant_count = catalog.resident_plants() as u32;
	extracted.branch_count = catalog.resident_branches() as u32;
	extracted.leaf_count = catalog.resident_leaves() as u32;
	extracted.grove_residency = catalog.grove_residency;
	if requested_world_tree_mode().is_some() && catalog.is_changed() {
		info!(
			target: "maybraid.tree_field",
			generation = catalog.generation,
			groves = catalog.live_grove_count(),
			plants = catalog.resident_plants(),
			branches = catalog.resident_branches(),
			leaves = catalog.resident_leaves(),
			truncated = catalog.truncated_plants,
			"world catalog inventory"
		);
	}
}

fn sync_presenter_bounds(
	catalog: Res<TreeGroveCatalog>,
	mut presenters: Query<&mut Aabb, With<TreeFieldPresenter>>,
) {
	if !catalog.is_changed() {
		return;
	}
	let Some((min, max)) = catalog.active_bounds() else {
		return;
	};
	let aabb = Aabb {
		center: Vec3A::from((min + max) * 0.5),
		half_extents: Vec3A::from((max - min) * 0.5),
	};
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
		let mut no_frustum = world.query::<&NoFrustumCulling>();
		assert_eq!(no_frustum.iter(world).count(), 2);
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
	fn idle_frame_does_not_enqueue_catalog_ops() {
		let mut app = App::new();
		app.init_resource::<TreeFieldSettings>()
			.init_resource::<TreeGroveCatalog>()
			.init_resource::<TreeCatalogOps>()
			.add_systems(Update, collect_catalog_ops);
		app.update();
		assert!(app.world().resource::<TreeCatalogOps>().is_empty());
		assert_eq!(app.world().resource::<TreeGroveCatalog>().last_writes, 0);
		assert_eq!(app.world().resource::<TreeGroveCatalog>().clones, 0);
	}

	#[test]
	fn admitting_one_patch_writes_only_that_grove() {
		let mut app = App::new();
		app.init_resource::<TreeFieldSettings>()
			.init_resource::<TreeGroveCatalog>()
			.init_resource::<TreeCatalogOps>()
			.add_systems(Update, collect_catalog_ops);
		app.world_mut()
			.spawn(TreePatch::isolated(1, Vec3::ZERO, TreeRecipe::storybook()));
		app.update();
		let catalog = app.world().resource::<TreeGroveCatalog>();
		assert_eq!(catalog.live_grove_count(), 1);
		assert_eq!(catalog.last_writes, 1);
		assert!(catalog.last_write_bytes > 0);
		assert_eq!(catalog.clones, 0);
		app.update();
		let catalog = app.world().resource::<TreeGroveCatalog>();
		assert_eq!(catalog.last_writes, 0);
	}

	#[test]
	fn tree_grove_admit_marks_grove_residency() {
		let mut app = App::new();
		app.init_resource::<TreeFieldSettings>()
			.init_resource::<TreeGroveCatalog>()
			.init_resource::<TreeCatalogOps>()
			.add_systems(Update, collect_catalog_ops);
		let patch = TreePatch::isolated(1, Vec3::ZERO, TreeRecipe::storybook());
		let field = TreeField::rebuild([&patch], &TreeFieldSettings::default());
		app.world_mut().spawn(TreeGrove(std::sync::Arc::new(field)));
		app.update();
		let catalog = app.world().resource::<TreeGroveCatalog>();
		assert!(catalog.grove_residency);
		assert_eq!(catalog.live_grove_count(), 1);
	}

	#[test]
	fn tree_grove_realize_admits_once_and_unrealize_retires_once() {
		let mut app = App::new();
		app.init_resource::<TreeFieldSettings>()
			.init_resource::<TreeGroveCatalog>()
			.init_resource::<TreeCatalogOps>()
			.add_systems(Update, collect_catalog_ops);
		let patch = TreePatch::isolated(1, Vec3::ZERO, TreeRecipe::storybook());
		let field = TreeField::rebuild([&patch], &TreeFieldSettings::default());
		let id = app.world_mut().spawn(TreeGrove(std::sync::Arc::new(field))).id();
		app.update();
		assert_eq!(app.world().resource::<TreeGroveCatalog>().live_grove_count(), 1);
		let admits = app
			.world()
			.resource::<TreeCatalogOps>()
			.ops
			.iter()
			.filter(|op| matches!(op, TreeCatalogOp::Admit { .. }))
			.count();
		assert_eq!(admits, 1);
		app.world_mut().entity_mut(id).despawn();
		app.update();
		assert_eq!(app.world().resource::<TreeGroveCatalog>().live_grove_count(), 0);
		let retires = app
			.world()
			.resource::<TreeCatalogOps>()
			.ops
			.iter()
			.filter(|op| matches!(op, TreeCatalogOp::Retire { .. }))
			.count();
		assert_eq!(retires, 1);
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

	fn admit_id(id: u64) -> TreeCatalogOp {
		admit_from_field(
			GroveId(id),
			TreeField::rebuild(
				[&TreePatch::isolated(1, Vec3::new(id as f32, 0.0, 0.0), TreeRecipe::storybook())],
				&TreeFieldSettings::default(),
			),
			1,
		)
	}

	#[test]
	fn mixed_batch_forwards_retire_after_budgeted_admits() {
		let mut app = App::new();
		app.init_resource::<TreeFieldSettings>()
			.init_resource::<TreeGroveCatalog>()
			.init_resource::<TreeCatalogOps>()
			.add_systems(Update, collect_catalog_ops);
		let live_y = app
			.world_mut()
			.spawn(TreePatch::isolated(1, Vec3::ZERO, TreeRecipe::storybook()))
			.id();
		app.update();
		let first = app.world_mut().resource_mut::<TreeCatalogOps>().take();
		let mut render = TreeGroveCatalog::default();
		render.apply(first, ADMISSION_BUDGET);
		assert!(app
			.world()
			.resource::<TreeGroveCatalog>()
			.contains(GroveId::from_entity(live_y)));

		let mut pending: Vec<_> = (0..ADMISSION_BUDGET as u64).map(|i| admit_id(200 + i)).collect();
		pending.push(admit_id(300));
		app.world_mut().resource_mut::<TreeGroveCatalog>().pending = pending;
		app.world_mut().entity_mut(live_y).despawn();
		app.update();

		let y_id = GroveId::from_entity(live_y);
		let catalog = app.world().resource::<TreeGroveCatalog>();
		assert!(!catalog.contains(y_id));
		assert!(!catalog.contains(GroveId(300)));
		for i in 0..ADMISSION_BUDGET as u64 {
			assert!(catalog.contains(GroveId(200 + i)), "{i}");
		}
		assert_eq!(catalog.pending.len(), 1);
		assert!(matches!(
			catalog.pending.first(),
			Some(TreeCatalogOp::Admit { id, .. }) if *id == GroveId(300)
		));

		let forwarded = app.world().resource::<TreeCatalogOps>().ops.clone();
		assert!(forwarded
			.iter()
			.any(|op| matches!(op, TreeCatalogOp::Retire { id } if *id == y_id)));
		assert!(forwarded.iter().all(|op| match op {
			TreeCatalogOp::Admit { id, .. } | TreeCatalogOp::Retire { id } => *id != GroveId(300),
		}));

		let CatalogApply { leftover, .. } = render.apply(forwarded, ADMISSION_BUDGET);
		assert!(leftover.is_empty());
		let main = app.world().resource::<TreeGroveCatalog>();
		assert_eq!(main.contains(y_id), render.contains(y_id));
		assert_eq!(main.contains(GroveId(300)), render.contains(GroveId(300)));
		for i in 0..ADMISSION_BUDGET as u64 {
			assert_eq!(main.contains(GroveId(200 + i)), render.contains(GroveId(200 + i)), "{i}");
		}
	}

	#[test]
	fn mixed_batch_cancels_pending_admit_on_retire() {
		let mut app = App::new();
		app.init_resource::<TreeFieldSettings>()
			.init_resource::<TreeGroveCatalog>()
			.init_resource::<TreeCatalogOps>()
			.add_systems(Update, collect_catalog_ops);
		let mut pending: Vec<_> = (0..ADMISSION_BUDGET as u64).map(|i| admit_id(200 + i)).collect();
		pending.push(admit_id(300));
		pending.push(TreeCatalogOp::Retire { id: GroveId(300) });
		app.world_mut().resource_mut::<TreeGroveCatalog>().pending = pending;
		app.update();

		let catalog = app.world().resource::<TreeGroveCatalog>();
		assert!(catalog.pending.is_empty());
		assert!(!catalog.contains(GroveId(300)));
		let forwarded = app.world().resource::<TreeCatalogOps>().ops.clone();
		assert!(forwarded
			.iter()
			.any(|op| matches!(op, TreeCatalogOp::Retire { id } if *id == GroveId(300))));
		assert!(forwarded
			.iter()
			.all(|op| !matches!(op, TreeCatalogOp::Admit { id, .. } if *id == GroveId(300))));

		let mut render = TreeGroveCatalog::default();
		let CatalogApply { leftover, .. } = render.apply(forwarded, ADMISSION_BUDGET);
		assert!(leftover.is_empty());
		assert!(!render.contains(GroveId(300)));
	}

	#[test]
	fn grove_component_and_render_catalog_share_field_arc() {
		let mut app = App::new();
		app.init_resource::<TreeFieldSettings>()
			.init_resource::<TreeGroveCatalog>()
			.init_resource::<TreeCatalogOps>()
			.add_systems(Update, collect_catalog_ops);
		let patch = TreePatch::isolated(1, Vec3::ZERO, TreeRecipe::storybook());
		let field = TreeField::rebuild([&patch], &TreeFieldSettings::default());
		let grove = Arc::new(field);
		let entity = app.world_mut().spawn(TreeGrove(Arc::clone(&grove))).id();
		app.update();
		let ops = app.world_mut().resource_mut::<TreeCatalogOps>().take();
		let mut render = TreeGroveCatalog::render();
		render.apply(ops, ADMISSION_BUDGET);
		let id = GroveId::from_entity(entity);
		let catalog_arc = render.source(id).expect("render catalog holds the grove");
		assert!(Arc::ptr_eq(catalog_arc, &grove), "render catalog must not deep-copy the field");
		let component = app.world().entity(entity).get::<TreeGrove>().expect("grove component");
		assert!(Arc::ptr_eq(&component.0, catalog_arc));
		assert!(
			Arc::strong_count(catalog_arc) >= 3,
			"entity, render catalog, and test handle share one allocation"
		);
	}
}
