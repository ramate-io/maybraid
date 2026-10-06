//! Courtyard wall, parking, and the player seat for one Training map.
//!
//! The urbanization scheme publishes [`TrainingPlazaStamped`]; the arena
//! resource seats the player once padded colliders exist.

use avian3d::prelude::{LinearVelocity, Position};
use bevy::prelude::*;
use building_components::{building_bounds, spawn_building_components};
use building_physics::{spawn_building_walk_colliders, BUILDING_FRICTION};
use buildings::wall_demo::TerrainPerimeterWall;
use durham::{Durham, WorldBaseTerrain};
use layer_stack::{ActiveGenerationMode, GenerationReadiness};
use lod::gen::Id;
use player_camera::FollowCamera;
use procedural_common::SeededHash;
use richmond::{DevelopmentFinish, Richmond};
use terrain_layer_model::{OnTerrain, TerrainView};
use urbanization_layer_model::Urbanization;
use world_player::player::{holding_elevation, player_spawn_point_at};
use world_player::{ModePlayerPolicies, OffTerrainAnchor, Player, PlayerSeat, PlayerSpawnXz};

use crate::markers::TrainingEnemyMarkers;
use crate::{TrainingArena, TrainingGround, TrainingMap, TrainingPlazaStamped, TrainingRound};

const TRAINING_WALL_STEP_M: f32 = 8.0;
const TRAINING_WALL_HEIGHT_M: f32 = 20.0;

/// Wall spawned for this stamp cell.
#[derive(Resource, Debug)]
pub(crate) struct TrainingPlazaWall {
	cell_id: Id,
}

/// The map whose site the player has already been parked on.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ParkedTrainingMap(TrainingMap);

/// Set on exit. [`finish_training_leave`] consumes it in [`Last`] so combat
/// commands queued through PostUpdate still find the wall and the anchor.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub(crate) struct TrainingLeave;

/// Stamped on Training fixtures so Leave can despawn them.
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct TrainingPlaza;

/// Raise the courtyard wall once the urbanization scheme has recorded a stamp.
pub(crate) fn mount_training_plaza(
	round: Res<TrainingRound>,
	stamped: Option<Res<TrainingPlazaStamped>>,
	wall: Option<Res<TrainingPlazaWall>>,
	ground: TerrainView<Urbanization<Richmond<OnTerrain<Durham>>>>,
	mut commands: Commands,
) {
	let Some(stamped) = stamped.as_deref() else {
		return;
	};
	if stamped.round().map() != round.map()
		|| wall.is_some_and(|wall| wall.cell_id == stamped.cell_id())
	{
		return;
	}
	info!(
		target: "world.training",
		"walled stamp at site {} (seed {:016x})",
		round.site(),
		round.seed,
	);
	spawn_training_wall(&mut commands, &ground, stamped, round.development_seed());
	commands.insert_resource(TrainingPlazaWall { cell_id: stamped.cell_id() });
}

/// Point the surface probe at a new round's site and move the player there
/// while the patch streams, so the camera, vegetation, and LOD follow it. A
/// body still being respawned is parked once it exists.
pub(crate) fn park_on_training_site(
	round: Res<TrainingRound>,
	base: Res<WorldBaseTerrain>,
	parked: Option<Res<ParkedTrainingMap>>,
	mut spawn_xz: ResMut<PlayerSpawnXz>,
	mut commands: Commands,
	mut players: Query<
		(&mut Transform, &mut GlobalTransform, Option<&mut Position>, Option<&mut LinearVelocity>),
		With<Player>,
	>,
	mut cameras: Query<
		(&mut Transform, &mut GlobalTransform, &FollowCamera),
		(With<Camera3d>, Without<Player>),
	>,
) {
	if parked.is_some_and(|parked| parked.0 == round.map()) {
		return;
	}
	let center = round.layout().region_center_xz().xz();
	spawn_xz.0 = Some(center);
	if players.is_empty() {
		return;
	}
	let at = player_spawn_point_at(center, holding_elevation(&base.0, center.x, center.y));
	PlayerSeat { at, facing: Vec3::Z }.apply(&mut players, &mut cameras);
	commands.insert_resource(ParkedTrainingMap(round.map()));
}

