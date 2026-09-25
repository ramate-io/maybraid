//! One seeded Richmond development on the Training FinePatch, walled at the pad.

use avian3d::prelude::{LinearVelocity, Position};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::Player;
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore, WorldBaseTerrain};
use mob_characters::{
	CharacterBrains, CharacterBuild, CharacterInventory, CharacterSpecies, MobCharacter,
};
use player::capsule_spawn_height;
use player_camera::FollowCamera;
use richmond_building_components::{building_bounds, spawn_building_components};
use richmond_building_physics::{BUILDING_FRICTION, spawn_building_walk_colliders};
use richmond_buildings::wall_demo::TerrainPerimeterWall;
use richmond_development_models::{
	DEVELOPMENT_CELL_SIZE, DevelopmentCell, DevelopmentConfig, DevelopmentHosts, DevelopmentKind,
};

use crate::PlayerSpawnXz;
use crate::control::WorldSurfaceReady;
use crate::training::TrainingGrounds;

const TRAINING_WALL_STEP_M: f32 = 8.0;
const TRAINING_WALL_CLEARANCE_M: f32 = 4.0;
const TRAINING_WALL_MARGIN_M: f32 = 6.0;
const TRAINING_COURTYARD_OFFSET_M: f32 = 2.4;

/// The development, brawler, and wall have been stamped for this Training session.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrainingPlazaMounted;

/// Stamped on Training fixtures so Leave can despawn them.
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct TrainingPlaza;

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct TrainingBrawler;

/// 300 m cell centered on the FinePatch origin.
fn training_development_cell() -> Aabb3d {
	let half = DEVELOPMENT_CELL_SIZE * 0.5;
	Aabb3d::from_min_max(Vec3::new(-half, 0.0, -half), Vec3::new(half, 1.0, half))
}

/// Fit a seeded Richmond development once the FinePatch collider exists.
pub(crate) fn mount_training_plaza(
	grounds: Res<TrainingGrounds>,
	ready: Res<WorldSurfaceReady>,
	mounted: Option<Res<TrainingPlazaMounted>>,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
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
	if !grounds.0 || !ready.0 || mounted.is_some() {
		return;
	}
	let cell = training_development_cell();
	let center = Vec2::new((cell.min.x + cell.max.x) * 0.5, (cell.min.z + cell.max.z) * 0.5);
	let plaza_y = store
		.composed_height_at(&layout, center.x, center.y)
		.unwrap_or_else(|| base.0.height_at(center.x, center.y));
	let config = DevelopmentConfig::from_world_seed(42);
	let Some((kind, filled, built)) =
		stamp_training_development(&store, &layout, cell, &config, plaza_y)
	else {
		warn!(target: "world.training", "no Richmond development fitted on the Training patch");
		commands.insert_resource(TrainingPlazaMounted);
		return;
	};
	info!(target: "world.training", "stamped {kind:?} on the Training patch");
	for host in built.hosts() {
		for entity in host.spawn(&mut commands) {
			commands.entity(entity).insert(TrainingPlaza);
		}
	}
	let wall = development_wall_rect(&filled, center);
	spawn_training_wall(&mut commands, &store, &layout, &base, wall, plaza_y);
	let courtyard = courtyard_spawns(&filled, plaza_y);
	spawn_xz.0 = Some(courtyard.player.xz());
	spawn_training_brawler(&mut commands, courtyard.npc, courtyard.look_yaw);
	seat_player_at(&mut players, &mut cameras, courtyard.player);
	commands.insert_resource(TrainingPlazaMounted);
}

pub(crate) fn clear_training_plaza(
	grounds: Res<TrainingGrounds>,
	mounted: Option<Res<TrainingPlazaMounted>>,
	mut spawn_xz: ResMut<PlayerSpawnXz>,
	mut commands: Commands,
	fixtures: Query<Entity, Or<(With<TrainingPlaza>, With<TrainingBrawler>)>>,
) {
	if grounds.0 || mounted.is_none() {
		return;
	}
	for entity in &fixtures {
		commands.entity(entity).despawn();
	}
	spawn_xz.0 = None;
	commands.remove_resource::<TrainingPlazaMounted>();
}

fn stamp_training_development(
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	cell: Aabb3d,
	config: &DevelopmentConfig,
	height: f32,
) -> Option<(DevelopmentKind, DevelopmentCell, richmond_development_models::BuiltDevelopment)> {
	let preferred = DevelopmentKind::pick_filled(cell, config);
	let mut order = vec![preferred];
	order.extend(DevelopmentKind::FILLED.into_iter().filter(|kind| *kind != preferred));
	for kind in order {
		let Some(filled) = DevelopmentCell::fill(store, layout, cell, kind, config, height) else {
			continue;
		};
		let Some(built) = filled.built(config.seed as i32) else {
			continue;
		};
		return Some((kind, filled, built));
	}
	None
}

