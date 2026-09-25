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
use lod::gen::Id;
use maybraid_mobs::{Mob, MobKind, MobScene};
use mob_intelligence::MemberOf;
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
/// Two brawler mobs flank the player along the courtyard band.
const TRAINING_MOBS: usize = 2;
/// Brawler rosters roll 6–12; Training keeps the floor so both mobs fit the band.
const TRAINING_MOB_MEMBERS: usize = 6;
const TRAINING_MEMBER_SPACING_M: f32 = 3.0;
/// FFA player clearance: nearest brawler this far along the band.
const TRAINING_PLAYER_CLEARANCE_M: f32 = 10.0;
const TRAINING_MOB_SEED: f32 = 42.0;

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

/// Training brawler mob host. Its members carry [`MemberOf`] back to it.
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
	mobs: Vec<TrainingMob>,
}

/// One brawler mob laid out along the courtyard band.
#[derive(Clone, Debug, PartialEq)]
struct TrainingMob {
	/// Host on the plaza surface.
	host: Vec3,
	/// Member feet XZ, in roster slot order.
	members: Vec<Vec2>,
}

impl TrainingMob {
	/// Brawler mob whose first members stand at [`Self::members`]. Brawler
	/// affiliations join and antagonize the FFA group, so members fight each
	/// other, the other mob, and the player.
	fn scene(&self, num: f32) -> MobScene {
		let mut mob = Mob::of_kind(MobKind::Brawler, num);
		mob.roster.members.truncate(self.members.len());
		for (member, xz) in mob.roster.members.iter_mut().zip(&self.members) {
			let offset = *xz - self.host.xz();
			member.offset = Vec3::new(offset.x, member.offset.y, offset.y);
		}
		mob.into_scene()
	}
}

