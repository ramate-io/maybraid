//! Short authored herd/pack on a Durham fine-grid patch.
//!
//! Same 4×4 / ~640 m terrain as routing-playground, without groves or the
//! world mob stream. Hosts spawn on composed height after the trimesh exists.

#![allow(clippy::too_many_arguments, clippy::type_complexity)]

pub mod camera;
mod catalog;
pub mod commands;
mod mobs;
mod pitch;
mod playground_player;
mod ui;

pub use camera::CameraController;
pub use catalog::{scene_for, PlaygroundCast, HERD_MEMBERS, PACK_MEMBERS};
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use playground_player::PlaygroundMode;

use std::time::Duration;

use avian3d::prelude::{CoefficientCombine, Friction, LinearVelocity};
use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use camera::{
	camera_controller, refocus_camera_on_elevation, release_modifiers_on_focus_change,
	setup_camera, surface_or_hold,
};
use commands::{RequestModeCharacter, RequestModeFree};
use crozon_characters::CharacterMotionSystems;
use durham_terrain::shaders::{DurhamTerrainShader, DurhamTerrainShaderPlugin, RefractionWater};
use durham_terrain_models::{
	AvianTerrainIndex, BaseTerrainNoise, ComposedWater, DurhamTerrainModelsPlugin, Terrain,
	TerrainCellLayout, TerrainConfig, TerrainEntryStore, TerrainFrictionConfig, TerrainMeshBuilder,
	TerrainMeshLodBand, TerrainPresentationAssets, TerrainRegionPresenter, TerrainStoreView, Water,
	WaterPresentationAssets, WaterRegionPresenter, WaterStoreView,
};
use evasion_intelligence::{EvasionPlugin, EvasionSystems};
use firearm_intelligence::{FirearmIntelligencePlugin, FirearmIntelligenceSystems};
use firearm_user::FirearmUserPlugin;
use firearms::{FirearmWeaponSystems, FirearmWeaponsPlugin};
use fleeing_intelligence::{FleeingPlugin, FleeingSystems};
use game_commands::command::{
	capture_command_line_input, GameCommandPlugin, TextEntryBlocked, TextEntryFocus,
};
use game_commands::ui::GameCommandDrawerConfig;
use hiding_intelligence::{HidingPlugin, HidingSystems};
use lod::gen::{GeneratingSpatialIndex, RegionPresenter, SpatialIndex};
use lod::lod_ref::LodRef;
use maybraid_character_controller::CharacterControllerPlugin;
use maybraid_input::{PadGameplayEnabled, VirtualPadPlugin, VirtualPadSystems};
use maybraid_mobs::{MobSceneSystems, MobScenesPlugin};
use meandering_intelligence::MeanderingIntelligencePlugin;
use mob_intelligence::MobSystems;
use movement_intelligence::{
	CandidateBudget, MovementIntelligenceLimits, MovementIntelligencePlugin,
};
use movement_intelligence_avian::AvianMovementSurface;
use movement_realization::MovementRealizationPlugin;
use npc_intelligence::NpcIntelligenceSystems;
use player::{CharacterLocomotion as NpcLocomotion, PlayerPlugin as MaybraidPlayerPlugin};
use poi_intelligence::PoiSystems;
use render_item::mesh::handle::EnforceCachingPlugin;
use routing_intelligence::RoutingSystems;
use spotting_intelligence::SpottingSystems;
use std::f32::consts::PI;
use tether_intelligence::TetherSystems;
use threat_intelligence::ThreatIntelligencePlugin;
use threat_intelligence_damage::ThreatIntelligenceDamagePlugin;
use threat_management_intelligence::ThreatManagementPlugin;

use crate::mobs::PlaygroundState;
use crate::pitch::{apply_avian_terrain_pitch, sync_suspend_terrain_pitch};
use crate::playground_player::{
	respawn_player_on_layout, snap_player_to_composed_surface, AwaitingTerrainSurface,
	CharacterLocomotion, Player, PlayerControlSystems, PlayerPlugin,
};

const DEFAULT_TERRAIN_RADIUS: i32 = 2;
/// Same walkable cap as the world playground ([#718](https://github.com/ramate-io/maybraid/pull/718)).
const PLAYGROUND_MAX_SLOPE_ANGLE: f32 = 70.0_f32.to_radians();
/// Static grip sits above `tan(70°)` ≈ 2.75 so walkable slopes do not ice-skate.
const PLAYGROUND_TERRAIN_FRICTION: Friction = Friction {
	dynamic_coefficient: 2.55,
	static_coefficient: 2.95,
	combine_rule: CoefficientCombine::Max,
};

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

/// Base noise used for camera / capsule height before generation.
#[derive(Resource)]
pub struct WorldBaseTerrain(pub BaseTerrainNoise);

#[derive(Resource)]
pub(crate) struct TerrainPresentationDirty(bool);

#[derive(Resource, Default)]
struct TerrainPresentPending(bool);

pub struct MobOnTerrainPlaygroundPlugin;