fn development_wall_rect(filled: &DevelopmentCell, fallback_center: Vec2) -> (Vec2, Vec2) {
	let mut min = Vec2::splat(f32::INFINITY);
	let mut max = Vec2::splat(f32::NEG_INFINITY);
	let mut any = false;
	for pad in filled.pad_complexes() {
		min = min.min(pad.bounds.min);
		max = max.max(pad.bounds.max);
		any = true;
	}
	if !any {
		let half = Vec2::splat(40.0);
		return (fallback_center - half, fallback_center + half);
	}
	let margin = Vec2::splat(TRAINING_WALL_MARGIN_M);
	(min - margin, max + margin)
}

fn courtyard_spawns(filled: &DevelopmentCell, plaza_y: f32) -> CourtyardSpawn {
	let (min, max) = development_wall_rect(filled, Vec2::ZERO);
	let center = (min + max) * 0.5;
	let pad_y = filled.pads().next().map(|pad| pad.height).unwrap_or(plaza_y);
	let y = pad_y + capsule_spawn_height();
	let south = Vec2::new(center.x, min.y - TRAINING_COURTYARD_OFFSET_M);
	let away = {
		let d = south - center;
		if d.length_squared() < 1e-4 { -Vec2::Y } else { d.normalize() }
	};
	let player = south;
	let npc = south + away * 1.8 + Vec2::new(-away.y, away.x) * 1.6;
	let look_yaw = (-away.y).atan2(-away.x);
	CourtyardSpawn {
		player: Vec3::new(player.x, y, player.y),
		npc: Vec3::new(npc.x, y, npc.y),
		look_yaw,
	}
}

struct CourtyardSpawn {
	player: Vec3,
	npc: Vec3,
	look_yaw: f32,
}

fn spawn_training_wall(
	commands: &mut Commands,
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	base: &WorldBaseTerrain,
	(min, max): (Vec2, Vec2),
	plaza_y: f32,
) {
	let samples = TerrainPerimeterWall::sample_rectangle(min, max, TRAINING_WALL_STEP_M);
	let terrain_y: Vec<f32> = samples
		.iter()
		.map(|sample| {
			store
				.composed_height_at(layout, sample.x, sample.y)
				.unwrap_or_else(|| base.0.height_at(sample.x, sample.y))
		})
		.collect();
	let wall = TerrainPerimeterWall::from_samples(
		&samples,
		&terrain_y,
		plaza_y,
		TRAINING_WALL_CLEARANCE_M,
	);
	let bounds = building_bounds(&wall);
	for entity in spawn_building_components(commands, &wall, Transform::IDENTITY, bounds) {
		spawn_building_walk_colliders(commands, entity, &wall, BUILDING_FRICTION);
		commands.entity(entity).insert(TrainingPlaza);
	}
}

fn spawn_training_brawler(commands: &mut Commands, at: Vec3, look_yaw: f32) {
	let recipe = MobCharacter {
		num: 42.0,
		build: CharacterBuild::Brawler,
		species: CharacterSpecies::Braidman,
		inventory: CharacterInventory::Grunt,
		brains: CharacterBrains::Brawler,
	}
	.scene_recipe();
	let body = recipe.spawn(
		commands,
		Transform::from_translation(at).with_rotation(Quat::from_rotation_y(look_yaw)),
	);
	commands.entity(body).insert(TrainingBrawler);
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
	let eye = look + Vec3::new(-follow.distance, follow.height, 0.0);
	let parked = Transform::from_translation(eye).looking_at(look, Vec3::Y);
	*camera_tf = parked;
	*camera_global = GlobalTransform::from(parked);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn training_cell_is_centered_on_the_origin() {
		let cell = training_development_cell();
		assert!(((cell.min.x + cell.max.x) * 0.5).abs() < 1e-4);
		assert!(((cell.min.z + cell.max.z) * 0.5).abs() < 1e-4);
		assert!((cell.max.x - cell.min.x - DEVELOPMENT_CELL_SIZE).abs() < 1e-4);
	}

	#[test]
	fn seed_42_picks_a_filled_kind() {
		let cell = training_development_cell();
		let config = DevelopmentConfig::from_world_seed(42);
		let kind = DevelopmentKind::pick_filled(cell, &config);
		assert_ne!(kind, DevelopmentKind::Empty);
	}

	#[test]
	fn courtyard_sits_outside_the_wall_rect() {
		let filled = DevelopmentCell::empty(training_development_cell());
		let spawn = courtyard_spawns(&filled, 10.0);
		assert!(spawn.player.z < -40.0);
		assert!(spawn.npc.distance(spawn.player) > 1.0);
	}

	#[test]
	fn training_brawler_is_an_armed_braidman() {
		let recipe = MobCharacter {
			num: 42.0,
			build: CharacterBuild::Brawler,
			species: CharacterSpecies::Braidman,
			inventory: CharacterInventory::Grunt,
			brains: CharacterBrains::Brawler,
		}
		.scene_recipe();
		assert!(recipe.armed());
		assert_eq!(recipe.brains, CharacterBrains::Brawler);
	}
}
