//! Durham-backed vegetation host used by `maybraid-world`.
//!
//! The runnable playground binary is retired; see `maybraid/PLAYGROUNDS.md`.
//! Character / camera stay on [`VegetationHostPlugin`]. Groves and canopy
//! bump-outs register through [`VegetationPlugin`]. Terrain LOD is
//! [`TerrainPlugin`](durham_terrain_models::TerrainPlugin) for
//! [`Durham`](durham_terrain_models::Durham).

mod bump_out;
pub mod camera;
pub mod character;
pub mod commands;
pub mod diagnostics;
mod forest;
mod groves;
mod material_lib;
mod pitch;
pub mod player;
mod ui;
mod vegetation;

pub use bump_out::{
	bump_out_from_cell, bump_out_noise, fine_terrain_for, medium_terrain_for,
	register_bump_out_lod, terrain_chunk_ref, BumpOutPresenter, CanopyBumpOutPresenter,
	CanopyBumpOutPresenterState, MediumCanopyBumpOutPresenter, MediumCanopyBumpOutPresenterState,
	TerrainMeshSource, WorldTerrainBuilder,
};
pub use camera::CameraController;
pub use character::{CharacterSpecies, PlayerVisual, RequestSetCharacter};
pub use chico_forests::{ForestStreamSpec, OnTerrain};
pub use commands::{GroveKind, PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use diagnostics::{PlaygroundDiag, PlaygroundTimingPlugin, RequestFpsToggle};
pub use durham_terrain_models::{
	terrain_streaming_enabled, TerrainCoverage, TerrainStreamingEnabled, WorldBaseTerrain,
};
pub use forest::DurhamHeight;
pub use game_commands::command::PendingStartupCommand;
pub use groves::{OwnedDurhamTerrain, StoredDurhamTerrain};
pub use material_lib::{
	init_vegetation_on_terrain_material_caches, VegetationOnTerrainMaterialLib,
	VegetationOnTerrainMaterialRefPlugin,
};
pub use player::{
	CharacterCameraFollowEnabled, CharacterLocomotion, MoveWish, MovementAction,
	PadMovementEnabled, Player, PlayerCapsule, PlayerControlSystems, PlayerPhysicsEnabled,
	PlayerPlugin, PlayerRespawn, PlaygroundMode, SpawnTerrainReady,
};
pub use vegetation::VegetationPlugin;

use avian3d::prelude::LinearVelocity;
use bevy::camera::visibility::VisibilitySystems;
use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
use camera::{
	camera_controller, refocus_camera_on_elevation, release_modifiers_on_focus_change,
	setup_camera, surface_or_hold,
};
use character::{apply_set_character, drive_player_locomotion};
use chico_forests::{stream_radii_m, VegetationViewPlugin};
use chico_groves::DEFAULT_GROVE_EXTENT_XZ;
use chico_vegetation_components::{FoliageLodProbe, StickLodProbe};
use commands::{
	RequestForest, RequestGrove, RequestGroveExtent, RequestMeshStats, RequestModeCharacter,
	RequestModeFree, RequestRebuild, RequestTerrainRadius, RequestTileRadius,
};
use crozon_characters::{CharacterHostsPlugin, CharacterMotionSystems};
use durham_terrain_models::{
	origin_cell_ids_for_layout, Durham, TerrainCellLayout, TerrainEntryStore, TerrainMeshLodBand,
	TerrainPlugin, TerrainPresentationAssets, TerrainPresentationDirty, TERRAIN_CELL_SIZE,
	WORLD_FINE_HALF_EXTENT_CELLS,
};
use game_commands::command::{
	capture_command_line_input, GameCommandPlugin, TextEntryBlocked, TextEntryFocus,
};
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText};
use groves::{spawn_tiled_groves, GroveRoot};
use lod::{LodPresentSystems, LodSceneHost};
use maybraid_input::{PadGameplayEnabled, VirtualPadPlugin, VirtualPadSystems};
use pitch::{apply_avian_terrain_pitch, sync_suspend_terrain_pitch};
use player::{respawn_player_on_layout, snap_player_to_composed_surface, AwaitingTerrainSurface};
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

fn terrain_cells_for_generate_m(generate_m: f32) -> i32 {
	(generate_m / TERRAIN_CELL_SIZE).ceil() as i32
}

