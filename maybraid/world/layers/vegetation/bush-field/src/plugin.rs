//! Main-world plugin: catalog admits, two kind presenters, render-app wiring.

use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{self, NoFrustumCulling, VisibilityClass};
use bevy::light::NotShadowCaster;
use bevy::math::Vec3A;
use bevy::prelude::*;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::{ExtractSchedule, MainWorld, RenderApp};
use vegetation_components::CanopyShadowMode;
use vegetation_grass_field::{GrassGpuCapability, GroveId, ADMISSION_BUDGET};
use vegetation_shaders::VegetationWeslPlugin;

use crate::bounds::BushBounds;
use crate::catalog::{BushCatalogOp, BushCatalogOps, BushGroveCatalog};
use crate::diagnostics::BushFieldStats;
use crate::field::{BushField, BushFieldSettings};
use crate::gpu::BushGpuPlugin;
use crate::patch::BushPatch;
use crate::shaders::BushShadersPlugin;

/// Marker extracted into the render world. CPU frustum cull uses the catalog AABB.
#[derive(Component, Clone, Copy, Debug, Eq, ExtractComponent, PartialEq)]
#[require(VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<BushFieldPresenter>)]
#[extract_app(RenderApp)]
pub(crate) enum BushFieldPresenter {
	Wood,
	Foliage,
}

/// Frame inputs. Descriptor arenas stay in [`BushGroveCatalog`].
#[derive(Resource, Clone, ExtractResource, Default)]
#[extract_app(RenderApp)]
pub struct ExtractedBushField {
	pub settings: BushFieldSettings,
	pub time_s: f32,
	pub prev_time_s: f32,
	pub light_direction: Vec3,
	pub shadow_mode: CanopyShadowMode,
	pub plant_count: u32,
	pub branch_count: u32,
	pub leaf_count: u32,
	pub grove_residency: bool,
}

/// Registers GPU bush field rendering. Existing kit bushes stay available.
pub struct BushFieldPlugin;

impl Plugin for BushFieldPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VegetationWeslPlugin>() {
			app.add_plugins(VegetationWeslPlugin);
		}
		app.add_plugins(BushShadersPlugin);
		app.insert_resource(BushFieldSettings::from_env());
		app.init_resource::<BushField>();
		app.init_resource::<BushFieldStats>();
		app.init_resource::<BushGroveCatalog>();
		app.init_resource::<BushCatalogOps>();
		app.init_resource::<GrassGpuCapability>();
		app.init_resource::<PrevBushTime>();
		app.init_resource::<ExtractedBushField>();
		app.add_plugins(ExtractResourcePlugin::<ExtractedBushField>::default());
		app.add_plugins(ExtractComponentPlugin::<BushFieldPresenter>::default());
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
		app.add_plugins(BushGpuPlugin);
		if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
			render_app
				.init_resource::<BushCatalogOps>()
				.add_systems(ExtractSchedule, extract_catalog_ops);
		}
	}
}

#[derive(Resource, Default)]
struct PrevBushTime(f32);