impl TrainingArena {
	/// Arena around `footprint` (world-axis half extents). The player sits due
	/// south on the band midline, and one brawler mob runs along the band on
	/// either side from the FFA clearance outward.
	fn around(center: Vec2, footprint: Vec2, plaza_y: f32) -> Self {
		let half = (footprint + Vec2::splat(TRAINING_ARENA_MARGIN_M))
			.min(Vec2::splat(TRAINING_ARENA_MAX_HALF_M));
		let band = (footprint.min(half) + half) * 0.5;
		let on_band = |arc: f32| center + Self::midline_point(band, arc);
		let player = on_band(0.0);
		let player = Vec3::new(player.x, plaza_y + capsule_spawn_height(), player.y);
		let mobs = (0..TRAINING_MOBS)
			.map(|mob| {
				let side = if mob % 2 == 0 { 1.0 } else { -1.0 };
				let arcs: Vec<f32> = (0..TRAINING_MOB_MEMBERS)
					.map(|slot| {
						side * (TRAINING_PLAYER_CLEARANCE_M
							+ slot as f32 * TRAINING_MEMBER_SPACING_M)
					})
					.collect();
				let mid = arcs.iter().sum::<f32>() / arcs.len() as f32;
				let host = on_band(mid);
				TrainingMob {
					host: Vec3::new(host.x, plaza_y, host.y),
					members: arcs.into_iter().map(on_band).collect(),
				}
			})
			.collect();
		Self { center, half, plaza_y, player, mobs }
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
	for (index, mob) in arena.mobs.iter().enumerate() {
		let host = mob
			.scene(TRAINING_MOB_SEED + index as f32)
			.spawn(&mut commands, Transform::from_translation(mob.host));
		commands.entity(host).insert(TrainingBrawler);
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

pub(crate) fn clear_training_plaza(
	grounds: Res<TrainingGrounds>,
	mounted: Option<Res<TrainingPlazaMounted>>,
	stamped: Option<Res<TrainingPlazaStamped>>,
	mut developments: ResMut<DevelopmentEntryStore>,
	mut spawn_xz: ResMut<PlayerSpawnXz>,
	mut commands: Commands,
	fixtures: Query<Entity, Or<(With<TrainingPlaza>, With<TrainingBrawler>)>>,
	brawler_hosts: Query<(), With<TrainingBrawler>>,
	members: Query<(Entity, &MemberOf)>,
	anchored: Query<Entity, (With<Player>, With<OffTerrainAnchor>)>,
) {
	if grounds.0 || (mounted.is_none() && stamped.is_none()) {
		return;
	}
	if let Some(stamped) = stamped.as_deref() {
		developments.remove_cell(stamped.cell_id);
	}
	// Respawned members are not tied to a roster stub, so dropping the host
	// alone would strand them.
	for (entity, member) in &members {
		if brawler_hosts.contains(member.mob) {
			commands.entity(entity).despawn();
		}
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

	fn member_feet(arena: &TrainingArena) -> impl Iterator<Item = Vec3> + '_ {
		arena
			.mobs
			.iter()
			.flat_map(|mob| mob.members.iter().map(|xz| Vec3::new(xz.x, arena.plaza_y, xz.y)))
	}

	#[test]
	fn everyone_spawns_inside_the_wall_and_outside_the_building() {
		let footprint = Vec2::new(36.0, 28.0);
		let arena = TrainingArena::around(Vec2::ZERO, footprint, 10.0);
		assert_eq!(arena.mobs.len(), TRAINING_MOBS);
		for at in std::iter::once(arena.player)
			.chain(member_feet(&arena))
			.chain(arena.mobs.iter().map(|mob| mob.host))
		{
			assert!(inside(&arena, at), "{at} is outside the wall");
			assert!(outside_footprint(Vec2::ZERO, footprint, at), "{at} is inside the building");
		}
	}

	#[test]
	fn brawler_mobs_flank_the_player_at_close_quarters() {
		let arena = TrainingArena::around(Vec2::ZERO, Vec2::new(36.0, 36.0), 0.0);
		let player = arena.player.with_y(0.0);
		for mob in &arena.mobs {
			let nearest = mob
				.members
				.iter()
				.map(|xz| xz.distance(player.xz()))
				.fold(f32::INFINITY, f32::min);
			assert!(
				(TRAINING_PLAYER_CLEARANCE_M - 1e-3..=14.0).contains(&nearest),
				"nearest {nearest}"
			);
			for xz in &mob.members {
				assert!(xz.distance(player.xz()) < 30.0, "{xz} is too far for close quarters");
			}
		}
		let east = arena.mobs[0].host.x - player.x;
		let west = arena.mobs[1].host.x - player.x;
		assert!(east > 0.0 && west < 0.0, "mobs should sit on either side: {east}, {west}");
	}

	#[test]
	fn mob_scene_seats_members_where_the_arena_planned() {
		let arena = TrainingArena::around(Vec2::ZERO, Vec2::new(30.0, 30.0), 4.0);
		let mob = &arena.mobs[0];
		let scene = mob.scene(TRAINING_MOB_SEED);
		assert_eq!(scene.mob.kind, MobKind::Brawler);
		assert_eq!(scene.mob.roster.members.len(), TRAINING_MOB_MEMBERS);
		for (member, xz) in scene.mob.roster.members.iter().zip(&mob.members) {
			let feet = mob.host.xz() + member.offset.xz();
			assert!(feet.distance(*xz) < 1e-3, "{feet} vs {xz}");
			assert!(member.offset.y > 0.0);
			assert!(member.character.armed());
		}
	}

	#[test]
	fn brawler_mobs_fight_each_other_and_the_player() {
		use maybraid_mobs::player_affiliations;
		use threat_intelligence::ThreatId;
		let scene = TrainingArena::around(Vec2::ZERO, Vec2::splat(30.0), 0.0).mobs[0]
			.scene(TRAINING_MOB_SEED);
		let pack = &scene.mob.intelligence.affiliations;
		let a = pack.for_member(ThreatId(1));
		let b = pack.for_member(ThreatId(2));
		let player = player_affiliations(ThreatId(3));
		assert!(a.threat_weight(&b, 0.0) >= 1.0, "same-mob brawlers must be FFA");
		assert!(a.threat_weight(&player, 0.0) >= 1.0);
		assert!(player.threat_weight(&a, 0.0) >= 1.0);
	}

	#[test]
	fn arena_stays_on_the_fine_patch_for_wide_developments() {
		let arena = TrainingArena::around(Vec2::ZERO, Vec2::splat(120.0), 0.0);
		let reach = arena.courtyard_half() + Vec2::splat(TRAINING_COURTYARD_EASE_M);
		assert!(reach.max_element() <= 160.0, "courtyard reach {reach}");
		for at in member_feet(&arena) {
			assert!(inside(&arena, at));
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
		let host = world.spawn(TrainingBrawler).id();
		let member = world.spawn(MemberOf { mob: host, slot: 0 }).id();
		let stranger_host = world.spawn_empty().id();
		let stranger = world.spawn(MemberOf { mob: stranger_host, slot: 0 }).id();
		world.run_system_once(clear_training_plaza)?;
		assert!(world.get::<OffTerrainAnchor>(player).is_none());
		assert_eq!(world.resource::<PlayerSpawnXz>().0, None);
		assert!(world.get_entity(host).is_err());
		assert!(world.get_entity(member).is_err(), "respawned members must leave with the mob");
		assert!(world.get_entity(stranger).is_ok());
		Ok(())
	}
}
