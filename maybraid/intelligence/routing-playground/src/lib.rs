//! Hierarchical routing on a Durham fine-grid patch.
//!
//! Models-playground survey camera, vegetation lighting and command drawer — no groves.

pub mod camera;
pub mod commands;
mod playground_player;
mod ui;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use playground_player::PlaygroundMode;

use std::time::Duration;

use avian3d::prelude::{GravityScale, LinearVelocity};
use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use camera::{
	camera_controller, refocus_camera_on_elevation, release_modifiers_on_focus_change,
	setup_camera, surface_or_hold,
};
use commands::{
	RequestGo, RequestModeCharacter, RequestModeFree, RequestStalk, RequestTether,
	RequestTetherDrive, RequestTetherIdle,
};
use durham_terrain::shaders::{DurhamTerrainShader, DurhamTerrainShaderPlugin, RefractionWater};
use durham_terrain_models::{
	AvianTerrainIndex, BaseTerrainNoise, ComposedWater, DurhamTerrainModelsPlugin, Terrain,
	TerrainCellLayout, TerrainConfig, TerrainEntryStore, TerrainMeshBuilder, TerrainMeshLodBand,
	TerrainPresentationAssets, TerrainRegionPresenter, TerrainStoreView, TerrainTrimeshCollider,
	Water, WaterPresentationAssets, WaterRegionPresenter, WaterStoreView,
};
use game_commands::command::{
	capture_command_line_input, GameCommandPlugin, TextEntryBlocked, TextEntryFocus,
};
use game_commands::ui::GameCommandDrawerConfig;
use lod::gen::{GeneratingSpatialIndex, RegionPresenter, SpatialIndex};
use lod::lod_ref::LodRef;
use maybraid_character_controller::CharacterControllerPlugin;
use maybraid_input::{PadGameplayEnabled, VirtualPadPlugin, VirtualPadSystems};
use movement_intelligence::{
	CandidateBudget, MovementIntelligence, MovementIntelligenceLimits, MovementIntelligencePlugin,
	MovementLocation, MovementObjective,
};
use movement_intelligence_avian::AvianMovementSurface;
use movement_realization::MovementRealizationPlugin;
use player::{
	spawn_npc, Npc, PlayerLook, PlayerPlugin as MaybraidPlayerPlugin, CAPSULE_LENGTH,
	CAPSULE_RADIUS,
};
use playground_player::{
	holding_elevation, respawn_player_on_layout, snap_player_to_composed_surface, spawn_point_at,
	terrain_collider_ready, AwaitingTerrainSurface, Player, PlayerControlSystems, PlayerPlugin,
};
use render_item::mesh::handle::EnforceCachingPlugin;
use routing_intelligence::{
	RoutingIntelligenceUser, RoutingPlugin, RoutingSettings, RoutingSystems,
};
use std::f32::consts::PI;
use tether_intelligence::{
	install_tether, StalkRadii, Tether, TetherIntelligenceUser, TetherObjective, TetherPlugin,
	TetherSystems,
};

const DEFAULT_TERRAIN_RADIUS: i32 = 2;
const START_OFFSET_XZ: Vec2 = Vec2::new(-48.0, -24.0);
const GOAL_OFFSET_XZ: Vec2 = Vec2::new(220.0, 48.0);
const ROUTING_BANDS: [f32; 3] = [160.0, 80.0, 32.0];

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

fn offset_xz(layout: &TerrainCellLayout, offset: Vec2) -> Vec2 {
	let center = layout.region_center_xz();
	Vec2::new(center.x + offset.x, center.z + offset.y)
}

/// Base noise used for camera / capsule height before generation.
#[derive(Resource)]
pub struct WorldBaseTerrain(pub BaseTerrainNoise);

#[derive(Resource)]
struct TerrainPresentationDirty(bool);

#[derive(Resource, Default)]
struct TerrainPresentPending(bool);

/// World XZ the router walks toward. Y is snapped from composed height.
#[derive(Resource, Clone, Copy)]
struct RoutingGoal(Vec2);

/// Gravity off until composed height + a terrain trimesh exist.
#[derive(Component)]
struct AwaitingRouterSurface;

pub struct RoutingPlaygroundPlugin;