fn spawn_presenter(mut commands: Commands) {
	for (name, kind) in [
		("BushFieldWood", BushFieldPresenter::Wood),
		("BushFieldFoliage", BushFieldPresenter::Foliage),
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

fn collect_catalog_ops(
	patches: Query<(Entity, &BushPatch), Changed<BushPatch>>,
	mut removed: RemovedComponents<BushPatch>,
	settings: Res<BushFieldSettings>,
	mut ops: ResMut<BushCatalogOps>,
	mut catalog: ResMut<BushGroveCatalog>,
) {
	let mut fresh = Vec::new();
	for id in removed.read() {
		fresh.push(BushCatalogOp::Retire { id: GroveId::from_entity(id) });
	}
	for (entity, patch) in &patches {
		if patch.grove_residency {
			catalog.grove_residency = true;
		}
		fresh.push(admit_from_field(
			GroveId::from_entity(entity),
			BushField::rebuild([patch], &settings),
			patch.samples.len(),
		));
	}
	if fresh.is_empty() && catalog.pending.is_empty() {
		catalog.last_writes = 0;
		catalog.last_write_bytes = 0;
		catalog.last_uploads.clear();
		return;
	}
	let mut batch = std::mem::take(&mut catalog.pending);
	batch.extend(fresh);
	let leftover = catalog.apply(batch.iter().cloned(), ADMISSION_BUDGET);
	let applied = batch.len().saturating_sub(leftover.len());
	ops.ops.extend(batch.into_iter().take(applied));
	catalog.pending = leftover;
}

fn admit_from_field(id: GroveId, field: BushField, requested: usize) -> BushCatalogOp {
	let truncated = requested.saturating_sub(field.plants.len()) as u32;
	let (aabb_min, aabb_max) = if field.bounds.is_empty() {
		(Vec3::ZERO, Vec3::ZERO)
	} else {
		(field.bounds.min, field.bounds.max)
	};
	BushCatalogOp::Admit {
		id,
		plants: field.plants,
		branches: field.branches,
		leaves: field.leaves,
		aabb_min,
		aabb_max,
		seed: 0,
		truncated_plants: truncated,
	}
}

fn extract_catalog_ops(mut main_world: ResMut<MainWorld>, mut render: ResMut<BushCatalogOps>) {
	let mut main = main_world.resource_mut::<BushCatalogOps>();
	render.ops.extend(main.take());
}

fn sync_presenter_shadows(
	mode: Option<Res<CanopyShadowMode>>,
	mut commands: Commands,
	casters: Query<Entity, (With<BushFieldPresenter>, Without<NotShadowCaster>)>,
	non_casters: Query<Entity, (With<BushFieldPresenter>, With<NotShadowCaster>)>,
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
	mut extracted: ResMut<ExtractedBushField>,
	catalog: Res<BushGroveCatalog>,
	settings: Res<BushFieldSettings>,
	time: Res<Time>,
	mut prev: ResMut<PrevBushTime>,
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
	extracted.plant_count = catalog.plants.len() as u32;
	extracted.branch_count = catalog.branches.len() as u32;
	extracted.leaf_count = catalog.leaves.len() as u32;
	extracted.grove_residency = catalog.grove_residency;
}

fn sync_presenter_bounds(
	catalog: Res<BushGroveCatalog>,
	settings: Res<BushFieldSettings>,
	mut presenters: Query<&mut Aabb, With<BushFieldPresenter>>,
) {
	if catalog.is_empty() {
		return;
	}
	let mut bounds = BushBounds::empty();
	for plant in catalog.plants.iter().filter(|plant| plant.is_active()) {
		bounds = bounds.union(BushBounds::from_plant(plant, settings.wind.strength_m));
	}
	if bounds.is_empty() {
		return;
	}
	let aabb = Aabb {
		center: Vec3A::from((bounds.min + bounds.max) * 0.5),
		half_extents: Vec3A::from((bounds.max - bounds.min) * 0.5),
	};
	for mut existing in &mut presenters {
		*existing = aabb;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::patch::BushPatch;
	use crate::recipe::BushRecipe;

	#[test]
	fn wood_and_foliage_have_separate_presenter_entities() {
		let mut app = App::new();
		app.add_systems(Startup, spawn_presenter);
		app.update();
		let world = app.world_mut();
		let mut query = world.query::<&BushFieldPresenter>();
		let kinds: Vec<_> = query.iter(world).copied().collect();
		assert_eq!(kinds.len(), 2);
		assert!(kinds.contains(&BushFieldPresenter::Wood));
		assert!(kinds.contains(&BushFieldPresenter::Foliage));
		let mut no_frustum = world.query::<&NoFrustumCulling>();
		assert_eq!(no_frustum.iter(world).count(), 2);
	}

	#[test]
	fn rebuild_uploads_compact_descriptors_not_meshes() -> anyhow::Result<()> {
		let patch = BushPatch::isolated(1, Vec3::ZERO, BushRecipe::small_scrub());
		let settings = BushFieldSettings::default();
		let field = BushField::rebuild([&patch], &settings);
		assert!(!field.plants.is_empty());
		assert!(!field.branches.is_empty());
		assert_eq!(field.patches.len(), 1);
		Ok(())
	}
}