/// A body respawned onto the live plaza (a new life on the same map) takes
/// the arena seat and its anchor.
pub(crate) fn reseat_training_life(
	round: Res<TrainingRound>,
	ready: Option<Res<GenerationReadiness>>,
	seat: Option<Res<TrainingArena>>,
	mut commands: Commands,
	unanchored: Query<Entity, (With<Player>, Without<OffTerrainAnchor>)>,
	mut players: Query<
		(&mut Transform, &mut GlobalTransform, Option<&mut Position>, Option<&mut LinearVelocity>),
		With<Player>,
	>,
	mut cameras: Query<
		(&mut Transform, &mut GlobalTransform, &FollowCamera),
		(With<Camera3d>, Without<Player>),
	>,
) {
	if !ready.is_some_and(|ready| ready.covers(round.map().readiness_key())) {
		return;
	}
	let Some(seat) = seat else {
		return;
	};
	let Ok(player) = unanchored.single() else {
		return;
	};
	PlayerSeat { at: seat.player, facing: seat.facing }.apply(&mut players, &mut cameras);
	commands.entity(player).insert(OffTerrainAnchor { translation: seat.player });
}

/// Seat the player once the mode has published [`TrainingArena`].
pub(crate) fn promote_training_plaza(
	stamped: Option<Res<TrainingPlazaStamped>>,
	seat: Option<Res<TrainingArena>>,
	ready: Option<Res<GenerationReadiness>>,
	mut spawn_xz: ResMut<PlayerSpawnXz>,
	mut commands: Commands,
	player_ids: Query<Entity, With<Player>>,
	mut players: Query<
		(&mut Transform, &mut GlobalTransform, Option<&mut Position>, Option<&mut LinearVelocity>),
		With<Player>,
	>,
	mut cameras: Query<
		(&mut Transform, &mut GlobalTransform, &FollowCamera),
		(With<Camera3d>, Without<Player>),
	>,
) {
	if ready.is_some() {
		return;
	}
	let Some(stamped) = stamped.as_deref() else {
		return;
	};
	let Some(seat) = seat.as_deref() else {
		return;
	};
	spawn_xz.0 = Some(seat.player.xz());
	PlayerSeat { at: seat.player, facing: seat.facing }.apply(&mut players, &mut cameras);
	// Terrain snap and void recovery sample the raw FinePatch, which is below
	// the courtyard wherever the terrace fills. The anchor keeps the seat on
	// the padded collider.
	for player in &player_ids {
		commands.entity(player).insert(OffTerrainAnchor { translation: seat.player });
	}
	commands.insert_resource(GenerationReadiness::new(stamped.round().map().readiness_key()));
}

pub(crate) fn request_training_leave(mut commands: Commands) {
	commands.insert_resource(TrainingLeave);
}

/// Tear a plaza down when the round moves to a new map. The live session stays;
/// leaving is [`finish_training_leave`].
pub(crate) fn clear_stale_training_plaza(
	round: Res<TrainingRound>,
	ready: Option<Res<GenerationReadiness>>,
	mut commands: Commands,
	fixtures: Query<Entity, With<TrainingPlaza>>,
	anchored: Query<Entity, (With<Player>, With<OffTerrainAnchor>)>,
) {
	let Some(ready) = ready else {
		return;
	};
	if ready.covers(round.map().readiness_key()) {
		return;
	}
	tear_down_plaza(&mut commands, &fixtures, &anchored);
}

/// Leave parks the player on the destination mode's home, so that mode resumes
/// its own trail instead of the last Training site.
pub(crate) fn finish_training_leave(
	mode: Res<State<ActiveGenerationMode>>,
	policies: Res<ModePlayerPolicies>,
	base: Res<WorldBaseTerrain>,
	ready: Option<Res<GenerationReadiness>>,
	mut spawn_xz: ResMut<PlayerSpawnXz>,
	mut commands: Commands,
	fixtures: Query<Entity, With<TrainingPlaza>>,
	anchored: Query<Entity, (With<Player>, With<OffTerrainAnchor>)>,
	markers: Query<Entity, With<TrainingEnemyMarkers>>,
	mut players: Query<
		(&mut Transform, &mut GlobalTransform, Option<&mut Position>, Option<&mut LinearVelocity>),
		With<Player>,
	>,
	mut cameras: Query<
		(&mut Transform, &mut GlobalTransform, &FollowCamera),
		(With<Camera3d>, Without<Player>),
	>,
) {
	commands.remove_resource::<TrainingLeave>();
	commands.remove_resource::<ParkedTrainingMap>();
	for root in &markers {
		commands.entity(root).try_despawn();
	}
	if ready.is_none() {
		return;
	}
	tear_down_plaza(&mut commands, &fixtures, &anchored);
	spawn_xz.0 = None;
	let Some(home) = policies.home(mode.get().mode_id()) else {
		return;
	};
	let at = player_spawn_point_at(home, holding_elevation(&base.0, home.x, home.y));
	PlayerSeat { at, facing: Vec3::Z }.apply(&mut players, &mut cameras);
}

