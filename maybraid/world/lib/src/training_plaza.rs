//! One seeded Richmond development on the Training FinePatch, walled into a
//! flat courtyard arena with an FFA roster inside.

use avian3d::prelude::{Collider, LinearVelocity, Position, RigidBody};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{OffTerrainAnchor, Player};
use durham_terrain_models::{
	PresentedTerrainScene, TerrainCellLayout, TerrainEntryStore, TerrainTrimeshCollider,
	WorldBaseTerrain,
};
use firearm_intelligence::{FirearmEngagement, RulesOfEngagement};
use firearms::WeaponFired;
use lod::gen::Id;
use mob_characters::{
	CharacterBrains, CharacterBuild, CharacterInventory, CharacterSpecies, MobCharacter,
};
use npc_intelligence::NpcInstallOverrides;
use player::capsule_spawn_height;
use player_camera::FollowCamera;
use procedural_common::SeededHash;
use richmond_building_components::{building_bounds, spawn_building_components};
use richmond_building_physics::{BUILDING_FRICTION, spawn_building_walk_colliders};
use richmond_buildings::wall_demo::TerrainPerimeterWall;
use richmond_development_models::{
	DEVELOPMENT_CELL_SIZE, DevelopmentCell, DevelopmentConfig, DevelopmentEntryStore,
	DevelopmentFinish, DevelopmentHosts, DevelopmentKind, PadParams, TerrainWithPads,
};

use crate::PlayerSpawnXz;
use crate::control::WorldSurfaceReady;
use crate::training::TrainingGrounds;

const TRAINING_WALL_STEP_M: f32 = 8.0;
const TRAINING_WALL_HEIGHT_M: f32 = 20.0;
/// Courtyard band between the building footprint and the wall.
const TRAINING_ARENA_MARGIN_M: f32 = 16.0;
/// Keeps the courtyard plus its ease inside the 320 m FinePatch.
const TRAINING_ARENA_MAX_HALF_M: f32 = 128.0;
/// Flatten runs under the wall so its base never meets the ease slope.
const TRAINING_COURTYARD_OVERHANG_M: f32 = 3.0;
const TRAINING_COURTYARD_EASE_M: f32 = 24.0;
/// FFA roster size and spacing along the courtyard band.
const TRAINING_ROSTER: usize = 6;
const TRAINING_NEIGHBOR_M: f32 = 14.0;
const TRAINING_SIGHT_M: f32 = 80.0;

/// The development, roster, and wall have been stamped for this Training session.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrainingPlazaMounted;

/// Pads are composed; unveil waits until the stamped FinePatch colliders exist.
#[derive(Resource, Debug)]
pub(crate) struct TrainingPlazaStamped {
	cell_id: Id,
	terrain_ids: Vec<Id>,
	arena: TrainingArena,
}

/// Stamped on Training fixtures so Leave can despawn them.
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct TrainingPlaza;

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct TrainingBrawler;

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct TrainingPaddedFill;

/// Walled courtyard around the development and the seats inside it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TrainingArena {
	center: Vec2,
	/// World-axis half extents of the wall rectangle.
	half: Vec2,
	plaza_y: f32,
	player: Vec3,
	/// `(translation, yaw)` per roster member.
	npcs: Vec<(Vec3, f32)>,
}

impl TrainingArena {
	/// Arena around `footprint` (world-axis half extents), seats on the band
	/// midline: player due south, roster at FFA neighbor spacing either side.
	fn around(center: Vec2, footprint: Vec2, plaza_y: f32) -> Self {
		let half = (footprint + Vec2::splat(TRAINING_ARENA_MARGIN_M))
			.min(Vec2::splat(TRAINING_ARENA_MAX_HALF_M));
		let band = (footprint.min(half) + half) * 0.5;
		let y = plaza_y + capsule_spawn_height();
		let seat = |arc: f32| {
			let xz = center + Self::midline_point(band, arc);
			Vec3::new(xz.x, y, xz.y)
		};
		let player = seat(0.0);
		let npcs = (0..TRAINING_ROSTER)
			.map(|slot| {
				let rank = (slot / 2 + 1) as f32;
				let side = if slot % 2 == 0 { 1.0 } else { -1.0 };
				let at = seat(side * rank * TRAINING_NEIGHBOR_M);
				(at, facing_yaw(at, player))
			})
			.collect();
		Self { center, half, plaza_y, player, npcs }
	}

