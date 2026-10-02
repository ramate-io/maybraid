//! One seeded Richmond development on the Training FinePatch, walled into a
//! flat courtyard arena. The training crate fields the roster as a mob cell;
//! this module raises the wall and seats the player once padded colliders exist.

use avian3d::prelude::{LinearVelocity, Position};
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::player::{holding_elevation, player_spawn_point_at};
use chico_vegetation_on_terrain_playground::{OffTerrainAnchor, Player};
use durham_terrain_models::{Durham, TerrainTrimeshCollider, WorldBaseTerrain};
use lod::gen::Id;
use player_camera::FollowCamera;
use procedural_common::SeededHash;
use richmond_building_components::{building_bounds, spawn_building_components};
use richmond_building_physics::{BUILDING_FRICTION, spawn_building_walk_colliders};
use richmond_buildings::wall_demo::TerrainPerimeterWall;
use richmond_development_models::{
	DevelopmentFinish, PresentedPaddedTerrainScene,
};
use terrain_layer_model::{OnTerrain, TerrainView};
use urbanization_layer_model::Urbanization;

use maybraid_game_mode_training_ground::{
	TrainingGround, TrainingMap, TrainingPlazaStamped, TrainingRosterSeat, TrainingRound,
};
use terrain_layer_model::ActiveGenerationMode;

use crate::PlayerSpawnXz;

const TRAINING_WALL_STEP_M: f32 = 8.0;
const TRAINING_WALL_HEIGHT_M: f32 = 20.0;

/// The development, wall, and player seat have been stamped for this round's map.
/// Also set, with nothing stamped, once every site the round tried fit no
/// development.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingPlazaMounted(pub TrainingRound);

impl TrainingPlazaMounted {
	pub fn serves(&self, round: TrainingRound) -> bool {
		self.0.map() == round.map()
	}
}

/// Wall spawned for this stamp cell.
#[derive(Resource, Debug)]
pub(crate) struct TrainingPlazaWall {
	cell_id: Id,
}

/// Stamped on Training fixtures so Leave can despawn them.
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct TrainingPlaza;

