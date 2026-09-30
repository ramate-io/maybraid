//! Richmond's complete development catalog on Durham terrain.
//!
//! Urbanization generate / present lives in `urbanization-layer-model` and
//! `urbanization-layer-presentation`. Vegetation lives in the vegetation layer
//! plugins. This crate is playground chrome (camera, commands, UI).

pub mod camera;
pub mod commands;
mod ui;

pub use camera::CameraController;
pub use commands::{DevelopmentFocus, PlaygroundCommand, PlaygroundStartup, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use camera::{camera_controller, release_modifiers_on_focus_change, setup_camera};
use commands::{
	RequestDevelopmentFocus, RequestLikelihood, RequestMeshStats, RequestRebuild, RequestSeed,
	RequestTerrainRadius,
};
use durham_terrain_models::{
	JerseyStampConfigs, MarazionWatershedConfigs, TerrainCellLayout, TerrainConfig,
	TerrainMeshLodBand, TerrainPresentationAssets, TerrainPresentationDirty, WorldBaseTerrain,
};
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText};
use richmond_development_models::DevelopmentConfig;
use std::f32::consts::PI;
use urbanization_layer_model::{DevelopmentFocus as LayerFocus, UrbanizationLayerConfig};

const DEFAULT_TERRAIN_RADIUS: i32 = 2;

fn playground_lod_bands(half_extent: i32) -> Vec<TerrainMeshLodBand> {
	vec![TerrainMeshLodBand { max_radius_cells: half_extent.max(1), res_2: 5 }]
}

#[derive(Resource, Clone)]
pub struct PlaygroundConfig {
	pub terrain_radius: i32,
}

impl Default for PlaygroundConfig {
	fn default() -> Self {
		Self { terrain_radius: DEFAULT_TERRAIN_RADIUS }
	}
}

/// Richmond developments on Durham terrain.
///
/// Assemblers add the urbanization layer plugins. This plugin keeps camera,
/// commands, and UI.
pub struct DevelopmentsOnTerrainPlugin {
	pub config: PlaygroundConfig,
	/// When false, the caller owns the command drawer / CLI.
	pub commands: bool,
}

impl Default for DevelopmentsOnTerrainPlugin {
	fn default() -> Self {
		Self { config: PlaygroundConfig::default(), commands: true }
	}
}

impl Plugin for DevelopmentsOnTerrainPlugin {
	fn build(&self, app: &mut App) {
		if self.commands {
			app.add_plugins(
				GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config())
					.with_drawer_config(GameCommandDrawerConfig {
						open_at_start: false,
						toggle_keys: vec![KeyCode::F1, KeyCode::KeyY],
						..default()
					}),
			);
			app.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
				.add_systems(Startup, (setup_camera, setup_lighting))
				.add_systems(
					Update,
					(
						release_modifiers_on_focus_change.before(camera_controller),
						camera_controller,
						apply_commands.after(capture_command_line_input::<PlaygroundCommand>),
						apply_mesh_stats,
						ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
					),
				);
		}

		app.insert_resource(self.config.clone());
	}
}