impl Plugin for RoutingPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		let config = TerrainConfig::new(42);
		let base = BaseTerrainNoise::from_config(&config);
		let layout = cell_layout(DEFAULT_TERRAIN_RADIUS);
		let goal = RoutingGoal(offset_xz(&layout, GOAL_OFFSET_XZ));

		app.add_plugins(DurhamTerrainModelsPlugin)
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
			);
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		app.add_plugins(PlayerPlugin)
			.add_plugins(CharacterControllerPlugin)
			.add_plugins(MaybraidPlayerPlugin)
			.add_plugins(MovementIntelligencePlugin::<AvianMovementSurface<'_, '_>>::default())
			.add_plugins(RoutingPlugin)
			.add_plugins(TetherPlugin)
			.add_plugins(MovementRealizationPlugin)
			.insert_resource(MovementIntelligenceLimits {
				max_budget: CandidateBudget { max_candidates: 12, max_steps: 3, horizon: 28.0 },
			})
			.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			.insert_resource(config.clone())
			.insert_resource(WorldBaseTerrain(base))
			.insert_resource(layout)
			.insert_resource(goal)
			.insert_resource(TerrainPresentationDirty(true))
			.init_resource::<TerrainPresentPending>()
			.configure_sets(
				Update,
				(
					TetherSystems::Write.run_if(on_timer(Duration::from_millis(250))),
					RoutingSystems::Plan.run_if(on_timer(Duration::from_millis(250))),
				),
			)
			.add_systems(Startup, (setup_camera, setup_lighting, setup_presentation_assets))
			.add_systems(PreUpdate, sync_pad_gameplay.before(VirtualPadSystems::Produce))
			.add_systems(
				Update,
				(
					release_modifiers_on_focus_change.before(camera_controller),
					camera_controller,
					apply_mode_commands.after(capture_command_line_input::<PlaygroundCommand>),
					apply_tether_commands.after(apply_mode_commands),
					apply_go_command.after(apply_tether_commands),
					spawn_router.after(apply_go_command),
					generate_cells.after(spawn_router),
					present_cells.after(generate_cells),
					snap_player_to_composed_surface
						.after(present_cells)
						.after(apply_mode_commands)
						.before(PlayerControlSystems),
					snap_router_to_composed_surface.after(snap_player_to_composed_surface),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
					draw_route_gizmos,
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
) {
	let material = terrain_materials.add(DurhamTerrainShader::default());
	commands.insert_resource(TerrainPresentationAssets {
		config: config.clone(),
		material,
		lod_bands: playground_lod_bands(DEFAULT_TERRAIN_RADIUS),
		outer_add_walls: true,
		fine_grid_max_radius: Some(DEFAULT_TERRAIN_RADIUS),
		macro_seam_half_extents: Vec::new(),
		macro_cell_min_size: None,
		macro_res_2: None,
	});
	commands.insert_resource(WaterPresentationAssets {
		material: water_materials.add(RefractionWater::default()),
	});
}

fn apply_mode_commands(
	mut commands: Commands,
	mut mode: ResMut<PlaygroundMode>,
	mut status: Option<ResMut<game_commands::ui::GameCommandStatusText>>,
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

fn apply_tether_commands(
	mut commands: Commands,
	mut status: Option<ResMut<game_commands::ui::GameCommandStatusText>>,
	players: Query<Entity, With<Player>>,
	tether: Query<(Entity, &RequestTether)>,
	stalk: Query<(Entity, &RequestStalk)>,
	idle: Query<Entity, With<RequestTetherIdle>>,
	drive: Query<Entity, With<RequestTetherDrive>>,
	mut users: Query<&mut TetherIntelligenceUser, With<Npc>>,
) {
	let Ok(player) = players.single() else {
		return;
	};
	for (entity, request) in &tether {
		for mut user in &mut users {
			user.objective = TetherObjective::Tether(player, request.radius.max(0.4));
			user.enabled = true;
		}
		ui::write_status(&mut status, format!("tether r={:.0}", request.radius));
		commands.entity(entity).despawn();
	}
	for (entity, request) in &stalk {
		let radii = StalkRadii::new(request.without, request.within);
		for mut user in &mut users {
			user.objective = TetherObjective::Stalk(player, radii);
			user.enabled = true;
		}
		ui::write_status(
			&mut status,
			format!("stalk without={:.0} within={:.0}", radii.without(), radii.within()),
		);
		commands.entity(entity).despawn();
	}
	for entity in &idle {
		for mut user in &mut users {
			user.enabled = false;
		}
		ui::write_status(&mut status, "tether idle");
		commands.entity(entity).despawn();
	}
	for entity in &drive {
		for mut user in &mut users {
			user.enabled = true;
		}
		ui::write_status(&mut status, "tether drive");
		commands.entity(entity).despawn();
	}
}

fn apply_go_command(
	mut commands: Commands,
	mut goal: ResMut<RoutingGoal>,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	mut status: Option<ResMut<game_commands::ui::GameCommandStatusText>>,
	requests: Query<(Entity, &RequestGo)>,
	mut routers: Query<
		(&mut RoutingIntelligenceUser, Option<&mut TetherIntelligenceUser>),
		With<Npc>,
	>,
) {
	for (entity, request) in &requests {
		goal.0 = Vec2::new(request.x, request.z);
		if let Some(elevation) = store.composed_height_at(&layout, request.x, request.z) {
			let destination = Vec3::new(request.x, elevation, request.z);
			for (mut routing, tether) in &mut routers {
				if let Some(mut tether) = tether {
					tether.enabled = false;
				}
				routing.set_destination(destination);
			}
			ui::write_status(&mut status, format!("go {0:.0} {1:.0}", request.x, request.z));
		} else {
			ui::write_status(
				&mut status,
				format!("go {0:.0} {1:.0}: waiting for terrain", request.x, request.z),
			);
		}
		commands.entity(entity).despawn();
	}
}

fn generate_cells(
	mut commands: Commands,
	mut index: AvianTerrainIndex,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mut world_base: ResMut<WorldBaseTerrain>,
	mode: Res<PlaygroundMode>,
	mut cameras: Query<(&mut Transform, &mut CameraController), (With<Camera3d>, Without<Player>)>,
	mut players: Query<(Entity, &mut Transform, &mut LinearVelocity), With<Player>>,
	routers: Query<Entity, With<Npc>>,
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
	let waters =
		GeneratingSpatialIndex::<Water>::get_or_generate_region(&mut index, region, &lod_ref);
	info!("generated terrain_cells={} water_cells={}", terrains.len(), waters.len());

	if let Some(base) = index.base_noise() {
		world_base.0 = base.clone();
	}

	if let Ok((player, mut transform, mut velocity)) = players.single_mut() {
		let center = layout.region_center_xz();
		if let Some(elevation) = index.composed_height_at(center.x, center.z) {
			respawn_player_on_layout(&layout, elevation, &mut transform, &mut velocity);
		}
		commands.entity(player).insert(AwaitingTerrainSurface);
	}

	for entity in &routers {
		commands.entity(entity).insert(AwaitingRouterSurface);
	}

	if *mode == PlaygroundMode::Free {
		if let Ok((mut transform, mut controller)) = cameras.single_mut() {
			let center = layout.region_center_xz();
			let elevation = index
				.composed_height_at(center.x, center.z)
				.unwrap_or_else(|| world_base.0.height_at(center.x, center.z));
			refocus_camera_on_elevation(&layout, elevation, &mut transform, &mut controller);
			info!(
				"survey camera=({:.0},{:.0},{:.0}) look_y={:.0}",
				transform.translation.x,
				transform.translation.y,
				transform.translation.z,
				elevation
			);
		} else {
			warn!("survey camera: expected one Camera3d, skip refocus");
		}
	}

	dirty.0 = false;
	pending.0 = true;
}

fn present_cells(
	mut terrain_presenter: TerrainRegionPresenter,
	mut water_presenter: WaterRegionPresenter,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	mut pending: ResMut<TerrainPresentPending>,
) {
	if !pending.0 {
		return;
	}

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
	let terrain_wanted = SpatialIndex::<Terrain>::tracked_ids_for(&terrain_view, region)
		.into_iter()
		.map(|tracked| tracked.0)
		.collect();
	terrain_presenter.remove_stale(&terrain_wanted);
	let water_view = WaterStoreView::new(&store, &layout);
	RegionPresenter::<Water, _>::present(&mut water_presenter, &water_view, region, &lod_ref);
	let water_wanted = SpatialIndex::<Water>::tracked_ids_for(&water_view, region)
		.into_iter()
		.map(|tracked| tracked.0)
		.collect();
	water_presenter.remove_stale(&water_wanted);
	info!(
		"presented terrain_scenes={} water_scenes={}",
		terrain_presenter.presented_ids().len(),
		water_presenter.presented_ids().len()
	);
	pending.0 = false;
}

fn spawn_router(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	players: Query<Entity, With<Player>>,
	routers: Query<(), With<Npc>>,
) {
	if !routers.is_empty() {
		return;
	}
	let Ok(player) = players.single() else {
		return;
	};
	commands.entity(player).insert(Tether);
	let start_xz = offset_xz(&layout, START_OFFSET_XZ);
	let elevation = holding_elevation(&base.0, start_xz.x, start_xz.y);
	let spawn = spawn_point_at(start_xz.x, start_xz.y, elevation);
	let npc =
		spawn_visible_npc(&mut commands, spawn, PlayerLook::default(), &mut meshes, &mut materials);
	let mut movement =
		MovementIntelligence::new(MovementObjective::Reach(MovementLocation::new(spawn, 0.5)));
	movement.ability.candidate_budget.horizon = 24.0;
	movement.ability.candidate_budget.max_candidates = 8;
	movement.ability.candidate_budget.max_steps = 3;
	let routing = RoutingIntelligenceUser::new(RoutingSettings::from_segments(ROUTING_BANDS));
	let tether = TetherIntelligenceUser::new(TetherObjective::Tether(player, 8.0))
		.with_horizon(28.0)
		.with_added_radius(2.0);
	commands
		.entity(npc)
		.insert((AwaitingRouterSurface, GravityScale(0.0), movement, routing));
	install_tether(&mut commands, npc, tether);
	info!("spawned router npc={npc} tether_anchor={player}");
}

fn snap_router_to_composed_surface(
	mut commands: Commands,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	awaiting: Query<Entity, (With<Npc>, With<AwaitingRouterSurface>)>,
	mut routers: Query<(&mut Transform, &mut LinearVelocity, &mut GravityScale), With<Npc>>,
	terrain_roots: Query<Entity, With<TerrainTrimeshCollider>>,
	children: Query<&Children>,
	colliders: Query<(), With<avian3d::prelude::Collider>>,
) {
	let Ok((mut transform, mut velocity, mut gravity)) = routers.single_mut() else {
		return;
	};

	let start_xz = offset_xz(&layout, START_OFFSET_XZ);
	let Some(elevation) = store.composed_height_at(&layout, start_xz.x, start_xz.y) else {
		gravity.0 = 0.0;
		**velocity = Vec3::ZERO;
		return;
	};

	let target = spawn_point_at(start_xz.x, start_xz.y, elevation);
	if awaiting.single().is_ok() {
		transform.translation = target;
		**velocity = Vec3::ZERO;
	}

	if terrain_collider_ready(&terrain_roots, &children, &colliders) {
		gravity.0 = 1.25;
		if let Ok(entity) = awaiting.single() {
			commands.entity(entity).remove::<AwaitingRouterSurface>();
		}
	} else {
		gravity.0 = 0.0;
		**velocity = Vec3::ZERO;
	}
}

fn draw_route_gizmos(
	mut gizmos: Gizmos,
	routers: Query<(&Transform, &RoutingIntelligenceUser), With<Npc>>,
) {
	let colors =
		[Color::srgb(1.0, 0.55, 0.15), Color::srgb(0.95, 0.85, 0.2), Color::srgb(0.25, 0.85, 1.0)];
	for (transform, routing) in &routers {
		gizmos.sphere(
			Isometry3d::from_translation(transform.translation),
			0.45,
			Color::srgb(1.0, 0.35, 0.75),
		);
		if let Some(goal) = routing.destination {
			gizmos.sphere(
				Isometry3d::from_translation(goal + Vec3::Y),
				0.8,
				Color::srgb(1.0, 1.0, 1.0),
			);
		}
		for (index, layer) in routing.plan.layers.iter().enumerate() {
			let color = colors[index.min(colors.len() - 1)];
			let lifted: Vec<Vec3> =
				layer.waypoints.iter().map(|point| *point + Vec3::Y * 1.5).collect();
			if lifted.len() >= 2 {
				gizmos.linestrip(lifted.iter().copied(), color);
			}
			for point in &lifted {
				gizmos.sphere(Isometry3d::from_translation(*point), 0.28, color);
			}
		}
	}
}

fn spawn_visible_npc(
	commands: &mut Commands,
	translation: Vec3,
	look: PlayerLook,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) -> Entity {
	let npc = spawn_npc(commands, translation, look);
	commands.spawn((
		Name::new("NpcCapsule"),
		ChildOf(npc),
		Mesh3d(meshes.add(Capsule3d::new(CAPSULE_RADIUS, CAPSULE_LENGTH))),
		MeshMaterial3d(materials.add(Color::srgb(1.0, 0.35, 0.75))),
	));
	npc
}

fn sync_pad_gameplay(
	focus: Option<Res<TextEntryFocus>>,
	blocked: Option<Res<TextEntryBlocked>>,
	mut enabled: ResMut<PadGameplayEnabled>,
) {
	let text = focus.is_some_and(|focus| focus.0) || blocked.is_some_and(|blocked| blocked.0);
	enabled.0 = !text;
}