impl Plugin for MobOnTerrainPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		let config = TerrainConfig::new(42);
		let base = BaseTerrainNoise::from_config(&config);
		let layout = cell_layout(DEFAULT_TERRAIN_RADIUS);

		app.insert_resource(TerrainFrictionConfig(PLAYGROUND_TERRAIN_FRICTION))
			.insert_resource(NpcLocomotion { max_slope_angle: PLAYGROUND_MAX_SLOPE_ANGLE })
			.insert_resource(CharacterLocomotion { max_slope_angle: PLAYGROUND_MAX_SLOPE_ANGLE })
			.add_plugins(DurhamTerrainModelsPlugin)
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
			.add_plugins(FirearmWeaponsPlugin)
			.add_plugins(FirearmUserPlugin)
			.add_plugins(FirearmIntelligencePlugin)
			.add_plugins(ThreatIntelligencePlugin)
			.add_plugins(ThreatIntelligenceDamagePlugin)
			.add_plugins(ThreatManagementPlugin)
			.add_plugins(EvasionPlugin)
			.add_plugins(FleeingPlugin)
			.add_plugins(HidingPlugin)
			.add_plugins(MeanderingIntelligencePlugin)
			.add_plugins(MovementRealizationPlugin)
			.add_plugins(MobScenesPlugin)
			.insert_resource(MovementIntelligenceLimits {
				max_budget: CandidateBudget { max_candidates: 8, max_steps: 3, horizon: 28.0 },
			})
			.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			.insert_resource(config.clone())
			.insert_resource(WorldBaseTerrain(base))
			.insert_resource(layout)
			.insert_resource(TerrainPresentationDirty(true))
			.insert_resource(PlaygroundState::default())
			.init_resource::<TerrainPresentPending>()
			.configure_sets(
				Update,
				(
					TetherSystems::Write.run_if(on_timer(Duration::from_millis(250))),
					RoutingSystems::Plan.run_if(on_timer(Duration::from_millis(250))),
				),
			)
			.configure_sets(
				Update,
				SpottingSystems::Observe.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				FirearmIntelligenceSystems::Spotting.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				FirearmIntelligenceSystems::Movement.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				(EvasionSystems::Ingest, EvasionSystems::Rank)
					.chain()
					.after(SpottingSystems::Observe)
					.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				(FleeingSystems::Write, HidingSystems::Write)
					.chain()
					.after(EvasionSystems::Rank)
					.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(Update, PoiSystems::Select.run_if(on_timer(Duration::from_millis(200))))
			.configure_sets(
				PostUpdate,
				FirearmIntelligenceSystems::ValidateAim.run_if(on_timer(Duration::from_millis(33))),
			)
			.configure_sets(
				PostUpdate,
				FirearmIntelligenceSystems::Fire.run_if(on_timer(Duration::from_millis(33))),
			)
			.configure_sets(
				PostUpdate,
				FirearmWeaponSystems::Fire.run_if(on_timer(Duration::from_millis(33))),
			)
			.add_systems(Startup, (setup_camera, setup_lighting, setup_presentation_assets))
			.add_systems(PreUpdate, sync_pad_gameplay.before(VirtualPadSystems::Produce))
			.add_systems(
				Update,
				(
					release_modifiers_on_focus_change.before(camera_controller),
					camera_controller,
					apply_mode_commands.after(capture_command_line_input::<PlaygroundCommand>),
					mobs::apply_cast_commands.after(apply_mode_commands),
					mobs::apply_rebuild_command.after(mobs::apply_cast_commands),
					generate_cells.after(mobs::apply_rebuild_command),
					present_cells.after(generate_cells),
					snap_player_to_composed_surface
						.after(present_cells)
						.after(apply_mode_commands)
						.before(PlayerControlSystems),
					mobs::spawn_forage_pois.after(present_cells),
					mobs::spawn_playground_mobs.after(mobs::spawn_forage_pois),
					mobs::tune_playground_journeying.after(MobSceneSystems::Install),
					mobs::widen_playground_member_leashes
						.after(MobSystems::Bind)
						.before(NpcIntelligenceSystems::Mix),
					sync_suspend_terrain_pitch,
					apply_avian_terrain_pitch
						.in_set(CharacterMotionSystems::Elevation)
						.after(sync_suspend_terrain_pitch),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
					mobs::draw_debug_gizmos,
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

fn generate_cells(
	mut commands: Commands,
	mut index: AvianTerrainIndex,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mut world_base: ResMut<WorldBaseTerrain>,
	mode: Res<PlaygroundMode>,
	mut cameras: Query<(&mut Transform, &mut CameraController), (With<Camera3d>, Without<Player>)>,
	mut players: Query<(Entity, &mut Transform, &mut LinearVelocity), With<Player>>,
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
	fn playground_slope_is_below_wall_grade() {
		let degrees = PLAYGROUND_MAX_SLOPE_ANGLE.to_degrees();
		assert!(degrees < 80.0);
		assert!(degrees > 55.0);
	}

	#[test]
	fn playground_terrain_static_friction_holds_max_walkable_slope() {
		assert!(PLAYGROUND_TERRAIN_FRICTION.static_coefficient > PLAYGROUND_MAX_SLOPE_ANGLE.tan());
	}
}