/// Raise the courtyard wall once the urbanization scheme has recorded a stamp.
pub(crate) fn mount_training_plaza(
	mode: Res<State<ActiveGenerationMode>>,
	round: Res<TrainingRound>,
	stamped: Option<Res<TrainingPlazaStamped>>,
	wall: Option<Res<TrainingPlazaWall>>,
	ground: TerrainView<Urbanization<OnTerrain<Durham>>>,
	mut commands: Commands,
) {
	if !mode.get().is::<TrainingGround>() {
		return;
	}
	let Some(stamped) = stamped.as_deref() else {
		return;
	};
	if stamped.round().map() != round.map() || wall.is_some_and(|wall| wall.cell_id == stamped.cell_id()) {
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
	mode: Res<State<ActiveGenerationMode>>,
	round: Res<TrainingRound>,
	base: Res<WorldBaseTerrain>,
	mut parked: Local<Option<TrainingMap>>,
	mut spawn_xz: ResMut<PlayerSpawnXz>,
	mut players: Query<
		(&mut Transform, &mut GlobalTransform, Option<&mut Position>, Option<&mut LinearVelocity>),
		With<Player>,
	>,
	mut cameras: Query<
		(&mut Transform, &mut GlobalTransform, &FollowCamera),
		(With<Camera3d>, Without<Player>),
	>,
) {
	if !mode.get().is::<TrainingGround>() {
		*parked = None;
		return;
	}
	if *parked == Some(round.map()) {
		return;
	}
	let center = round.layout().region_center_xz().xz();
	spawn_xz.0 = Some(center);
	if players.is_empty() {
		return;
	}
	let at = player_spawn_point_at(center, holding_elevation(&base.0, center.x, center.y));
	seat_player_at(&mut players, &mut cameras, at, Vec3::Z);
	*parked = Some(round.map());
}

/// A body respawned onto the live plaza (a new life on the same map) takes
/// the arena seat and its anchor.
pub(crate) fn reseat_training_life(
	mode: Res<State<ActiveGenerationMode>>,
	round: Res<TrainingRound>,
	mounted: Option<Res<TrainingPlazaMounted>>,
	seat: Option<Res<TrainingRosterSeat>>,
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
	if !mode.get().is::<TrainingGround>() || !mounted.is_some_and(|mounted| mounted.serves(*round)) {
		return;
	}
	let Some(seat) = seat else {
		return;
	};
	let Ok(player) = unanchored.single() else {
		return;
	};
	seat_player_at(&mut players, &mut cameras, seat.player, seat.facing);
	commands.entity(player).insert(OffTerrainAnchor { translation: seat.player });
}

/// Seat the player once padded colliders exist over the stamp and the roster
/// has published its seat.
pub(crate) fn promote_training_plaza(
	mode: Res<State<ActiveGenerationMode>>,
	stamped: Option<Res<TrainingPlazaStamped>>,
	seat: Option<Res<TrainingRosterSeat>>,
	mounted: Option<Res<TrainingPlazaMounted>>,
	ready_pads: Query<&PresentedPaddedTerrainScene, With<TerrainTrimeshCollider>>,
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
	if !mode.get().is::<TrainingGround>() || mounted.is_some() {
		return;
	}
	let Some(stamped) = stamped.as_deref() else {
		return;
	};
	let Some(seat) = seat.as_deref() else {
		return;
	};
	let cooked = ready_pads.iter().filter(|scene| stamped.terrain_ids().contains(&scene.0)).count();
	if !stamped.fills_ready(cooked) {
		return;
	}
	spawn_xz.0 = Some(seat.player.xz());
	seat_player_at(&mut players, &mut cameras, seat.player, seat.facing);
	// Terrain snap and void recovery sample the raw FinePatch, which is below
	// the courtyard wherever the terrace fills. The anchor keeps the seat on
	// the padded collider.
	for player in &player_ids {
		commands.entity(player).insert(OffTerrainAnchor { translation: seat.player });
	}
	commands.insert_resource(TrainingPlazaMounted(stamped.round()));
}

/// Tear the plaza down when Training ends or the round moves to a new map. Leaving
/// parks the player on Discovery's default spawn, so Discovery resumes the
/// character's saved trail instead of starting at the last Training site.
pub(crate) fn clear_training_plaza(
	mode: Res<State<ActiveGenerationMode>>,
	round: Res<TrainingRound>,
	base: Res<WorldBaseTerrain>,
	mounted: Option<Res<TrainingPlazaMounted>>,
	mut spawn_xz: ResMut<PlayerSpawnXz>,
	mut commands: Commands,
	fixtures: Query<Entity, With<TrainingPlaza>>,
	anchored: Query<Entity, (With<Player>, With<OffTerrainAnchor>)>,
	mut players: Query<
		(&mut Transform, &mut GlobalTransform, Option<&mut Position>, Option<&mut LinearVelocity>),
		With<Player>,
	>,
	mut cameras: Query<
		(&mut Transform, &mut GlobalTransform, &FollowCamera),
		(With<Camera3d>, Without<Player>),
	>,
) {
	let live = mode.get().is::<TrainingGround>();
	let stale = |of: TrainingRound| !live || of.map() != round.map();
	let mounted_stale = mounted.as_deref().is_some_and(|mounted| stale(mounted.0));
	if !mounted_stale {
		return;
	}
	// Fixtures nest under one another; every teardown command must tolerate a
	// target already gone.
	for entity in &fixtures {
		commands.entity(entity).try_despawn();
	}
	for player in &anchored {
		commands.entity(player).try_remove::<OffTerrainAnchor>();
	}
	commands.remove_resource::<TrainingPlazaWall>();
	commands.remove_resource::<TrainingPlazaMounted>();
	if live {
		return;
	}
	spawn_xz.0 = None;
	let home = Vec2::ZERO;
	let at = player_spawn_point_at(home, holding_elevation(&base.0, home.x, home.y));
	seat_player_at(&mut players, &mut cameras, at, Vec3::Z);
}

fn spawn_training_wall(
	commands: &mut Commands,
	ground: &TerrainView<Urbanization<OnTerrain<Durham>>>,
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
		commands.entity(entity).insert((TrainingPlaza, DespawnOnExit(ActiveGenerationMode::of::<TrainingGround>())));
	}
}