#[derive(Resource, Clone)]
pub struct PlaygroundConfig {
	pub grove: commands::GroveKind,
	pub terrain_radius: i32,
	pub grove_extent_xz: f32,
	pub tile_radius: i32,
	/// `Some` streams the forest and skips tiled groves.
	pub forest: Option<ForestStreamSpec>,
	pub coverage: TerrainCoverage,
}

impl Default for PlaygroundConfig {
	fn default() -> Self {
		Self {
			grove: commands::GroveKind::MonsterGrass,
			terrain_radius: DEFAULT_TERRAIN_RADIUS,
			grove_extent_xz: DEFAULT_GROVE_EXTENT_XZ,
			tile_radius: DEFAULT_TILE_RADIUS,
			forest: None,
			coverage: TerrainCoverage::FinePatch,
		}
	}
}

impl PlaygroundConfig {
	/// Terrain + forest at playable present / generate extents.
	pub fn world_defaults() -> Self {
		Self {
			grove: commands::GroveKind::MonsterGrass,
			terrain_radius: WORLD_FINE_HALF_EXTENT_CELLS,
			grove_extent_xz: DEFAULT_GROVE_EXTENT_XZ,
			tile_radius: DEFAULT_TILE_RADIUS,
			forest: Some(ForestStreamSpec { stream_radius: 1, ..ForestStreamSpec::default() }),
			coverage: TerrainCoverage::PlayableWorld,
		}
	}
}

#[derive(Resource)]
struct GrovesDirty(bool);

/// Character, camera, snap, and locomotion without tiled groves or stream drivers.
///
/// The assembled world plugin uses this. Groves and bump-outs stay on
/// [`VegetationPlugin`].
pub struct VegetationHostPlugin;

impl Plugin for VegetationHostPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		if !app.is_plugin_added::<PlayerPlugin>() {
			app.add_plugins(PlayerPlugin);
		}
		if !app.is_plugin_added::<CharacterHostsPlugin>() {
			app.add_plugins(CharacterHostsPlugin);
		}
		app.add_systems(Startup, setup_camera)
			.add_systems(PreUpdate, sync_pad_gameplay.before(VirtualPadSystems::Produce))
			.add_systems(
				Update,
				(
					release_modifiers_on_focus_change.before(camera_controller),
					camera_controller,
					apply_set_character,
					apply_mode_commands.after(apply_set_character),
					snap_player_to_composed_surface
						.after(LodPresentSystems::Drain)
						.after(apply_mode_commands)
						.before(PlayerControlSystems),
					drive_player_locomotion
						.after(PlayerControlSystems)
						.before(CharacterMotionSystems::Anim),
					sync_suspend_terrain_pitch.after(PlayerControlSystems),
					apply_avian_terrain_pitch
						.in_set(CharacterMotionSystems::Elevation)
						.after(drive_player_locomotion)
						.after(sync_suspend_terrain_pitch),
				),
			);
	}
}

/// Tiled-grove playground host. World uses [`VegetationHostPlugin`] instead.
pub struct VegetationOnTerrainPlugin {
	pub config: PlaygroundConfig,
	/// When false, the caller owns the command drawer / CLI.
	pub commands: bool,
	/// When false, the caller owns [`TerrainPlugin`] for [`Durham`].
	pub own_terrain: bool,
}

impl Default for VegetationOnTerrainPlugin {
	fn default() -> Self {
		Self { config: PlaygroundConfig::default(), commands: true, own_terrain: true }
	}
}