	/// Walk `arc` metres counter-clockwise (seen from above, +X first) from
	/// the south midpoint of a rectangle with half extents `band`.
	fn midline_point(band: Vec2, arc: f32) -> Vec2 {
		let perimeter = 4.0 * (band.x + band.y);
		let mut s = arc.rem_euclid(perimeter);
		let legs = [
			(Vec2::new(0.0, -band.y), Vec2::new(band.x, -band.y)),
			(Vec2::new(band.x, -band.y), Vec2::new(band.x, band.y)),
			(Vec2::new(band.x, band.y), Vec2::new(-band.x, band.y)),
			(Vec2::new(-band.x, band.y), Vec2::new(-band.x, -band.y)),
			(Vec2::new(-band.x, -band.y), Vec2::new(0.0, -band.y)),
		];
		for (from, to) in legs {
			let len = from.distance(to);
			if s <= len {
				return from + (to - from) * (s / len.max(1e-4));
			}
			s -= len;
		}
		Vec2::new(0.0, -band.y)
	}

	fn wall_rect(&self) -> (Vec2, Vec2) {
		(self.center - self.half, self.center + self.half)
	}

	fn courtyard_half(&self) -> Vec2 {
		self.half + Vec2::splat(TRAINING_COURTYARD_OVERHANG_M)
	}

	fn player_facing(&self) -> Vec3 {
		let toward = Vec3::new(self.center.x - self.player.x, 0.0, self.center.y - self.player.z);
		toward.try_normalize().unwrap_or(Vec3::Z)
	}
}

/// Bevy yaw that turns `-Z` toward `target`.
fn facing_yaw(from: Vec3, target: Vec3) -> f32 {
	let d = target - from;
	(-d.x).atan2(-d.z)
}

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
	stamped: Option<Res<TrainingPlazaStamped>>,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	mut developments: ResMut<DevelopmentEntryStore>,
	mut commands: Commands,
) {
	if !grounds.0 || !ready.0 || mounted.is_some() || stamped.is_some() {
		return;
	}
	let cell = training_development_cell();
	let center = Vec2::new((cell.min.x + cell.max.x) * 0.5, (cell.min.z + cell.max.z) * 0.5);
	let plaza_y = store
		.composed_height_at(&layout, center.x, center.y)
		.unwrap_or_else(|| base.0.height_at(center.x, center.y));
	let config = DevelopmentConfig::from_world_seed(42);
	let Some((kind, filled, built, arena)) =
		stamp_training_development(&store, &layout, cell, &config, plaza_y)
	else {
		warn!(target: "world.training", "no Richmond development fitted on the Training patch");
		commands.insert_resource(TrainingPlazaMounted);
		return;
	};
	info!(target: "world.training", "stamped {kind:?} on the Training patch");
	let cell_id = Id::from_cell(filled.cell);
	let terrain_ids = stamp_training_terrain(&mut commands, &store, &mut developments, &filled);
	for host in built.hosts() {
		for entity in host.spawn(&mut commands) {
			commands.entity(entity).insert(TrainingPlaza);
		}
	}
	spawn_training_wall(&mut commands, &store, &developments, &layout, &base, &arena, config.seed);
	if terrain_ids.is_empty() {
		warn!(target: "world.training", "no FinePatch cells overlapped the development pads");
	}
	commands.insert_resource(TrainingPlazaStamped { cell_id, terrain_ids, arena });
}

/// Hide raw FinePatch cells once the pad-modulated meshes carry colliders.
pub(crate) fn promote_training_plaza(
	grounds: Res<TrainingGrounds>,
	stamped: Option<Res<TrainingPlazaStamped>>,
	mounted: Option<Res<TrainingPlazaMounted>>,
	ready_fills: Query<(), (With<TrainingPaddedFill>, With<TerrainTrimeshCollider>)>,
	mut raw: Query<(Entity, &PresentedTerrainScene, &mut Visibility)>,
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
	if !grounds.0 || mounted.is_some() {
		return;
	}
	let Some(stamped) = stamped else {
		return;
	};
	if !stamped.terrain_ids.is_empty() && ready_fills.iter().count() < stamped.terrain_ids.len() {
		return;
	}
	for (entity, presented, mut visibility) in &mut raw {
		if !stamped.terrain_ids.contains(&presented.0) {
			continue;
		}
		*visibility = Visibility::Hidden;
		commands.entity(entity).remove::<(Collider, RigidBody, TerrainTrimeshCollider)>();
	}
	let arena = &stamped.arena;
	spawn_xz.0 = Some(arena.player.xz());
	for (slot, (at, yaw)) in arena.npcs.iter().enumerate() {
		spawn_training_brawler(&mut commands, slot, *at, *yaw);
	}
	seat_player_at(&mut players, &mut cameras, arena.player, arena.player_facing());
	// Terrain snap and void recovery sample the raw FinePatch, which is below
	// the courtyard wherever the terrace fills. The anchor keeps the seat on
	// the padded collider.
	for player in &player_ids {
		commands.entity(player).insert(OffTerrainAnchor { translation: arena.player });
	}
	commands.insert_resource(TrainingPlazaMounted);
}

