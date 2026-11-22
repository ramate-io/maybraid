//! Main-world plugin: catalog admits, one grass presenter, render-app wiring.

use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{self, NoFrustumCulling, VisibilityClass};
use bevy::light::NotShadowCaster;
use bevy::math::Vec3A;
use bevy::prelude::*;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::{ExtractSchedule, MainWorld, RenderApp};
use vegetation_components::CanopyShadowMode;
use vegetation_shaders::VegetationWeslPlugin;

use crate::bounds::GrassBounds;
use crate::capability::GrassGpuCapability;
use crate::catalog::{GroveId, ADMISSION_BUDGET};
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

fn collect_catalog_ops(
	patches: Query<(Entity, &GrassPatch), Changed<GrassPatch>>,
	mut removed: RemovedComponents<GrassPatch>,
	settings: Res<GrassFieldSettings>,
	mut ops: ResMut<GrassCatalogOps>,
	mut catalog: ResMut<GrassGroveCatalog>,
) {
	let mut fresh = Vec::new();
	for id in removed.read() {
		fresh.push(GrassCatalogOp::Retire { id: GroveId::from_entity(id) });
	}
	for (entity, patch) in &patches {
		let requested = patch.descriptors().len();
		let field = GrassField::rebuild([patch], &settings);
		let truncated = requested.saturating_sub(field.blades.len()) as u32;
		let (aabb_min, aabb_max) = if field.bounds.is_empty() {
			(Vec3::ZERO, Vec3::ZERO)
		} else {
			(field.bounds.min, field.bounds.max)
		};
		fresh.push(GrassCatalogOp::Admit {
			id: GroveId::from_entity(entity),
			blades: field.blades,
			aabb_min,
			aabb_max,
			seed: patch.seed,
			truncated_plants: truncated,
			grove_residency: patch.grove_residency,
		});
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
	extracted.blade_count = catalog.blades.len() as u32;
	extracted.patch_count = catalog.patches.len() as u32;
	extracted.grove_residency = catalog.grove_residency;
}

fn sync_presenter_bounds(
	catalog: Res<GrassGroveCatalog>,
	mut presenters: Query<&mut Aabb, With<GrassFieldPresenter>>,
) {
	if catalog.is_empty() {
		return;
	}
	let mut bounds = GrassBounds::empty();
	for header in catalog.patches.iter().filter(|header| header.is_active()) {
		bounds = bounds.union(GrassBounds { min: header.aabb_min, max: header.aabb_max });
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
}
