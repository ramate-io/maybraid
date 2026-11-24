//! Main-world plugin: catalog admits, one grass presenter, render-app wiring.

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
use vegetation_shaders::VegetationWeslPlugin;

use crate::capability::GrassGpuCapability;
use crate::catalog::{CatalogApply, GroveId, ADMISSION_BUDGET};
use crate::diagnostics::GrassFieldStats;
use crate::field::{GrassField, GrassFieldSettings};
use crate::gpu::GrassGpuPlugin;
use crate::grove_catalog::{GrassCatalogOp, GrassCatalogOps, GrassGroveCatalog};
use crate::patch::GrassPatch;
use crate::shaders::GrassShadersPlugin;

/// Marker extracted into the render world. CPU frustum cull uses the catalog AABB.
#[derive(Component, Clone, ExtractComponent)]
#[require(VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<GrassFieldPresenter>)]
#[extract_app(RenderApp)]
pub struct GrassFieldPresenter;

/// Immutable snapshot used as an admit source. Presenters are not parented here.
#[derive(Component, Clone, Debug)]
pub struct GrassGrove(pub Arc<GrassField>);

/// Frame inputs. Blade arenas stay in [`GrassGroveCatalog`].
#[derive(Resource, Clone, ExtractResource, Default)]
#[extract_app(RenderApp)]
pub struct ExtractedGrassField {
	pub settings: GrassFieldSettings,
	pub time_s: f32,
	pub prev_time_s: f32,
	pub light_direction: Vec3,
	pub shadow_mode: CanopyShadowMode,
	pub blade_count: u32,
	pub patch_count: u32,
	pub grove_residency: bool,
}

/// Registers grass field rendering. Existing turf/frond materials are untouched.
pub struct GrassFieldPlugin;

impl Plugin for GrassFieldPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VegetationWeslPlugin>() {
			app.add_plugins(VegetationWeslPlugin);
		}
		app.add_plugins(GrassShadersPlugin);
		app.insert_resource(GrassFieldSettings::from_env());
		app.init_resource::<GrassField>();
		app.init_resource::<GrassFieldStats>();
		app.init_resource::<GrassGroveCatalog>();
		app.init_resource::<GrassCatalogOps>();
		app.init_resource::<GrassGpuCapability>();
		app.init_resource::<PrevGrassTime>();
		app.init_resource::<ExtractedGrassField>();
		crate::grove_retain::GroveRetainPolicy::install(app);
		app.add_plugins(ExtractResourcePlugin::<ExtractedGrassField>::default());
		app.add_plugins(ExtractComponentPlugin::<GrassFieldPresenter>::default());
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
		app.add_plugins(GrassGpuPlugin);
		if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
			render_app
				.init_resource::<GrassCatalogOps>()
				.add_systems(ExtractSchedule, extract_catalog_ops);
		}
	}
}

#[derive(Resource, Default)]
struct PrevGrassTime(f32);

fn spawn_presenter(mut commands: Commands) {
	commands.spawn((
		Name::new("GrassFieldPresenter"),
		GrassFieldPresenter,
		Visibility::default(),
		Transform::default(),
		Aabb::default(),
		NoFrustumCulling,
		NotShadowCaster,
	));
}

