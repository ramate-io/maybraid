//! Main-world plugin: patch upload, presenter AABB, and render-app wiring.

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

use crate::diagnostics::BushFieldStats;
use crate::field::{BushField, BushFieldSettings};
use crate::gpu::BushGpuPlugin;
use crate::patch::BushPatch;
use crate::shaders::BushShadersPlugin;

/// Marker extracted into the render world. CPU frustum cull uses the conservative AABB.
#[derive(Component, Clone, ExtractComponent)]
#[require(VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<BushFieldPresenter>)]
#[extract_app(RenderApp)]
pub struct BushFieldPresenter;

/// Snapshot copied into the render world each frame.
#[derive(Resource, Clone, ExtractResource, Default)]
#[extract_app(RenderApp)]
pub struct ExtractedBushField {
	pub field: BushField,
	pub settings: BushFieldSettings,
	pub time_s: f32,
	pub prev_time_s: f32,
	pub light_direction: Vec3,
	pub shadow_mode: CanopyShadowMode,
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
		app.init_resource::<GrassGpuCapability>();
		app.init_resource::<PrevBushTime>();
		app.init_resource::<ExtractedBushField>();
		app.add_plugins(ExtractResourcePlugin::<ExtractedBushField>::default());
		app.add_plugins(ExtractComponentPlugin::<BushFieldPresenter>::default());
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
		app.add_plugins(BushGpuPlugin);
	}
}

#[derive(Resource, Default)]
struct PrevBushTime(f32);

fn spawn_presenter(mut commands: Commands) {
	commands.spawn((
		Name::new("BushFieldPresenter"),
		BushFieldPresenter,
		Visibility::default(),
		Transform::default(),
		Aabb::default(),
		NotShadowCaster,
	));
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

fn rebuild_field(
	changed: Query<(), Changed<BushPatch>>,
	added: Query<(), Added<BushPatch>>,
	removed: RemovedComponents<BushPatch>,
	all_patches: Query<&BushPatch>,
	settings: Res<BushFieldSettings>,
	mut field: ResMut<BushField>,
) {
	if changed.is_empty() && added.is_empty() && removed.is_empty() && !settings.is_changed() {
		return;
	}
	let generation = field.generation.wrapping_add(1);
	*field = BushField::rebuild(all_patches.iter(), &settings).with_generation(generation);
}

fn snapshot_extracted_field(
	mut commands: Commands,
	field: Res<BushField>,
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
	commands.insert_resource(ExtractedBushField {
		field: field.clone(),
		settings: settings.clone(),
		time_s,
		prev_time_s,
		light_direction: light_direction.normalize_or(Vec3::Y),
		shadow_mode: shadow_mode.map(|mode| *mode).unwrap_or_default(),
	});
}

fn sync_presenter_bounds(
	field: Res<BushField>,
	mut presenters: Query<&mut Aabb, With<BushFieldPresenter>>,
) {
	if field.is_empty() {
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
	use crate::patch::BushPatch;
	use crate::recipe::BushRecipe;

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
