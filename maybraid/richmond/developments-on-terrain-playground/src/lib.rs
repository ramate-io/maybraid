//! Richmond's complete development catalog on Durham terrain.

pub mod camera;
pub mod commands;
mod hosts;
mod ui;

pub use camera::CameraController;
pub use commands::{DevelopmentFocus, PlaygroundCommand, PlaygroundStartup, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
use camera::{
	camera_controller, refocus_camera_on_layout, release_modifiers_on_focus_change, setup_camera,
};
use commands::{
	RequestDevelopmentFocus, RequestLikelihood, RequestMeshStats, RequestRebuild, RequestSeed,
	RequestTerrainRadius,
};
use durham_terrain::shaders::{DurhamTerrainShader, DurhamTerrainShaderPlugin, RefractionWater};
use durham_terrain_models::{
	AvianTerrainIndex, BaseTerrainNoise, ComposedWater, DurhamTerrainModelsPlugin,
	JerseyStampConfigs, MarazionWatershedConfigs, Terrain, TerrainCellLayout, TerrainConfig,
	TerrainEntryStore, TerrainMeshBuilder, TerrainMeshLodBand, TerrainPresentationAssets, Water,
	WaterPresentationAssets, WaterRegionPresenter, WaterStoreView,
};
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText};
use hosts::{spawn_development_hosts, DevelopmentHostRoot};
use lod::gen::{GeneratingSpatialIndex, RegionPresenter, SpatialIndex};
use lod::lod_ref::LodRef;
use render_item::mesh::handle::EnforceCachingPlugin;
use richmond_development_models::{
	BuiltDevelopment, BuiltDevelopmentStoreView, DevelopmentCell, DevelopmentConfig,
	DevelopmentEntryStore, DevelopmentIndex, PaddedStoreView, PaddedTerrainPresenter,
	RichmondDevelopmentModelsPlugin, TerrainWithPads,
};
use std::f32::consts::PI;

const DEFAULT_TERRAIN_RADIUS: i32 = 2;
/// Occupancy fill for the playground: high enough that Empty does not dominate.
const PLAYGROUND_LIKELIHOOD: f32 = 0.9;

fn playground_lod_bands(half_extent: i32) -> Vec<TerrainMeshLodBand> {
	vec![TerrainMeshLodBand { max_radius_cells: half_extent.max(1), res_2: 5 }]
}

fn cell_layout(half_extent: i32) -> TerrainCellLayout {
	let r = half_extent.max(1);
	let n = (2 * r) as u32;
	TerrainCellLayout {
		origin: IVec2::new(-r, -r),
		extents: UVec2::new(n, n),
		outer_rings: Vec::new(),
		..TerrainCellLayout::default()
	}
}

#[derive(Resource)]
pub struct WorldBaseTerrain(pub BaseTerrainNoise);

#[derive(Resource, Clone)]
pub struct PlaygroundConfig {
	pub terrain_radius: i32,
	pub focus_development: Option<DevelopmentFocus>,
}

impl Default for PlaygroundConfig {
	fn default() -> Self {
		Self { terrain_radius: DEFAULT_TERRAIN_RADIUS, focus_development: None }
	}
}

#[derive(Resource)]
struct TerrainPresentationDirty(bool);

#[derive(Resource, Default)]
struct DevelopmentsGeneratePending(bool);

#[derive(Resource, Default)]
struct TerrainPresentPending(bool);

#[derive(Resource, Default)]
struct HostsDirty(bool);

#[derive(Default)]
pub struct DevelopmentsOnTerrainPlugin {
	pub config: PlaygroundConfig,
}