/// FFA bell: the roster holds fire until the player's first shot.
pub(crate) fn release_training_brawlers(
	players: Query<Entity, With<Player>>,
	mut fired: MessageReader<WeaponFired>,
	mut brawlers: Query<&mut FirearmEngagement, With<TrainingBrawler>>,
) {
	if !fired.read().any(|event| players.contains(event.shooter)) {
		return;
	}
	for mut rules in &mut brawlers {
		if rules.rules != RulesOfEngagement::WeaponsFree {
			rules.set_rules(RulesOfEngagement::WeaponsFree);
		}
	}
}

pub(crate) fn clear_training_plaza(
	grounds: Res<TrainingGrounds>,
	mounted: Option<Res<TrainingPlazaMounted>>,
	stamped: Option<Res<TrainingPlazaStamped>>,
	mut developments: ResMut<DevelopmentEntryStore>,
	mut spawn_xz: ResMut<PlayerSpawnXz>,
	mut commands: Commands,
	fixtures: Query<Entity, Or<(With<TrainingPlaza>, With<TrainingBrawler>)>>,
	anchored: Query<Entity, (With<Player>, With<OffTerrainAnchor>)>,
) {
	if grounds.0 || (mounted.is_none() && stamped.is_none()) {
		return;
	}
	if let Some(stamped) = stamped.as_deref() {
		developments.remove_cell(stamped.cell_id);
	}
	for entity in &fixtures {
		commands.entity(entity).despawn();
	}
	for player in &anchored {
		commands.entity(player).remove::<OffTerrainAnchor>();
	}
	spawn_xz.0 = None;
	commands.remove_resource::<TrainingPlazaStamped>();
	commands.remove_resource::<TrainingPlazaMounted>();
}

/// First single-terrace kind from the seeded pick onward, re-padded as one
/// flat walled courtyard. Multi-terrace kinds cannot share a level arena.
fn stamp_training_development(
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	cell: Aabb3d,
	config: &DevelopmentConfig,
	height: f32,
) -> Option<(
	DevelopmentKind,
	DevelopmentCell,
	richmond_development_models::BuiltDevelopment,
	TrainingArena,
)> {
	let preferred = DevelopmentKind::pick_filled(cell, config);
	let start = DevelopmentKind::FILLED.iter().position(|kind| *kind == preferred).unwrap_or(0);
	let count = DevelopmentKind::FILLED.len();
	for kind in (0..count).map(|i| DevelopmentKind::FILLED[(start + i) % count]) {
		let Some(filled) = DevelopmentCell::fill(store, layout, cell, kind, config, height) else {
			continue;
		};
		let Some(footprint) = filled.footprint_half_extents() else {
			continue;
		};
		let center = Vec2::new((cell.min.x + cell.max.x) * 0.5, (cell.min.z + cell.max.z) * 0.5);
		let arena = TrainingArena::around(center, footprint, height);
		let params = PadParams { berm: 0.0, ease: TRAINING_COURTYARD_EASE_M, round: 0.0 };
		let Some(walled) = filled.with_courtyard(arena.courtyard_half(), params) else {
			continue;
		};
		let Some(built) = walled.built(config.seed as i32) else {
			continue;
		};
		return Some((kind, walled, built, arena));
	}
	None
}

