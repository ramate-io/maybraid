//! Small Durham fine-grid patch for iterating Chico groves on real ground.

pub mod camera;
pub mod commands;
pub mod diagnostics;
mod groves;
mod ui;

pub use camera::CameraController;
pub use commands::{GroveKind, PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use diagnostics::{PlaygroundDiag, PlaygroundTimingPlugin};
pub use game_commands::command::PendingStartupCommand;

use bevy::camera::visibility::VisibilitySystems;
use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
use camera::{camera_controller, refocus_camera_on_layout, setup_camera};
use chico_groves::DEFAULT_GROVE_EXTENT_XZ;
use chico_sbs_trees_playground::register_vegetation_view;
use chico_vegetation_components::{FoliageLodProbe, StickLodProbe};
use commands::{
	RequestGrove, RequestGroveExtent, RequestMeshStats, RequestRebuild, RequestTerrainRadius,
	RequestTileRadius,
};
use durham_terrain::shaders::{DurhamTerrainShader, DurhamTerrainShaderPlugin};
use durham_terrain_models::{
	AvianTerrainIndex, BaseTerrainNoise, ComposedTerrain, DurhamTerrainModelsPlugin, Terrain,
	TerrainCellLayout, TerrainConfig, TerrainEntryStore, TerrainMeshLodBand,
	TerrainPresentationAssets, TerrainRegionPresenter, TerrainStoreView, WaterPresentationAssets,
};
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText};
use groves::{spawn_tiled_groves, GroveRoot};
use lod::gen::{GeneratingSpatialIndex, RegionPresenter};
use lod::lod_ref::LodRef;
use lod::LodSceneHost;
use render_item::mesh::handle::EnforceCachingPlugin;
use render_item::sdf::cpu_shot::CpuShotBuilder;
use std::f32::consts::PI;

const DEFAULT_TERRAIN_RADIUS: i32 = 2;
const DEFAULT_TILE_RADIUS: i32 = 1;

fn playground_lod_bands(half_extent: i32) -> Vec<TerrainMeshLodBand> {
	vec![TerrainMeshLodBand { max_radius_cells: half_extent.max(1), res_2: 5 }]
}

fn cell_layout(half_extent: i32) -> TerrainCellLayout {
	let r = half_extent.max(1);
	let mut layout = TerrainCellLayout::default();
	layout.origin = IVec2::new(-r, -r);
	let n = (2 * r) as u32;
	layout.extents = UVec2::new(n, n);
	layout.outer_rings.clear();
	layout
}

/// Base noise used for camera height before (and alongside) generation.
#[derive(Resource)]
pub struct WorldBaseTerrain(pub BaseTerrainNoise);

#[derive(Resource, Clone)]
pub struct PlaygroundConfig {
	pub grove: commands::GroveKind,
	pub terrain_radius: i32,
	pub grove_extent_xz: f32,
	pub tile_radius: i32,
}

impl Default for PlaygroundConfig {
	fn default() -> Self {
		Self {
			grove: commands::GroveKind::MonsterGrass,
			terrain_radius: DEFAULT_TERRAIN_RADIUS,
			grove_extent_xz: DEFAULT_GROVE_EXTENT_XZ,
			tile_radius: DEFAULT_TILE_RADIUS,
		}
	}
}

#[derive(Resource)]
struct TerrainPresentationDirty(bool);

#[derive(Resource, Default)]
struct TerrainPresentPending(bool);

#[derive(Resource)]
struct GrovesDirty(bool);

pub struct VegetationOnTerrainPlugin;