impl Plugin for DevelopmentsOnTerrainPlugin {
	fn build(&self, app: &mut App) {
		let config = TerrainConfig::new(42);
		let base = BaseTerrainNoise::from_config(&config);
		let playground = self.config.clone();
		let layout = cell_layout(playground.terrain_radius);
		let mut development_config = DevelopmentConfig {
			likelihood: PLAYGROUND_LIKELIHOOD,
			..DevelopmentConfig::from_world_seed(42)
		};
		if let Some(focus) = playground.focus_development {
			focus.apply(&mut development_config);
		}

		app.add_plugins(DurhamTerrainModelsPlugin)
			.add_plugins(RichmondDevelopmentModelsPlugin)
			.add_plugins(DurhamTerrainShaderPlugin)
			.add_plugins(EnforceCachingPlugin::<TerrainMeshBuilder, DurhamTerrainShader>::default())
			.add_plugins(EnforceCachingPlugin::<ComposedWater, RefractionWater>::default())
			.add_plugins(
				GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config())
					.with_drawer_config(GameCommandDrawerConfig {
						open_at_start: false,
						toggle_keys: vec![KeyCode::F1, KeyCode::KeyY],
						..default()
					}),
			)
			.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			.insert_resource(config.clone())
			.insert_resource(WorldBaseTerrain(base))
			.insert_resource(playground.clone())
			.insert_resource(layout)
			.insert_resource(development_config)
			.insert_resource(TerrainPresentationDirty(true))
			.init_resource::<DevelopmentsGeneratePending>()
			.init_resource::<TerrainPresentPending>()
			.init_resource::<HostsDirty>()
			.add_systems(Startup, (setup_camera, setup_lighting, setup_presentation_assets))
			.add_systems(
				Update,
				(
					release_modifiers_on_focus_change.before(camera_controller),
					camera_controller,
					apply_commands.after(capture_command_line_input::<PlaygroundCommand>),
					generate_terrain.after(apply_commands),
					generate_developments.after(generate_terrain),
					present_cells.after(generate_developments),
					spawn_hosts.after(present_cells),
					apply_mesh_stats.after(present_cells),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
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

fn setup_presentation_assets(
	mut commands: Commands,
	mut terrain_materials: ResMut<Assets<DurhamTerrainShader>>,
	mut water_materials: ResMut<Assets<RefractionWater>>,
	config: Res<TerrainConfig>,
	playground: Res<PlaygroundConfig>,
) {
	let material = terrain_materials.add(DurhamTerrainShader::default());
	commands.insert_resource(TerrainPresentationAssets {
		config: config.clone(),
		material,
		lod_bands: playground_lod_bands(playground.terrain_radius),
		outer_add_walls: true,
		fine_grid_max_radius: Some(playground.terrain_radius),
		macro_seam_half_extents: Vec::new(),
		macro_cell_min_size: None,
		macro_res_2: None,
	});
	commands.insert_resource(WaterPresentationAssets {
		material: water_materials.add(RefractionWater::default()),
	});
}

#[allow(clippy::too_many_arguments)]
fn apply_commands(
	mut commands: Commands,
	mut playground: ResMut<PlaygroundConfig>,
	mut layout: ResMut<TerrainCellLayout>,
	mut assets: ResMut<TerrainPresentationAssets>,
	mut config: ResMut<TerrainConfig>,
	mut jersey: ResMut<JerseyStampConfigs>,
	mut marazion: ResMut<MarazionWatershedConfigs>,
	mut world_base: ResMut<WorldBaseTerrain>,
	mut development: ResMut<DevelopmentConfig>,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut status: ResMut<GameCommandStatusText>,
	seeds: Query<(Entity, &RequestSeed)>,
	likelihoods: Query<(Entity, &RequestLikelihood)>,
	focuses: Query<(Entity, &RequestDevelopmentFocus)>,
	radii: Query<(Entity, &RequestTerrainRadius)>,
	rebuild: Query<Entity, With<RequestRebuild>>,
) {
	for (entity, request) in &seeds {
		config.seed = request.0;
		assets.config.seed = request.0;
		*jersey = JerseyStampConfigs::from_world_seed(request.0);
		*marazion = MarazionWatershedConfigs::default().with_seed(request.0);
		world_base.0 = BaseTerrainNoise::from_config(&config);
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
		request.0.apply(&mut development);
		playground.focus_development = (request.0 != DevelopmentFocus::All).then_some(request.0);
		dirty.0 = true;
		status.0 = format!("focus-development {} (regen)", request.0);
		commands.entity(entity).despawn();
	}
	for (entity, request) in &radii {
		let cells = request.0.max(1);
		playground.terrain_radius = cells;
		*layout = cell_layout(cells);
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

fn generate_terrain(
	mut terrain_index: AvianTerrainIndex,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending_dev: ResMut<DevelopmentsGeneratePending>,
	mut world_base: ResMut<WorldBaseTerrain>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	if !dirty.0 {
		return;
	}

	terrain_index.clear();

	let layout = terrain_index.layout().clone();
	let region = layout.request_region();
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};

	let terrains = GeneratingSpatialIndex::<Terrain>::get_or_generate_region(
		&mut terrain_index,
		region,
		&lod_ref,
	);
	let waters = GeneratingSpatialIndex::<Water>::get_or_generate_region(
		&mut terrain_index,
		region,
		&lod_ref,
	);
	info!("generated terrain_cells={} water_cells={}", terrains.len(), waters.len());

	if let Some(base) = terrain_index.base_noise() {
		world_base.0 = base.clone();
	}

	if let Ok((mut transform, mut controller)) = cameras.single_mut() {
		refocus_camera_on_layout(&layout, &world_base.0, &mut transform, &mut controller);
	}

	dirty.0 = false;
	pending_dev.0 = true;
}

fn generate_developments(
	mut development_index: DevelopmentIndex,
	mut pending_dev: ResMut<DevelopmentsGeneratePending>,
	mut pending: ResMut<TerrainPresentPending>,
	mut hosts_dirty: ResMut<HostsDirty>,
) {
	if !pending_dev.0 {
		return;
	}

	development_index.clear();

	let layout = development_index.layout().clone();
	let region = layout.request_region();
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};

	let cells = GeneratingSpatialIndex::<DevelopmentCell>::get_or_generate_region(
		&mut development_index,
		region,
		&lod_ref,
	);
	let padded = GeneratingSpatialIndex::<TerrainWithPads>::get_or_generate_region(
		&mut development_index,
		region,
		&lod_ref,
	);
	let developments = GeneratingSpatialIndex::<BuiltDevelopment>::get_or_generate_region(
		&mut development_index,
		region,
		&lod_ref,
	);
	info!(
		"generated development_cells={} padded={} developments={}",
		cells.len(),
		padded.len(),
		developments.len(),
	);

	pending_dev.0 = false;
	pending.0 = true;
	hosts_dirty.0 = true;
}

fn present_cells(
	mut padded_presenter: PaddedTerrainPresenter,
	mut water_presenter: WaterRegionPresenter,
	terrain_store: Res<TerrainEntryStore>,
	dev_store: Res<DevelopmentEntryStore>,
	layout: Res<TerrainCellLayout>,
	mut pending: ResMut<TerrainPresentPending>,
) {
	if !pending.0 {
		return;
	}

	padded_presenter.clear_presented();
	water_presenter.clear_presented();

	let region = layout.request_region();
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};

	let padded_view = PaddedStoreView::new(&dev_store);
	RegionPresenter::<TerrainWithPads, _>::present(
		&mut padded_presenter,
		&padded_view,
		region,
		&lod_ref,
	);
	let padded_wanted = SpatialIndex::<TerrainWithPads>::tracked_ids_for(&padded_view, region)
		.into_iter()
		.map(|tracked| tracked.0)
		.collect();
	padded_presenter.remove_stale(&padded_wanted);

	let water_view = WaterStoreView::new(&terrain_store, &layout);
	RegionPresenter::<Water, _>::present(&mut water_presenter, &water_view, region, &lod_ref);
	let water_wanted = SpatialIndex::<Water>::tracked_ids_for(&water_view, region)
		.into_iter()
		.map(|tracked| tracked.0)
		.collect();
	water_presenter.remove_stale(&water_wanted);

	pending.0 = false;
}

fn spawn_hosts(
	mut commands: Commands,
	store: Res<DevelopmentEntryStore>,
	layout: Res<TerrainCellLayout>,
	mut dirty: ResMut<HostsDirty>,
	pending: Res<TerrainPresentPending>,
	terrain_dirty: Res<TerrainPresentationDirty>,
	roots: Query<Entity, With<DevelopmentHostRoot>>,
) {
	if !dirty.0 || pending.0 || terrain_dirty.0 {
		return;
	}

	for entity in &roots {
		commands.entity(entity).despawn();
	}

	let region = layout.request_region();
	let view = BuiltDevelopmentStoreView::new(&store);
	let mut n = 0usize;
	for tracked in SpatialIndex::<BuiltDevelopment>::tracked_ids_for(&view, region) {
		let Some(dev) = SpatialIndex::<BuiltDevelopment>::get(&view, tracked.0) else {
			continue;
		};
		n += spawn_development_hosts(&mut commands, dev);
	}
	info!("spawned {n} development host roots");
	dirty.0 = false;
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
	use durham_terrain_models::{origin_cell_ids_for_layout, TERRAIN_CELL_SIZE};

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