fn setup_lighting(mut commands: Commands) {
	commands.insert_resource(GlobalAmbientLight { brightness: 450.0, ..default() });
	commands.spawn((
		DirectionalLight { illuminance: 12_000.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 2_500.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}

#[derive(SystemParam)]
struct ApplyCommandStores<'w> {
	playground: ResMut<'w, PlaygroundConfig>,
	layout: ResMut<'w, TerrainCellLayout>,
	assets: ResMut<'w, TerrainPresentationAssets>,
	config: ResMut<'w, TerrainConfig>,
	jersey: ResMut<'w, JerseyStampConfigs>,
	marazion: ResMut<'w, MarazionWatershedConfigs>,
	world_base: ResMut<'w, WorldBaseTerrain>,
	development: ResMut<'w, DevelopmentConfig>,
	urban: ResMut<'w, UrbanizationLayerConfig>,
	dirty: ResMut<'w, TerrainPresentationDirty>,
	status: ResMut<'w, GameCommandStatusText>,
}

fn apply_commands(
	mut commands: Commands,
	mut stores: ApplyCommandStores,
	seeds: Query<(Entity, &RequestSeed)>,
	likelihoods: Query<(Entity, &RequestLikelihood)>,
	focuses: Query<(Entity, &RequestDevelopmentFocus)>,
	radii: Query<(Entity, &RequestTerrainRadius)>,
	rebuild: Query<Entity, With<RequestRebuild>>,
) {
	let ApplyCommandStores {
		playground,
		layout,
		assets,
		config,
		jersey,
		marazion,
		world_base,
		development,
		urban,
		dirty,
		status,
	} = &mut stores;
	for (entity, request) in &seeds {
		config.seed = request.0;
		assets.config.seed = request.0;
		**jersey = JerseyStampConfigs::from_world_seed(request.0);
		**marazion = MarazionWatershedConfigs::default().with_seed(request.0);
		world_base.0 = durham_terrain_models::BaseTerrainNoise::from_config(config);
		development.seed = request.0;
		dirty.0 = true;
		status.0 = format!("seed {} (regen)", request.0);
		commands.entity(entity).despawn();
	}
	for (entity, request) in &likelihoods {
		development.likelihood = request.0.clamp(0.0, 1.0);
		dirty.0 = true;
		status.0 = format!("likelihood {:.2} (regen)", development.likelihood);
		commands.entity(entity).despawn();
	}
	for (entity, request) in &focuses {
		request.0.apply(development);
		urban.focus_development = (request.0 != LayerFocus::All).then_some(request.0);
		development.use_urbanization =
			urban.urbanization.is_some() || urban.focus_development.is_none();
		dirty.0 = true;
		status.0 = format!("focus-development {} (regen)", request.0);
		commands.entity(entity).despawn();
	}
	for (entity, request) in &radii {
		let cells = request.0.max(1);
		playground.terrain_radius = cells;
		let r = cells.max(1);
		let n = (2 * r) as u32;
		layout.origin = bevy::math::IVec2::new(-r, -r);
		layout.extents = bevy::math::UVec2::new(n, n);
		layout.outer_rings.clear();
		layout.stream_rings.clear();
		assets.lod_bands = playground_lod_bands(cells);
		assets.fine_grid_max_radius = Some(cells);
		dirty.0 = true;
		status.0 = format!("terrain-radius {cells}");
		commands.entity(entity).despawn();
	}
	for entity in &rebuild {
		dirty.0 = true;
		status.0 = "rebuild".into();
		commands.entity(entity).despawn();
	}
}

fn apply_mesh_stats(
	mut commands: Commands,
	mut status: ResMut<GameCommandStatusText>,
	mesh_assets: Res<Assets<Mesh>>,
	requests: Query<Entity, With<RequestMeshStats>>,
	mesh_entities: Query<&Mesh3d>,
) {
	for entity in &requests {
		let mut mesh_count = 0usize;
		let mut missing = 0usize;
		let mut vertices = 0usize;
		let mut indices = 0usize;
		let mut triangles = 0usize;
		let mut unique_handles = std::collections::HashSet::new();

		for mesh3d in &mesh_entities {
			mesh_count += 1;
			unique_handles.insert(mesh3d.0.id());
			let Some(mesh) = mesh_assets.get(&mesh3d.0) else {
				missing += 1;
				continue;
			};
			let verts = mesh.count_vertices();
			let index_count = mesh.indices().map(|i| i.len()).unwrap_or(verts);
			vertices += verts;
			indices += index_count;
			triangles += index_count / 3;
		}

		let text = format!(
			"stats mesh: entities={mesh_count} unique_handles={} missing={missing} verts={vertices} indices={indices} tris={triangles}",
			unique_handles.len()
		);
		info!("{text}");
		status.0 = text;
		commands.entity(entity).despawn();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::{IVec2, UVec2};
	use durham_terrain_models::{origin_cell_ids_for_layout, TerrainCellLayout, TERRAIN_CELL_SIZE};

	fn cell_layout(half_extent: i32) -> TerrainCellLayout {
		let r = half_extent.max(1);
		TerrainCellLayout {
			origin: IVec2::new(-r, -r),
			extents: UVec2::new((2 * r) as u32, (2 * r) as u32),
			outer_rings: Vec::new(),
			..TerrainCellLayout::default()
		}
	}

	#[test]
	fn default_patch_is_two_cell_radius() {
		let layout = cell_layout(DEFAULT_TERRAIN_RADIUS);
		let ids = origin_cell_ids_for_layout(&layout, layout.request_region());
		assert_eq!(ids.len(), 16);
	}

	#[test]
	fn default_cell_size_is_naturescapes() {
		assert!((TERRAIN_CELL_SIZE - 160.0).abs() < 1e-3);
	}
}
