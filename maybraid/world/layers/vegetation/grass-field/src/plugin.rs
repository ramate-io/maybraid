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
use vegetation_shaders::VegetationWeslPlugin;

use crate::capability::GrassGpuCapability;
use crate::diagnostics::GrassFieldStats;
use crate::field::{GrassField, GrassFieldSettings};
use crate::gpu::GrassGpuPlugin;
use crate::patch::GrassPatch;
use crate::shaders::GrassShadersPlugin;

/// Marker extracted into the render world. CPU frustum cull uses the conservative AABB.
#[derive(Component, Clone, ExtractComponent)]
#[require(VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<GrassFieldPresenter>)]
#[extract_app(RenderApp)]
pub struct GrassFieldPresenter;

/// Snapshot copied into the render world each frame.
#[derive(Resource, Clone, ExtractResource, Default)]
#[extract_app(RenderApp)]
pub struct ExtractedGrassField {
	pub field: GrassField,
	pub settings: GrassFieldSettings,
	pub time_s: f32,
	pub prev_time_s: f32,
	pub light_direction: Vec3,
	pub shadow_mode: CanopyShadowMode,
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
		app.init_resource::<GrassGpuCapability>();
		app.init_resource::<PrevGrassTime>();
		app.init_resource::<ExtractedGrassField>();
		app.add_plugins(ExtractResourcePlugin::<ExtractedGrassField>::default());
		app.add_plugins(ExtractComponentPlugin::<GrassFieldPresenter>::default());
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
		app.add_plugins(GrassGpuPlugin);
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

fn rebuild_field(
	changed: Query<(), Changed<GrassPatch>>,
	added: Query<(), Added<GrassPatch>>,
	removed: RemovedComponents<GrassPatch>,
	all_patches: Query<&GrassPatch>,
	settings: Res<GrassFieldSettings>,
	mut field: ResMut<GrassField>,
) {
	if changed.is_empty() && added.is_empty() && removed.is_empty() && !settings.is_changed() {
		return;
	}
	let generation = field.generation.wrapping_add(1);
	*field = GrassField::rebuild(all_patches.iter(), &settings).with_generation(generation);
}

fn snapshot_extracted_field(
	mut commands: Commands,
	field: Res<GrassField>,
	settings: Res<GrassFieldSettings>,
	time: Res<Time>,
	mut prev: ResMut<PrevGrassTime>,
	lights: Query<&GlobalTransform, With<DirectionalLight>>,
	shadow_mode: Option<Res<CanopyShadowMode>>,
) {
	let time_s = time.elapsed_secs();
	let prev_time_s = prev.0;
	prev.0 = time_s;
	let light_direction = lights
		.iter()
		.next()
		.map(|transform| *transform.back())
		.unwrap_or(Vec3::new(0.4, 0.8, 0.3));
	commands.insert_resource(ExtractedGrassField {
		field: field.clone(),
		settings: settings.clone(),
		time_s,
		prev_time_s,
		light_direction: light_direction.normalize_or(Vec3::Y),
		shadow_mode: shadow_mode.map(|mode| *mode).unwrap_or_default(),
	});
}

fn sync_presenter_bounds(
	field: Res<GrassField>,
	mut presenters: Query<&mut Aabb, With<GrassFieldPresenter>>,
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
	use crate::patch::GrassPatch;

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