impl Plugin for VegetationOnTerrainPlugin {
	fn build(&self, app: &mut App) {
		let config = TerrainConfig::new(42);
		let base = BaseTerrainNoise::from_config(&config);
		let playground = PlaygroundConfig::default();

		app.add_plugins(DurhamTerrainModelsPlugin)
			.add_plugins(DurhamTerrainShaderPlugin)
			.add_plugins(EnforceCachingPlugin::<
				CpuShotBuilder<ComposedTerrain>,
				DurhamTerrainShader,
			>::default())
			.add_plugins(
				GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config())
					.with_drawer_config(GameCommandDrawerConfig {
						open_at_start: false,
						toggle_keys: vec![KeyCode::F1, KeyCode::KeyY],
						..default()
					}),
			);
		register_vegetation_view(app);
		app.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			.insert_resource(config.clone())
			.insert_resource(WorldBaseTerrain(base))
			.insert_resource(playground.clone())
			.insert_resource(cell_layout(playground.terrain_radius))
			.insert_resource(TerrainPresentationDirty(true))
			.init_resource::<TerrainPresentPending>()
			.insert_resource(GrovesDirty(true))
			.add_systems(Startup, (setup_camera, setup_lighting, setup_presentation_assets))
			.add_systems(
				Update,
				(
					camera_controller,
					apply_commands.after(capture_command_line_input::<PlaygroundCommand>),
					generate_cells.after(apply_commands),
					present_cells.after(generate_cells),
					spawn_groves.after(present_cells),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			)
			.add_systems(PostUpdate, apply_mesh_stats.after(VisibilitySystems::CheckVisibility));
	}
}