fn seat_player_at(
	players: &mut Query<
		(&mut Transform, &mut GlobalTransform, Option<&mut Position>, Option<&mut LinearVelocity>),
		With<Player>,
	>,
	cameras: &mut Query<
		(&mut Transform, &mut GlobalTransform, &FollowCamera),
		(With<Camera3d>, Without<Player>),
	>,
	at: Vec3,
	facing: Vec3,
) {
	let Ok((mut player_tf, mut player_global, mut body, mut velocity)) = players.single_mut()
	else {
		return;
	};
	player_tf.translation = at;
	*player_global = GlobalTransform::from(*player_tf);
	if let Some(position) = body.as_deref_mut() {
		position.0 = at;
	}
	if let Some(velocity) = velocity.as_deref_mut() {
		velocity.0 = Vec3::ZERO;
	}
	drop((player_tf, player_global, body, velocity));

	let Ok((mut camera_tf, mut camera_global, follow)) = cameras.single_mut() else {
		return;
	};
	let look = at + Vec3::Y * follow.look_height;
	let eye = look - facing * follow.distance + Vec3::Y * follow.height;
	let parked = Transform::from_translation(eye).looking_at(look, Vec3::Y);
	*camera_tf = parked;
	*camera_global = GlobalTransform::from(parked);
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_game_mode_training_ground::{
		pad_influence_region, training_development_cell, TRAINING_ARENA_MARGIN_M,
		TRAINING_ARENA_MAX_HALF_M, TRAINING_COURTYARD_EASE_M, TRAINING_COURTYARD_OVERHANG_M,
	};
	use richmond_development_models::{
		DEVELOPMENT_CELL_SIZE, DevelopmentCell, DevelopmentConfig, DevelopmentKind, PadParams,
	};

	fn base_terrain() -> WorldBaseTerrain {
		use durham_terrain_models::{BaseTerrainNoise, TerrainConfig};
		WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(42)))
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
		let empty = DevelopmentCell::empty(training_development_cell(Vec2::ZERO));
		anyhow::ensure!(pad_influence_region(&empty).is_none());
		Ok(())
	}

	#[test]
	fn les_halles_courtyard_covers_the_wall() -> anyhow::Result<()> {
		let cell = training_development_cell(Vec2::ZERO);
		let config = DevelopmentConfig::from_world_seed(42);
		let filled = DevelopmentCell::with_les_halles(cell, 20.0, &config);
		let footprint = filled
			.footprint_half_extents()
			.ok_or_else(|| anyhow::anyhow!("footprint"))?;
		let half = (footprint + Vec2::splat(TRAINING_ARENA_MARGIN_M))
			.min(Vec2::splat(TRAINING_ARENA_MAX_HALF_M));
		let courtyard_half = half + Vec2::splat(TRAINING_COURTYARD_OVERHANG_M);
		let params = PadParams { berm: 0.0, ease: TRAINING_COURTYARD_EASE_M, round: 0.0 };
		let walled = filled
			.with_courtyard(courtyard_half, params)
			.ok_or_else(|| anyhow::anyhow!("courtyard"))?;
		let complex = walled
			.pad_complexes()
			.next()
			.ok_or_else(|| anyhow::anyhow!("pad"))?;
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
			ActiveGenerationMode::of::<maybraid_game_mode_discover::Discovery>()
		}));
		world.insert_resource(round);
		world.insert_resource(base_terrain());
		world.insert_resource(PlayerSpawnXz(Some(Vec2::ONE)));
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
		world.insert_resource(TrainingPlazaMounted(round));
		let seat = Vec3::new(900.0, 30.0, -400.0);
		let player = seated_player(&mut world, seat);
		let wall = world.spawn(TrainingPlaza).id();
		world
			.run_system_once(clear_training_plaza)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(world.get_resource::<TrainingPlazaMounted>().is_none());
		anyhow::ensure!(world.get_entity(wall).is_err());
		anyhow::ensure!(world.get::<OffTerrainAnchor>(player).is_none());
		anyhow::ensure!(world.get::<Transform>(player).map(|t| t.translation) == Some(seat));
		anyhow::ensure!(world.resource::<PlayerSpawnXz>().0 == Some(Vec2::ONE));

		world.insert_resource(TrainingPlazaMounted(round.next()));
		let wall = world.spawn(TrainingPlaza).id();
		world
			.run_system_once(clear_training_plaza)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(world.get_entity(wall).is_ok(), "the live round keeps its plaza");

		world.insert_resource(round.next().next_life());
		world
			.run_system_once(clear_training_plaza)
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
		world.insert_resource(TrainingPlazaMounted(round));
		world.insert_resource(TrainingRosterSeat { player: seat, facing: Vec3::Z });
		let transform = Transform::from_translation(Vec3::new(0.0, 90.0, 0.0));
		let body = world.spawn((Player, transform, GlobalTransform::from(transform))).id();
		world
			.run_system_once(reseat_training_life)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(world.get::<Transform>(body).map(|t| t.translation) == Some(seat));
		anyhow::ensure!(
			world.get::<OffTerrainAnchor>(body).map(|a| a.translation) == Some(seat)
		);

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
		world.insert_resource(TrainingPlazaMounted(round));
		let player = seated_player(&mut world, Vec3::new(4_000.0, 12.0, -2_000.0));
		let wall = world.spawn(TrainingPlaza).id();
		world
			.run_system_once(clear_training_plaza)
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
		world.insert_resource(TrainingPlazaMounted(round));
		let player = seated_player(&mut world, Vec3::new(4_000.0, 12.0, -2_000.0));
		world.entity_mut(player).insert(DoomedThisFrame);
		let wall = world.spawn((TrainingPlaza, DoomedThisFrame)).id();

		let mut schedule = Schedule::default();
		schedule.add_systems((despawn_doomed, clear_training_plaza).chain_ignore_deferred());
		schedule.run(&mut world);

		for entity in [player, wall] {
			anyhow::ensure!(world.get_entity(entity).is_err());
		}
		anyhow::ensure!(world.get_resource::<TrainingPlazaMounted>().is_none());
		Ok(())
	}
}