fn tear_down_plaza(
	commands: &mut Commands,
	fixtures: &Query<Entity, With<TrainingPlaza>>,
	anchored: &Query<Entity, (With<Player>, With<OffTerrainAnchor>)>,
) {
	// Fixtures nest under one another; every teardown command must tolerate a
	// target already gone.
	for entity in fixtures {
		commands.entity(entity).try_despawn();
	}
	for player in anchored {
		commands.entity(player).try_remove::<OffTerrainAnchor>();
	}
	commands.remove_resource::<TrainingPlazaWall>();
	commands.remove_resource::<GenerationReadiness>();
}

fn spawn_training_wall(
	commands: &mut Commands,
	ground: &TerrainView<Urbanization<Richmond<OnTerrain<Durham>>>>,
	stamped: &TrainingPlazaStamped,
	seed: u32,
) {
	let (min, max) = (stamped.center() - stamped.half(), stamped.center() + stamped.half());
	let samples = TerrainPerimeterWall::sample_rectangle(min, max, TRAINING_WALL_STEP_M);
	let terrain_y: Vec<f32> = samples
		.iter()
		.map(|sample| ground.height_or_fallback(Vec2::new(sample.x, sample.y)))
		.collect();
	let wall = TerrainPerimeterWall::from_samples(
		&samples,
		&terrain_y,
		stamped.plaza_y(),
		TRAINING_WALL_HEIGHT_M,
	)
	.with_material(DevelopmentFinish::rampart_stone(SeededHash::new(seed)));
	let bounds = building_bounds(&wall);
	for entity in spawn_building_components(commands, &wall, Transform::IDENTITY, bounds) {
		spawn_building_walk_colliders(commands, entity, &wall, BUILDING_FRICTION);
		commands
			.entity(entity)
			.insert((TrainingPlaza, DespawnOnExit(ActiveGenerationMode::of::<TrainingGround>())));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		pad_influence_region, training_development_cell, TRAINING_ARENA_MARGIN_M,
		TRAINING_ARENA_MAX_HALF_M,
	};
	use maybraid_game_mode_discover::Discovery;
	use crate::urbanization::{author_for_test, TrainingDevelopment, TRAINING_COURTYARD};
	use durham::HcsgStorage;
	use richmond::{AuthoredDevelopment, DevelopmentConfig, DevelopmentKind, DEVELOPMENT_CELL_SIZE};
	use std::any::TypeId;

	fn base_terrain() -> WorldBaseTerrain {
		use durham::{BaseTerrainNoise, TerrainConfig};
		WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(42)))
	}

	fn discovery_policies() -> ModePlayerPolicies {
		let mut policies = ModePlayerPolicies::default();
		policies.register(
			TypeId::of::<Discovery>(),
			world_player::ModePlayerPolicy {
				home: Vec2::ZERO,
				keep_waypoints: true,
				respawn_ends_life: false,
				pick_first_spawn: true,
			},
		);
		policies
	}

	fn readiness_for(round: TrainingRound) -> GenerationReadiness {
		GenerationReadiness::new(round.map().readiness_key())
	}

	#[test]
	fn training_cell_is_centered_on_the_site() -> anyhow::Result<()> {
		for center in [Vec2::ZERO, Vec2::new(-4_800.0, 1_120.0)] {
			let cell = training_development_cell(center);
			anyhow::ensure!(((cell.min.x + cell.max.x) * 0.5 - center.x).abs() < 1e-3);
			anyhow::ensure!(((cell.min.z + cell.max.z) * 0.5 - center.y).abs() < 1e-3);
			anyhow::ensure!((cell.max.x - cell.min.x - DEVELOPMENT_CELL_SIZE).abs() < 1e-3);
		}
		Ok(())
	}

	#[test]
	fn round_sites_pick_filled_kinds() -> anyhow::Result<()> {
		for seed in 0..8 {
			let round = TrainingRound::new(seed);
			let cell = training_development_cell(round.layout().region_center_xz().xz());
			let config = DevelopmentConfig::from_world_seed(round.development_seed());
			anyhow::ensure!(DevelopmentKind::pick_filled(cell, &config) != DevelopmentKind::Empty);
		}
		Ok(())
	}

	#[test]
	fn empty_cell_has_no_pad_influence() -> anyhow::Result<()> {
		let empty = TrainingDevelopment::Empty(training_development_cell(Vec2::ZERO));
		anyhow::ensure!(pad_influence_region(&empty).is_none());
		Ok(())
	}

	#[test]
	fn les_halles_courtyard_covers_the_wall() -> anyhow::Result<()> {
		let cell = training_development_cell(Vec2::ZERO);
		let mut storage = HcsgStorage::default();
		let id = author_for_test(
			&mut storage,
			AuthoredDevelopment {
				cell,
				kinds: vec![DevelopmentKind::LesHalles],
				height: 20.0,
				config: DevelopmentConfig::from_world_seed(42),
				courtyard: Some(TRAINING_COURTYARD),
			},
		)?;
		let walled = storage
			.get::<TrainingDevelopment>(id)
			.ok_or_else(|| anyhow::anyhow!("courtyard"))?;
		let footprint =
			walled.footprint_half_extents().ok_or_else(|| anyhow::anyhow!("footprint"))?;
		let half = (footprint + Vec2::splat(TRAINING_ARENA_MARGIN_M))
			.min(Vec2::splat(TRAINING_ARENA_MAX_HALF_M));
		let complex = walled.pad_complexes().next().ok_or_else(|| anyhow::anyhow!("pad"))?;
		let (min, max) = (-half, half);
		for sample in TerrainPerimeterWall::sample_rectangle(min, max, TRAINING_WALL_STEP_M) {
			let y = complex.modify_elevation(-15.0, sample.x, sample.y);
			anyhow::ensure!(
				(y - 20.0).abs() <= 1e-3,
				"wall station {sample} sits at {y}, expected 20"
			);
		}
		Ok(())
	}

	fn plaza_world(grounds: bool, round: TrainingRound) -> World {
		let mut world = World::new();
		world.insert_resource(State::new(if grounds {
			ActiveGenerationMode::of::<TrainingGround>()
		} else {
			ActiveGenerationMode::of::<Discovery>()
		}));
		world.insert_resource(round);
		world.insert_resource(base_terrain());
		world.insert_resource(PlayerSpawnXz(Some(Vec2::ONE)));
		world.insert_resource(discovery_policies());
		world
	}

	fn seated_player(world: &mut World, at: Vec3) -> Entity {
		let transform = Transform::from_translation(at);
		world
			.spawn((
				Player,
				transform,
				GlobalTransform::from(transform),
				OffTerrainAnchor { translation: at },
			))
			.id()
	}

	#[test]
	fn a_new_round_tears_the_plaza_down_in_place() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;
		let round = TrainingRound::new(8);
		let mut world = plaza_world(true, round.next());
		world.insert_resource(readiness_for(round));
		let seat = Vec3::new(900.0, 30.0, -400.0);
		let player = seated_player(&mut world, seat);
		let wall = world.spawn(TrainingPlaza).id();
		world
			.run_system_once(clear_stale_training_plaza)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(world.get_resource::<GenerationReadiness>().is_none());
		anyhow::ensure!(world.get_entity(wall).is_err());
		anyhow::ensure!(world.get::<OffTerrainAnchor>(player).is_none());
		anyhow::ensure!(world.get::<Transform>(player).map(|t| t.translation) == Some(seat));
		anyhow::ensure!(world.resource::<PlayerSpawnXz>().0 == Some(Vec2::ONE));

		world.insert_resource(readiness_for(round.next()));
		let wall = world.spawn(TrainingPlaza).id();
		world
			.run_system_once(clear_stale_training_plaza)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(world.get_entity(wall).is_ok(), "the live round keeps its plaza");

		world.insert_resource(round.next().next_life());
		world
			.run_system_once(clear_stale_training_plaza)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(world.get_entity(wall).is_ok(), "a new life keeps the map's plaza");
		Ok(())
	}

	#[test]
	fn a_new_life_takes_the_arena_seat() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;
		let round = TrainingRound::new(13);
		let mut world = plaza_world(true, round.next_life());
		let seat = Vec3::new(40.0, 8.0, -20.0);
		world.insert_resource(readiness_for(round));
		world.insert_resource(TrainingArena::at_seat(seat, Vec3::Z));
		let transform = Transform::from_translation(Vec3::new(0.0, 90.0, 0.0));
		let body = world.spawn((Player, transform, GlobalTransform::from(transform))).id();
		world
			.run_system_once(reseat_training_life)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(world.get::<Transform>(body).map(|t| t.translation) == Some(seat));
		anyhow::ensure!(world.get::<OffTerrainAnchor>(body).map(|a| a.translation) == Some(seat));

		let moved = Vec3::new(1.0, 2.0, 3.0);
		if let Some(mut at) = world.get_mut::<Transform>(body) {
			at.translation = moved;
		}
		world
			.run_system_once(reseat_training_life)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(world.get::<Transform>(body).map(|t| t.translation) == Some(moved));

		world.entity_mut(body).remove::<OffTerrainAnchor>();
		world.insert_resource(round.next());
		world
			.run_system_once(reseat_training_life)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(
			world.get::<Transform>(body).map(|t| t.translation) == Some(moved),
			"a new map seats through its own promote"
		);
		Ok(())
	}

	#[test]
	fn a_new_round_parks_the_player_on_its_site() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;
		let round = TrainingRound::new(21);
		let mut world = plaza_world(true, round);
		let player = seated_player(&mut world, Vec3::ZERO);
		world
			.run_system_once(park_on_training_site)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let center = round.layout().region_center_xz().xz();
		anyhow::ensure!(world.resource::<PlayerSpawnXz>().0 == Some(center));
		let at = world.get::<Transform>(player).map(|t| t.translation.xz());
		anyhow::ensure!(at.is_some_and(|at| at.distance(center) < 1e-3), "{at:?} vs {center}");
		Ok(())
	}

	#[test]
	fn leaving_training_drops_the_seat_anchor() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;
		let round = TrainingRound::default();
		let mut world = plaza_world(false, round);
		world.insert_resource(readiness_for(round));
		let player = seated_player(&mut world, Vec3::new(4_000.0, 12.0, -2_000.0));
		let wall = world.spawn(TrainingPlaza).id();
		world
			.run_system_once(finish_training_leave)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(world.get::<OffTerrainAnchor>(player).is_none());
		anyhow::ensure!(world.resource::<PlayerSpawnXz>().0.is_none());
		let home = world.get::<Transform>(player).map(|t| t.translation.xz());
		anyhow::ensure!(home == Some(Vec2::ZERO), "Discovery resumes from its default spawn");
		anyhow::ensure!(world.get_entity(wall).is_err());
		Ok(())
	}

	#[derive(Component)]
	struct DoomedThisFrame;

	fn despawn_doomed(doomed: Query<Entity, With<DoomedThisFrame>>, mut commands: Commands) {
		for entity in &doomed {
			commands.entity(entity).despawn();
		}
	}

	#[test]
	fn leaving_tolerates_targets_despawned_in_the_same_frame() -> anyhow::Result<()> {
		let round = TrainingRound::default();
		let mut world = plaza_world(false, round);
		world.insert_resource(readiness_for(round));
		let player = seated_player(&mut world, Vec3::new(4_000.0, 12.0, -2_000.0));
		world.entity_mut(player).insert(DoomedThisFrame);
		let wall = world.spawn((TrainingPlaza, DoomedThisFrame)).id();

		let mut schedule = Schedule::default();
		schedule.add_systems((despawn_doomed, finish_training_leave).chain_ignore_deferred());
		schedule.run(&mut world);

		for entity in [player, wall] {
			anyhow::ensure!(world.get_entity(entity).is_err());
		}
		anyhow::ensure!(world.get_resource::<GenerationReadiness>().is_none());
		Ok(())
	}
}