fn stamp_training_terrain(
	commands: &mut Commands,
	terrain_store: &TerrainEntryStore,
	developments: &mut DevelopmentEntryStore,
	filled: &DevelopmentCell,
) -> Vec<Id> {
	developments.insert_cell(Id::from_cell(filled.cell), filled.clone());
	let Some(region) = pad_influence_region(filled) else {
		return Vec::new();
	};
	let pads: Vec<_> = filled.pad_complexes().cloned().collect();
	let mut stamped = Vec::new();
	for id in terrain_store.terrain_ids_overlapping(region) {
		let Some(terrain) = terrain_store.terrain(id) else {
			continue;
		};
		let padded = TerrainWithPads::compose(terrain, pads.iter());
		let entity = padded.spawn_fill(commands, Visibility::Inherited, true);
		commands.entity(entity).insert((
			Name::new("Training padded terrain"),
			TrainingPlaza,
			TrainingPaddedFill,
		));
		stamped.push(id);
	}
	stamped
}

fn pad_influence_region(filled: &DevelopmentCell) -> Option<Aabb3d> {
	let mut min = Vec2::splat(f32::INFINITY);
	let mut max = Vec2::splat(f32::NEG_INFINITY);
	let mut any = false;
	for pad in filled.pad_complexes() {
		min = min.min(pad.bounds.min);
		max = max.max(pad.bounds.max);
		any = true;
	}
	any.then(|| {
		Aabb3d::from_min_max(Vec3::new(min.x, -10_000.0, min.y), Vec3::new(max.x, 10_000.0, max.y))
	})
}

fn stamped_height(
	store: &TerrainEntryStore,
	developments: &DevelopmentEntryStore,
	layout: &TerrainCellLayout,
	base: &WorldBaseTerrain,
	x: f32,
	z: f32,
) -> f32 {
	let raw = store
		.composed_height_at(layout, x, z)
		.unwrap_or_else(|| base.0.height_at(x, z));
	let probe = Aabb3d::from_min_max(
		Vec3::new(x - 0.5, -10_000.0, z - 0.5),
		Vec3::new(x + 0.5, 10_000.0, z + 0.5),
	);
	developments.merged_pad_complex(probe).modify_elevation(raw, x, z)
}

fn spawn_training_wall(
	commands: &mut Commands,
	store: &TerrainEntryStore,
	developments: &DevelopmentEntryStore,
	layout: &TerrainCellLayout,
	base: &WorldBaseTerrain,
	arena: &TrainingArena,
	seed: u32,
) {
	let (min, max) = arena.wall_rect();
	let samples = TerrainPerimeterWall::sample_rectangle(min, max, TRAINING_WALL_STEP_M);
	let terrain_y: Vec<f32> = samples
		.iter()
		.map(|sample| stamped_height(store, developments, layout, base, sample.x, sample.y))
		.collect();
	let wall = TerrainPerimeterWall::from_samples(
		&samples,
		&terrain_y,
		arena.plaza_y,
		TRAINING_WALL_HEIGHT_M,
	)
	.with_material(DevelopmentFinish::rampart_stone(SeededHash::new(seed)));
	let bounds = building_bounds(&wall);
	for entity in spawn_building_components(commands, &wall, Transform::IDENTITY, bounds) {
		spawn_building_walk_colliders(commands, entity, &wall, BUILDING_FRICTION);
		commands.entity(entity).insert(TrainingPlaza);
	}
}

fn training_brawler(slot: usize) -> MobCharacter {
	MobCharacter {
		num: 42.0 + slot as f32,
		build: CharacterBuild::Brawler,
		species: CharacterSpecies::Braidman,
		inventory: CharacterInventory::Grunt,
		brains: CharacterBrains::Brawler,
	}
}