/// Count total vs view-visible mesh triangles (`ViewVisibility`) and LOD probe hosts.
fn apply_mesh_stats(
	mut commands: Commands,
	mut status: ResMut<GameCommandStatusText>,
	mesh_assets: Res<Assets<Mesh>>,
	requests: Query<Entity, With<RequestMeshStats>>,
	mesh_entities: Query<(&Mesh3d, &ViewVisibility)>,
	foliage_probes: Query<(), With<FoliageLodProbe>>,
	stick_probes: Query<(), With<StickLodProbe>>,
	lod_hosts: Query<(), With<LodSceneHost>>,
) {
	for entity in &requests {
		let mut total_entities = 0usize;
		let mut visible_entities = 0usize;
		let mut missing = 0usize;
		let mut total_tris = 0usize;
		let mut visible_tris = 0usize;
		let mut unique_handles = std::collections::HashSet::new();
		let mut visible_unique_handles = std::collections::HashSet::new();

		for (mesh3d, view_visibility) in &mesh_entities {
			total_entities += 1;
			unique_handles.insert(mesh3d.0.id());
			let Some(mesh) = mesh_assets.get(&mesh3d.0) else {
				missing += 1;
				continue;
			};
			let verts = mesh.count_vertices();
			let index_count = mesh.indices().map(|i| i.len()).unwrap_or(verts);
			let tris = index_count / 3;
			total_tris += tris;
			if view_visibility.get() {
				visible_entities += 1;
				visible_unique_handles.insert(mesh3d.0.id());
				visible_tris += tris;
			}
		}

		let foliage_probes = foliage_probes.iter().count();
		let stick_probes = stick_probes.iter().count();
		let lod_hosts = lod_hosts.iter().count();
		let probes_total = foliage_probes + stick_probes;

		status.0 = format!(
			"stats mesh:\n  total_tris={total_tris}\n  visible_tris={visible_tris}\n  entities={total_entities} visible_entities={visible_entities} unique_handles={} visible_unique={} missing={missing}\n  probes: foliage={foliage_probes} stick={stick_probes} total={probes_total}\n  lod_hosts={lod_hosts}",
			unique_handles.len(),
			visible_unique_handles.len(),
		);
		info!("{}", status.0);
		commands.entity(entity).despawn();
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
	mut standard_materials: ResMut<Assets<StandardMaterial>>,
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
	// AvianTerrainIndex requires water presentation assets even when we skip water.
	let water_material = standard_materials.add(StandardMaterial {
		base_color: Color::srgba(0.15, 0.45, 0.75, 0.72),
		alpha_mode: AlphaMode::Blend,
		..default()
	});
	commands.insert_resource(WaterPresentationAssets { material: water_material });
}

fn apply_commands(
	mut commands: Commands,
	mut playground: ResMut<PlaygroundConfig>,
	mut layout: ResMut<TerrainCellLayout>,
	mut terrain_assets: ResMut<TerrainPresentationAssets>,
	mut terrain_dirty: ResMut<TerrainPresentationDirty>,
	mut groves_dirty: ResMut<GrovesDirty>,
	mut status: ResMut<GameCommandStatusText>,
	grove: Query<(Entity, &RequestGrove)>,
	terrain_radius: Query<(Entity, &RequestTerrainRadius)>,
	grove_extent: Query<(Entity, &RequestGroveExtent)>,
	tile_radius: Query<(Entity, &RequestTileRadius)>,
	rebuild: Query<Entity, With<RequestRebuild>>,
) {
	for (entity, request) in &grove {
		playground.grove = request.0;
		groves_dirty.0 = true;
		status.0 = format!("grove {}", request.0.label());
		commands.entity(entity).despawn();
	}
	for (entity, request) in &terrain_radius {
		let cells = request.0.max(1);
		playground.terrain_radius = cells;
		*layout = cell_layout(cells);
		terrain_assets.lod_bands = playground_lod_bands(cells);
		terrain_assets.fine_grid_max_radius = Some(cells);
		terrain_dirty.0 = true;
		groves_dirty.0 = true;
		status.0 = format!("terrain-radius {cells}");
		commands.entity(entity).despawn();
	}
	for (entity, request) in &grove_extent {
		playground.grove_extent_xz = request.0.max(1.0);
		groves_dirty.0 = true;
		status.0 = format!("grove-extent {}", playground.grove_extent_xz);
		commands.entity(entity).despawn();
	}
	for (entity, request) in &tile_radius {
		playground.tile_radius = request.0.max(0);
		groves_dirty.0 = true;
		status.0 = format!("tile-radius {}", playground.tile_radius);
		commands.entity(entity).despawn();
	}
	for entity in &rebuild {
		groves_dirty.0 = true;
		status.0 = "rebuild".into();
		commands.entity(entity).despawn();
	}
}

fn generate_cells(
	mut index: AvianTerrainIndex,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mut world_base: ResMut<WorldBaseTerrain>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	if !dirty.0 {
		return;
	}

	index.clear();

	let layout = index.layout().clone();
	let region = layout.request_region();
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};

	let terrains =
		GeneratingSpatialIndex::<Terrain>::get_or_generate_region(&mut index, region, &lod_ref);
	info!("generated terrain_cells={}", terrains.len());

	if let Some(base) = index.base_noise() {
		world_base.0 = base.clone();
	}

	if let Ok((mut transform, mut controller)) = cameras.single_mut() {
		refocus_camera_on_layout(&layout, &world_base.0, &mut transform, &mut controller);
	}

	dirty.0 = false;
	pending.0 = true;
}

fn present_cells(
	mut terrain_presenter: TerrainRegionPresenter,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	mut pending: ResMut<TerrainPresentPending>,
) {
	if !pending.0 {
		return;
	}

	terrain_presenter.clear_presented();

	let region = layout.request_region();
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};
	let terrain_view = TerrainStoreView::new(&store, &layout);
	RegionPresenter::<Terrain, _>::present(&mut terrain_presenter, &terrain_view, region, &lod_ref);
	pending.0 = false;
}

fn spawn_groves(
	mut commands: Commands,
	config: Res<PlaygroundConfig>,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	mut dirty: ResMut<GrovesDirty>,
	pending: Res<TerrainPresentPending>,
	terrain_dirty: Res<TerrainPresentationDirty>,
	roots: Query<Entity, With<GroveRoot>>,
) {
	if !dirty.0 || pending.0 || terrain_dirty.0 {
		return;
	}

	for entity in &roots {
		commands.entity(entity).despawn();
	}

	let n = spawn_tiled_groves(&mut commands, &config, &store, &layout, &base.0);
	info!(
		"spawned {} grove hosts ({} {}m tiles, r={})",
		n,
		config.grove.label(),
		config.grove_extent_xz,
		config.tile_radius
	);
	dirty.0 = false;
}