impl Plugin for VegetationOnTerrainPlugin {
	fn build(&self, app: &mut App) {
		let playground = self.config.clone();

		if self.own_terrain {
			app.add_plugins(match playground.coverage {
				TerrainCoverage::FinePatch => {
					TerrainPlugin::<Durham>::fine_patch(playground.terrain_radius)
				}
				TerrainCoverage::PlayableWorld => TerrainPlugin::<Durham>::playable_world(),
			});
		}
		if self.commands {
			app.add_plugins(
				GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config())
					.with_drawer_config(GameCommandDrawerConfig {
						open_at_start: false,
						toggle_keys: vec![KeyCode::F1, KeyCode::KeyY],
						..default()
					}),
			);
		}
		if !app.is_plugin_added::<VegetationViewPlugin>() {
			app.add_plugins(VegetationViewPlugin);
		}
		if !app.is_plugin_added::<VegetationOnTerrainMaterialRefPlugin>() {
			app.add_plugins(VegetationOnTerrainMaterialRefPlugin);
		}
		if !app.is_plugin_added::<VegetationHostPlugin>() {
			app.add_plugins(VegetationHostPlugin);
		}
		app.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			.insert_resource(playground.clone())
			.insert_resource(GrovesDirty(true))
			.add_systems(Startup, setup_lighting)
			.add_systems(PostUpdate, apply_mesh_stats.after(VisibilitySystems::CheckVisibility));
		if self.commands {
			app.add_systems(
				Update,
				(
					apply_commands.after(capture_command_line_input::<PlaygroundCommand>),
					spawn_groves.after(LodPresentSystems::Drain).run_if(terrain_streaming_enabled),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
		} else {
			app.add_systems(
				Update,
				spawn_groves.after(LodPresentSystems::Drain).run_if(terrain_streaming_enabled),
			);
		}
	}
}

/// Count total vs view-visible mesh triangles (`ViewVisibility`) and LOD probe hosts.
fn apply_mesh_stats(
	mut commands: Commands,
	mut status: Option<ResMut<GameCommandStatusText>>,
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

		let text = format!(
			"stats mesh:\n  total_tris={total_tris}\n  visible_tris={visible_tris}\n  entities={total_entities} visible_entities={visible_entities} unique_handles={} visible_unique={} missing={missing}\n  probes: foliage={foliage_probes} stick={stick_probes} total={probes_total}\n  lod_hosts={lod_hosts}",
			unique_handles.len(),
			visible_unique_handles.len(),
		);
		info!("{text}");
		ui::write_status(&mut status, text);
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

fn apply_commands(
	mut commands: Commands,
	mut playground: ResMut<PlaygroundConfig>,
	mut layout: ResMut<TerrainCellLayout>,
	mut terrain_assets: ResMut<TerrainPresentationAssets>,
	mut terrain_dirty: ResMut<TerrainPresentationDirty>,
	mut groves_dirty: ResMut<GrovesDirty>,
	mut status: Option<ResMut<GameCommandStatusText>>,
	grove: Query<(Entity, &RequestGrove)>,
	forest: Query<(Entity, &RequestForest)>,
	terrain_radius: Query<(Entity, &RequestTerrainRadius)>,
	grove_extent: Query<(Entity, &RequestGroveExtent)>,
	tile_radius: Query<(Entity, &RequestTileRadius)>,
	rebuild: Query<Entity, With<RequestRebuild>>,
) {
	for (entity, request) in &grove {
		playground.grove = request.0;
		playground.forest = None;
		groves_dirty.0 = true;
		ui::write_status(&mut status, format!("grove {}", request.0.label()));
		commands.entity(entity).despawn();
	}
	for (entity, request) in &forest {
		let spec = request.0;
		playground.forest = Some(spec);
		if playground.coverage == TerrainCoverage::FinePatch {
			let (_, generate_m) = stream_radii_m(spec.stream_radius);
			let needed = terrain_cells_for_generate_m(generate_m).max(1);
			if playground.terrain_radius < needed {
				playground.terrain_radius = needed;
				*layout = cell_layout(needed);
				terrain_assets.lod_bands = playground_lod_bands(needed);
				terrain_assets.fine_grid_max_radius = Some(needed);
				terrain_dirty.0 = true;
			}
		}
		groves_dirty.0 = true;
		let layering = spec.layering.map(|k| k.as_kebab()).unwrap_or("hopscotch");
		ui::write_status(&mut status, format!("forest {layering} r={}", spec.stream_radius));
		commands.entity(entity).despawn();
	}
	for (entity, request) in &terrain_radius {
		let cells = request.0.max(1);
		playground.terrain_radius = cells;
		if playground.coverage == TerrainCoverage::FinePatch {
			*layout = cell_layout(cells);
			terrain_assets.lod_bands = playground_lod_bands(cells);
			terrain_assets.fine_grid_max_radius = Some(cells);
			terrain_dirty.0 = true;
		}
		groves_dirty.0 = true;
		ui::write_status(&mut status, format!("terrain-radius {cells}"));
		commands.entity(entity).despawn();
	}
	for (entity, request) in &grove_extent {
		playground.grove_extent_xz = request.0.max(1.0);
		groves_dirty.0 = true;
		ui::write_status(&mut status, format!("grove-extent {}", playground.grove_extent_xz));
		commands.entity(entity).despawn();
	}
	for (entity, request) in &tile_radius {
		playground.tile_radius = request.0.max(0);
		groves_dirty.0 = true;
		ui::write_status(&mut status, format!("tile-radius {}", playground.tile_radius));
		commands.entity(entity).despawn();
	}
	for entity in &rebuild {
		groves_dirty.0 = true;
		ui::write_status(&mut status, "rebuild");
		commands.entity(entity).despawn();
	}
}

fn apply_mode_commands(
	mut commands: Commands,
	mut mode: ResMut<PlaygroundMode>,
	mut status: Option<ResMut<GameCommandStatusText>>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	store: Res<TerrainEntryStore>,
	free: Query<Entity, With<RequestModeFree>>,
	character: Query<Entity, With<RequestModeCharacter>>,
	mut players: Query<(Entity, &mut Transform, &mut LinearVelocity), With<Player>>,
	mut cameras: Query<(&mut Transform, &mut CameraController), (With<Camera3d>, Without<Player>)>,
) {
	for entity in &free {
		*mode = PlaygroundMode::Free;
		ui::write_status(&mut status, "mode free");
		if let Ok((mut cam_t, mut controller)) = cameras.single_mut() {
			refocus_camera_on_elevation(
				&layout,
				surface_or_hold(&layout, &store, &base.0),
				&mut cam_t,
				&mut controller,
			);
		}
		commands.entity(entity).despawn();
	}

	for entity in &character {
		*mode = PlaygroundMode::Character;
		ui::write_status(&mut status, "mode character — WASD move, mouse look, Space jump");
		if let Ok((player, mut transform, mut velocity)) = players.single_mut() {
			let center = layout.region_center_xz();
			if let Some(elevation) = store.composed_height_at(&layout, center.x, center.z) {
				respawn_player_on_layout(&layout, elevation, &mut transform, &mut velocity);
			}
			commands.entity(player).insert(AwaitingTerrainSurface);
		}
		commands.entity(entity).despawn();
	}
}

fn spawn_groves(
	mut commands: Commands,
	config: Res<PlaygroundConfig>,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	mut dirty: ResMut<GrovesDirty>,
	roots: Query<Entity, With<GroveRoot>>,
) {
	if config.forest.is_some() {
		dirty.0 = false;
		return;
	}
	if !dirty.0 {
		return;
	}
	let terrain_ready = origin_cell_ids_for_layout(&layout, layout.request_region())
		.into_iter()
		.all(|original| store.terrain(original.0).is_some());
	if !terrain_ready {
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

fn sync_pad_gameplay(
	focus: Option<Res<TextEntryFocus>>,
	blocked: Option<Res<TextEntryBlocked>>,
	mut enabled: ResMut<PadGameplayEnabled>,
) {
	let text = focus.is_some_and(|focus| focus.0) || blocked.is_some_and(|blocked| blocked.0);
	enabled.0 = !text;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn world_defaults_keep_grove_fill_at_one_kilometre() {
		let spec = PlaygroundConfig::world_defaults().forest.expect("forest on");
		assert_eq!(spec.stream_radius, 1);
		assert_eq!(stream_radii_m(1), (1_000.0, 3_000.0));
	}

	#[test]
	fn world_stream_boundaries_align_all_three_grids() {
		use durham_terrain_models::{
			WORLD_TERRAIN_BACKGROUND_RADIUS_M, WORLD_TERRAIN_FAR_RADIUS_M,
			WORLD_TERRAIN_NEAR_RADIUS_M, WORLD_TERRAIN_PRESENT_STEP_M,
		};
		assert_eq!(WORLD_TERRAIN_NEAR_RADIUS_M % (2.0 * TERRAIN_CELL_SIZE), 0.0);
		assert_eq!(WORLD_TERRAIN_FAR_RADIUS_M % (4.0 * TERRAIN_CELL_SIZE), 0.0);
		assert_eq!(WORLD_TERRAIN_BACKGROUND_RADIUS_M, 3_840.0);
		assert_eq!(chico_forests::MEDIUM_BUMP_OUT_INNER_RADIUS_M, WORLD_TERRAIN_NEAR_RADIUS_M);
		assert_eq!(chico_forests::MEDIUM_BUMP_OUT_OUTER_RADIUS_M, WORLD_TERRAIN_FAR_RADIUS_M);
		assert_eq!(chico_forests::MEDIUM_BUMP_OUT_ANCHOR_STEP_M, WORLD_TERRAIN_PRESENT_STEP_M);
	}
}