fn spawn_training_brawler(commands: &mut Commands, slot: usize, at: Vec3, yaw: f32) {
	let recipe = training_brawler(slot).scene_recipe();
	let body = recipe.spawn(
		commands,
		Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw)),
	);
	commands
		.entity(body)
		.insert((TrainingBrawler, NpcInstallOverrides::ffa(TRAINING_SIGHT_M)));
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

	fn inside(arena: &TrainingArena, at: Vec3) -> bool {
		let (min, max) = arena.wall_rect();
		at.x > min.x && at.x < max.x && at.z > min.y && at.z < max.y
	}

	fn outside_footprint(center: Vec2, footprint: Vec2, at: Vec3) -> bool {
		let d = (at.xz() - center).abs();
		d.x > footprint.x || d.y > footprint.y
	}

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
	fn everyone_spawns_inside_the_wall_and_outside_the_building() {
		let footprint = Vec2::new(36.0, 28.0);
		let arena = TrainingArena::around(Vec2::ZERO, footprint, 10.0);
		assert_eq!(arena.npcs.len(), TRAINING_ROSTER);
		for at in std::iter::once(arena.player).chain(arena.npcs.iter().map(|(at, _)| *at)) {
			assert!(inside(&arena, at), "{at} is outside the wall");
			assert!(outside_footprint(Vec2::ZERO, footprint, at), "{at} is inside the building");
		}
	}

	#[test]
	fn roster_is_close_quarters_around_the_player() {
		let arena = TrainingArena::around(Vec2::ZERO, Vec2::new(36.0, 36.0), 0.0);
		let nearest = arena
			.npcs
			.iter()
			.map(|(at, _)| at.distance(arena.player))
			.fold(f32::INFINITY, f32::min);
		assert!((10.0..=TRAINING_NEIGHBOR_M + 1e-3).contains(&nearest), "nearest {nearest}");
		for (at, _) in &arena.npcs {
			assert!(at.distance(arena.player) < 60.0, "{at} is too far for close quarters");
		}
	}

	#[test]
	fn roster_faces_the_player() {
		let arena = TrainingArena::around(Vec2::ZERO, Vec2::new(30.0, 30.0), 0.0);
		for (at, yaw) in &arena.npcs {
			let forward = Quat::from_rotation_y(*yaw) * Vec3::NEG_Z;
			let toward = (arena.player - *at).with_y(0.0).normalize();
			assert!(forward.dot(toward) > 0.99);
		}
	}

	#[test]
	fn arena_stays_on_the_fine_patch_for_wide_developments() {
		let arena = TrainingArena::around(Vec2::ZERO, Vec2::splat(120.0), 0.0);
		let reach = arena.courtyard_half() + Vec2::splat(TRAINING_COURTYARD_EASE_M);
		assert!(reach.max_element() <= 160.0, "courtyard reach {reach}");
		for (at, _) in &arena.npcs {
			assert!(inside(&arena, *at));
		}
	}

	#[test]
	fn empty_cell_has_no_pad_influence() {
		assert!(pad_influence_region(&DevelopmentCell::empty(training_development_cell())).is_none());
	}

	#[test]
	fn les_halles_courtyard_covers_the_wall() -> Result<(), String> {
		let cell = training_development_cell();
		let filled = DevelopmentCell::with_les_halles(cell, 20.0, &DevelopmentConfig::from_world_seed(42));
		let footprint = filled.footprint_half_extents().ok_or("footprint")?;
		let arena = TrainingArena::around(Vec2::ZERO, footprint, 20.0);
		let params = PadParams { berm: 0.0, ease: TRAINING_COURTYARD_EASE_M, round: 0.0 };
		let walled = filled.with_courtyard(arena.courtyard_half(), params).ok_or("courtyard")?;
		let complex = walled.pad_complexes().next().ok_or("pad")?;
		let (min, max) = arena.wall_rect();
		for sample in TerrainPerimeterWall::sample_rectangle(min, max, TRAINING_WALL_STEP_M) {
			let y = complex.modify_elevation(-15.0, sample.x, sample.y);
			if (y - 20.0).abs() > 1e-3 {
				return Err(format!("wall station {sample} sits at {y}, expected 20"));
			}
		}
		Ok(())
	}

	#[test]
	fn leaving_training_drops_the_seat_anchor() -> Result<(), bevy::ecs::system::RunSystemError> {
		use bevy::ecs::system::RunSystemOnce;
		let mut world = World::new();
		world.insert_resource(TrainingGrounds(false));
		world.insert_resource(TrainingPlazaMounted);
		world.insert_resource(DevelopmentEntryStore::default());
		world.insert_resource(PlayerSpawnXz(Some(Vec2::ONE)));
		let player = world.spawn((Player, OffTerrainAnchor { translation: Vec3::Y })).id();
		world.run_system_once(clear_training_plaza)?;
		assert!(world.get::<OffTerrainAnchor>(player).is_none());
		assert_eq!(world.resource::<PlayerSpawnXz>().0, None);
		Ok(())
	}

	#[test]
	fn training_brawler_is_an_armed_braidman() {
		let recipe = training_brawler(0).scene_recipe();
		assert!(recipe.armed());
		assert_eq!(recipe.brains, CharacterBrains::Brawler);
	}
}