fn sync_presenter_shadows(
	mode: Option<Res<CanopyShadowMode>>,
	mut commands: Commands,
	casters: Query<Entity, (With<GrassFieldPresenter>, Without<NotShadowCaster>)>,
	non_casters: Query<Entity, (With<GrassFieldPresenter>, With<NotShadowCaster>)>,
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

pub fn collect_catalog_ops(
	patches: Query<(Entity, &GrassPatch), Changed<GrassPatch>>,
	mut removed_patches: RemovedComponents<GrassPatch>,
	groves: Query<(Entity, &GrassGrove), (Changed<GrassGrove>, Allow<Disabled>)>,
	mut removed_groves: RemovedComponents<GrassGrove>,
	settings: Res<GrassFieldSettings>,
	mut ops: ResMut<GrassCatalogOps>,
	mut catalog: ResMut<GrassGroveCatalog>,
) {
	let mut fresh = Vec::new();
	for id in removed_patches.read() {
		fresh.push(GrassCatalogOp::Retire { id: GroveId::from_entity(id) });
	}
	for id in removed_groves.read() {
		fresh.push(GrassCatalogOp::Retire { id: GroveId::from_entity(id) });
	}
	for (entity, patch) in &patches {
		let requested = patch.descriptors().len();
		fresh.push(admit_from_field(
			GroveId::from_entity(entity),
			GrassField::rebuild([patch], &settings),
			requested,
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

fn admit_from_field(id: GroveId, field: GrassField, requested: usize) -> GrassCatalogOp {
	admit_from_arc(id, Arc::new(field), requested)
}

fn admit_from_grove(id: GroveId, field: Arc<GrassField>) -> GrassCatalogOp {
	let requested = field.blades.len();
	admit_from_arc(id, field, requested)
}

fn admit_from_arc(id: GroveId, field: Arc<GrassField>, requested: usize) -> GrassCatalogOp {
	let truncated = requested.saturating_sub(field.blades.len()) as u32;
	GrassCatalogOp::Admit { id, field, truncated_plants: truncated }
}

fn extract_catalog_ops(mut main_world: ResMut<MainWorld>, mut render: ResMut<GrassCatalogOps>) {
	let mut main = main_world.resource_mut::<GrassCatalogOps>();
	render.ops.extend(main.take());
}

fn snapshot_extracted_frame(
	mut extracted: ResMut<ExtractedGrassField>,
	catalog: Res<GrassGroveCatalog>,
	settings: Res<GrassFieldSettings>,
	time: Res<Time>,
	mut prev: ResMut<PrevGrassTime>,
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
	extracted.blade_count = catalog.resident_blades() as u32;
	extracted.patch_count = catalog.patches.len() as u32;
	extracted.grove_residency = catalog.grove_residency;
}

fn sync_presenter_bounds(
	catalog: Res<GrassGroveCatalog>,
	mut presenters: Query<&mut Aabb, With<GrassFieldPresenter>>,
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

	#[test]
	fn rebuild_uploads_compact_descriptors_not_meshes() -> anyhow::Result<()> {
		let patch = GrassPatch::isolated_clump(1, Vec3::ZERO, 16);
		let settings = GrassFieldSettings::default();
		let field = GrassField::rebuild([&patch], &settings);
		assert!(!field.blades.is_empty());
		assert_eq!(field.patches.len(), 1);
		assert!(field.blades.len() <= 16);
		Ok(())
	}

	fn admit_id(id: u64) -> GrassCatalogOp {
		let patch =
			GrassPatch::isolated_clump(1, Vec3::new(id as f32, 0.0, 0.0), 8).with_grove_residency();
		admit_from_field(
			GroveId(id),
			GrassField::rebuild([&patch], &GrassFieldSettings::default()),
			8,
		)
	}

	#[test]
	fn mixed_batch_forwards_retire_after_budgeted_admits() {
		let mut app = App::new();
		app.init_resource::<GrassFieldSettings>()
			.init_resource::<GrassGroveCatalog>()
			.init_resource::<GrassCatalogOps>()
			.add_systems(Update, collect_catalog_ops);
		let live_y = app.world_mut().spawn(GrassPatch::isolated_clump(1, Vec3::ZERO, 8)).id();
		app.update();
		let first = app.world_mut().resource_mut::<GrassCatalogOps>().take();
		let mut render = GrassGroveCatalog::default();
		render.apply(first, ADMISSION_BUDGET);
		assert!(app
			.world()
			.resource::<GrassGroveCatalog>()
			.contains(GroveId::from_entity(live_y)));

		let mut pending: Vec<_> = (0..ADMISSION_BUDGET as u64).map(|i| admit_id(200 + i)).collect();
		pending.push(admit_id(300));
		app.world_mut().resource_mut::<GrassGroveCatalog>().pending = pending;
		app.world_mut().entity_mut(live_y).despawn();
		app.update();

		let y_id = GroveId::from_entity(live_y);
		let catalog = app.world().resource::<GrassGroveCatalog>();
		assert!(!catalog.contains(y_id));
		assert!(!catalog.contains(GroveId(300)));
		for i in 0..ADMISSION_BUDGET as u64 {
			assert!(catalog.contains(GroveId(200 + i)), "{i}");
		}
		assert!(matches!(
			catalog.pending.first(),
			Some(GrassCatalogOp::Admit { id, .. }) if *id == GroveId(300)
		));

		let forwarded = app.world().resource::<GrassCatalogOps>().ops.clone();
		assert!(forwarded
			.iter()
			.any(|op| matches!(op, GrassCatalogOp::Retire { id } if *id == y_id)));
		assert!(forwarded.iter().all(|op| match op {
			GrassCatalogOp::Admit { id, .. } | GrassCatalogOp::Retire { id } => *id != GroveId(300),
		}));

		let CatalogApply { leftover, .. } = render.apply(forwarded, ADMISSION_BUDGET);
		assert!(leftover.is_empty());
		let main = app.world().resource::<GrassGroveCatalog>();
		assert_eq!(main.contains(y_id), render.contains(y_id));
		assert_eq!(main.contains(GroveId(300)), render.contains(GroveId(300)));
	}

	#[test]
	fn mixed_batch_cancels_pending_admit_on_retire() {
		let mut app = App::new();
		app.init_resource::<GrassFieldSettings>()
			.init_resource::<GrassGroveCatalog>()
			.init_resource::<GrassCatalogOps>()
			.add_systems(Update, collect_catalog_ops);
		let mut pending: Vec<_> = (0..ADMISSION_BUDGET as u64).map(|i| admit_id(200 + i)).collect();
		pending.push(admit_id(300));
		pending.push(GrassCatalogOp::Retire { id: GroveId(300) });
		app.world_mut().resource_mut::<GrassGroveCatalog>().pending = pending;
		app.update();

		let catalog = app.world().resource::<GrassGroveCatalog>();
		assert!(catalog.pending.is_empty());
		assert!(!catalog.contains(GroveId(300)));
		let forwarded = app.world().resource::<GrassCatalogOps>().ops.clone();
		assert!(forwarded
			.iter()
			.any(|op| matches!(op, GrassCatalogOp::Retire { id } if *id == GroveId(300))));
		assert!(forwarded
			.iter()
			.all(|op| !matches!(op, GrassCatalogOp::Admit { id, .. } if *id == GroveId(300))));

		let mut render = GrassGroveCatalog::default();
		let CatalogApply { leftover, .. } = render.apply(forwarded, ADMISSION_BUDGET);
		assert!(leftover.is_empty());
		assert!(!render.contains(GroveId(300)));
	}

	#[test]
	fn grove_component_and_render_catalog_share_field_arc() {
		let mut app = App::new();
		app.init_resource::<GrassFieldSettings>()
			.init_resource::<GrassGroveCatalog>()
			.init_resource::<GrassCatalogOps>()
			.add_systems(Update, collect_catalog_ops);
		let patch = GrassPatch::isolated_clump(1, Vec3::ZERO, 8).with_grove_residency();
		let field = GrassField::rebuild([&patch], &GrassFieldSettings::default());
		let grove = Arc::new(field);
		let entity = app.world_mut().spawn(GrassGrove(Arc::clone(&grove))).id();
		app.update();
		let ops = app.world_mut().resource_mut::<GrassCatalogOps>().take();
		let mut render = GrassGroveCatalog::render();
		render.apply(ops, ADMISSION_BUDGET);
		let id = GroveId::from_entity(entity);
		let catalog_arc = render.source(id).expect("render catalog holds the grove");
		assert!(Arc::ptr_eq(catalog_arc, &grove), "render catalog must not deep-copy the field");
		let component = app.world().entity(entity).get::<GrassGrove>().expect("grove component");
		assert!(Arc::ptr_eq(&component.0, catalog_arc));
		assert!(
			Arc::strong_count(catalog_arc) >= 3,
			"entity, render catalog, and test handle share one allocation"
		);
	}
}
