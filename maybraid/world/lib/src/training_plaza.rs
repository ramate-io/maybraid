//! One seeded Richmond development on the Training FinePatch, walled into a
//! flat courtyard arena with an FFA roster inside. Each [`TrainingMap`]
//! stamps its own site, development, and roster; its later lives respawn
//! on the same plaza.

use avian3d::prelude::{Collider, LinearVelocity, Position, RigidBody};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::player::{holding_elevation, player_spawn_point_at};
use chico_vegetation_on_terrain_playground::{OffTerrainAnchor, Player};
use durham_terrain_models::{
	PresentedTerrainScene, TerrainCellLayout, TerrainEntryStore, TerrainSuperseded,
	TerrainTrimeshCollider, WorldBaseTerrain,
};
use lod::gen::Id;
use maybraid_mobs::{Mob, MobKind, MobScene};
use mob_characters::CharacterSpecies;
use mob_intelligence::MemberOf;
use player::capsule_spawn_height;
use player_camera::FollowCamera;
use procedural_common::SeededHash;
use richmond_building_components::{building_bounds, spawn_building_components};
use richmond_building_physics::{BUILDING_FRICTION, spawn_building_walk_colliders};
use richmond_buildings::wall_demo::TerrainPerimeterWall;
use richmond_development_models::{
	DEVELOPMENT_CELL_SIZE, DevelopmentCell, DevelopmentConfig, DevelopmentEntryStore,
	DevelopmentFinish, DevelopmentHost, DevelopmentHosts, DevelopmentKind, PadParams,
	TerrainWithPads,
};

use crate::PlayerSpawnXz;
use crate::control::WorldSurfaceReady;
use crate::training::{TrainingGrounds, TrainingMap, TrainingRound};

const TRAINING_WALL_STEP_M: f32 = 8.0;
const TRAINING_WALL_HEIGHT_M: f32 = 20.0;
/// Courtyard band between the building footprint and the wall.
const TRAINING_ARENA_MARGIN_M: f32 = 16.0;
/// Keeps the courtyard plus its ease inside the 320 m FinePatch.
const TRAINING_ARENA_MAX_HALF_M: f32 = 128.0;
/// Flatten runs under the wall so its base never meets the ease slope.
const TRAINING_COURTYARD_OVERHANG_M: f32 = 3.0;
const TRAINING_COURTYARD_EASE_M: f32 = 24.0;
/// FFA headcount, split across one Brawler squad per development POI.
const TRAINING_ROSTER: usize = 16;
const TRAINING_MIN_SQUADS: usize = 2;
const TRAINING_MAX_SQUADS: usize = 4;
/// Sunflower scale: neighbours land ≳ 2.8 m apart, clear of agent separation.
const TRAINING_SQUAD_SPREAD_M: f32 = 1.6;
const GOLDEN_ANGLE: f32 = 2.399_963;
/// Squads try rings from just off their POI's building outward.
const TRAINING_POI_STANDOFF_M: f32 = 2.0;
const TRAINING_RING_STEP_M: f32 = 3.0;
const TRAINING_RING_STEPS: usize = 6;
const TRAINING_RING_BEARINGS: usize = 24;
/// Keeps capsules off building walls and the perimeter wall.
const TRAINING_OBSTACLE_MARGIN_M: f32 = 1.5;
/// Each squad starts as its own knot of fighting.
const TRAINING_SQUAD_GAP_M: f32 = 8.0;
/// FFA player clearance: nearest brawler this far from the seat.
const TRAINING_PLAYER_CLEARANCE_M: f32 = 10.0;
/// Seed stride for extra Brawler rolls when a squad outgrows its roster.
const TRAINING_SPARE_ROLL: f32 = 100.0;
const TRAINING_SPECIES: [CharacterSpecies; 6] = CharacterSpecies::PLAYER_SCALE_BIPEDS;
/// Hosts sharing one POI (Les Halles storeys) merge within this.
const TRAINING_POI_MERGE_M: f32 = 1.0;

/// The development, roster, and wall have been stamped for this round's map.
/// Also set, with nothing stamped, once every site the round tried fit no
/// development.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingPlazaMounted(pub TrainingRound);

impl TrainingPlazaMounted {
	pub fn serves(&self, round: TrainingRound) -> bool {
		self.0.map() == round.map()
	}
}

/// Pads are composed; unveil waits until the stamped FinePatch colliders exist.
#[derive(Resource, Debug)]
pub(crate) struct TrainingPlazaStamped {
	round: TrainingRound,
	cell_id: Id,
	terrain_ids: Vec<Id>,
	arena: TrainingArena,
}

impl TrainingPlazaStamped {
	fn fills_ready(&self, cooked: usize) -> bool {
		self.terrain_ids.is_empty() || cooked >= self.terrain_ids.len()
	}
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
	/// Development footprint half extents, capped at the wall.
	footprint: Vec2,
	/// Courtyard band midline half extents, between the footprint and the wall.
	band: Vec2,
	plaza_y: f32,
	player: Vec3,
	mobs: Vec<TrainingMob>,
}

/// Development building rects and POI anchors, in world XZ.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TrainingSite {
	obstacles: Vec<(Vec2, Vec2)>,
	/// POI center and the rect of the building(s) it pins.
	pois: Vec<(Vec2, (Vec2, Vec2))>,
}

impl TrainingSite {
	/// Host rects are clipped to the development footprint: some storey
	/// bounds overhang the pad by tens of metres.
	fn of_hosts(hosts: &[DevelopmentHost], center: Vec2, footprint: Vec2) -> Self {
		let (lo, hi) = (center - footprint, center + footprint);
		let mut site = Self::default();
		for host in hosts {
			let transform = host.transform();
			let (min, max) = Self::world_rect(&transform, host.local_bounds());
			let (min, max) = (min.clamp(lo, hi), max.clamp(lo, hi));
			if min.cmpge(max).any() {
				continue;
			}
			site.obstacles.push((min, max));
			if host.discoverable_place().is_some() {
				site.add_poi(transform.translation.xz().clamp(min, max), (min, max));
			}
		}
		site
	}

	/// One building covering `footprint`, with its POI at the center.
	#[cfg(test)]
	fn single(center: Vec2, footprint: Vec2) -> Self {
		let rect = (center - footprint, center + footprint);
		let mut site = Self { obstacles: vec![rect], pois: Vec::new() };
		site.add_poi(center, rect);
		site
	}

	fn add_poi(&mut self, at: Vec2, rect: (Vec2, Vec2)) {
		match self.pois.iter_mut().find(|(poi, _)| poi.distance(at) < TRAINING_POI_MERGE_M) {
			Some((_, (min, max))) => {
				*min = min.min(rect.0);
				*max = max.max(rect.1);
			}
			None => self.pois.push((at, rect)),
		}
	}

	fn blocked(&self, at: Vec2) -> bool {
		let margin = Vec2::splat(TRAINING_OBSTACLE_MARGIN_M);
		self.obstacles
			.iter()
			.any(|(min, max)| at.cmpgt(*min - margin).all() && at.cmplt(*max + margin).all())
	}

	fn world_rect(transform: &Transform, bounds: Aabb3d) -> (Vec2, Vec2) {
		let (lo, hi) = (Vec3::from(bounds.min), Vec3::from(bounds.max));
		let mut min = Vec2::splat(f32::INFINITY);
		let mut max = Vec2::splat(f32::NEG_INFINITY);
		for (x, z) in [(lo.x, lo.z), (hi.x, lo.z), (lo.x, hi.z), (hi.x, hi.z)] {
			let at = transform.transform_point(Vec3::new(x, 0.0, z)).xz();
			min = min.min(at);
			max = max.max(at);
		}
		(min, max)
	}
}

/// One Brawler squad seated around a spot near a POI.
#[derive(Clone, Debug, PartialEq)]
struct TrainingMob {
	/// Host on the plaza surface, at the squad's seat.
	host: Vec3,
	/// Member feet XZ, in roster slot order.
	members: Vec<Vec2>,
}

impl TrainingMob {
	fn sunflower(seat: Vec2, size: usize) -> Vec<Vec2> {
		(0..size)
			.map(|slot| {
				let radius = TRAINING_SQUAD_SPREAD_M * (slot as f32 + 0.5).sqrt();
				seat + Vec2::from_angle(slot as f32 * GOLDEN_ANGLE) * radius
			})
			.collect()
	}

	fn spread(size: usize) -> f32 {
		TRAINING_SQUAD_SPREAD_M * (size.max(1) as f32 - 0.5).sqrt()
	}

	/// Brawler mob whose members stand at [`Self::members`]. Brawler
	/// affiliations join and antagonize the FFA group, so members fight each
	/// other, the other squads, and the player. Members are player-scale
	/// bipeds so every fighter reads at close quarters.
	fn scene(&self, num: f32) -> MobScene {
		let brawlers = |num| Mob::of_kind_among(MobKind::Brawler, num, &TRAINING_SPECIES);
		let mut mob = brawlers(num);
		let mut roll = 1.0;
		while mob.roster.members.len() < self.members.len() {
			let spare = brawlers(num + roll * TRAINING_SPARE_ROLL);
			mob.roster.members.extend(spare.roster.members);
			roll += 1.0;
		}
		mob.roster.members.truncate(self.members.len());
		for (member, xz) in mob.roster.members.iter_mut().zip(&self.members) {
			let offset = *xz - self.host.xz();
			member.offset = Vec3::new(offset.x, member.offset.y, offset.y);
		}
		mob.into_scene()
	}
}

impl TrainingArena {
	/// Walled courtyard around `footprint` (world-axis half extents). The
	/// player defaults to the south band midpoint until [`Self::with_roster`].
	fn around(center: Vec2, footprint: Vec2, plaza_y: f32) -> Self {
		let half = (footprint + Vec2::splat(TRAINING_ARENA_MARGIN_M))
			.min(Vec2::splat(TRAINING_ARENA_MAX_HALF_M));
		let footprint = footprint.min(half);
		let band = (footprint + half) * 0.5;
		let player = center + Self::midline_point(band, 0.0);
		let player = Vec3::new(player.x, plaza_y + capsule_spawn_height(), player.y);
		Self { center, half, footprint, band, plaza_y, player, mobs: Vec::new() }
	}

	/// One Brawler squad per POI (2–4 squads, [`TRAINING_ROSTER`] in all),
	/// each on the first open ring just outside its POI's building. A POI
	/// that seats two squads puts the second on its far side. The player sits
	/// at FFA clearance from the first squad.
	fn with_roster(mut self, site: &TrainingSite) -> Self {
		let fallback = [(self.center, (self.center, self.center))];
		let pois = if site.pois.is_empty() { &fallback[..] } else { &site.pois[..] };
		let squads = pois.len().clamp(TRAINING_MIN_SQUADS, TRAINING_MAX_SQUADS);
		let mut player = None;
		for squad in 0..squads {
			let (poi, building) = pois[squad % pois.len()];
			let bearing = std::f32::consts::PI * (squad / pois.len()) as f32;
			let size = TRAINING_ROSTER / squads + usize::from(squad < TRAINING_ROSTER % squads);
			let (seat, members) = self.seat_squad(site, poi, building, bearing, size, player);
			self.mobs.push(TrainingMob { host: Vec3::new(seat.x, self.plaza_y, seat.y), members });
			if player.is_none() {
				player = Some(self.seat_player(site, poi));
			}
		}
		if let Some(at) = player {
			self.player = Vec3::new(at.x, self.plaza_y + capsule_spawn_height(), at.y);
		}
		self
	}

	fn seat_squad(
		&self,
		site: &TrainingSite,
		poi: Vec2,
		building: (Vec2, Vec2),
		bearing: f32,
		size: usize,
		player: Option<Vec2>,
	) -> (Vec2, Vec<Vec2>) {
		let taken: Vec<Vec2> = self.mobs.iter().flat_map(|mob| mob.members.clone()).collect();
		let fits = |seat: Vec2| {
			let members = TrainingMob::sunflower(seat, size);
			let clear = members.iter().all(|at| {
				self.open(site, *at)
					&& taken.iter().all(|other| at.distance(*other) >= TRAINING_SQUAD_GAP_M)
					&& player.is_none_or(|p| at.distance(p) >= TRAINING_PLAYER_CLEARANCE_M)
			});
			clear.then_some((seat, members))
		};
		let standoff =
			TRAINING_POI_STANDOFF_M + TRAINING_OBSTACLE_MARGIN_M + TrainingMob::spread(size);
		Self::rect_rings(poi, building, standoff, bearing)
			.chain(self.band_seats(poi))
			.find_map(fits)
			.unwrap_or_else(|| {
				let seat = self.band_seats(poi).next().unwrap_or(poi);
				(seat, TrainingMob::sunflower(seat, size))
			})
	}

	/// Beside the first squad, along its ring, at FFA clearance from every member.
	fn seat_player(&self, site: &TrainingSite, poi: Vec2) -> Vec2 {
		let Some(squad) = self.mobs.first() else {
			return self.player.xz();
		};
		let seat = squad.host.xz();
		let outward = (seat - poi).to_angle() + std::f32::consts::FRAC_PI_2;
		let ring = TRAINING_PLAYER_CLEARANCE_M + TrainingMob::spread(squad.members.len());
		Self::rings(seat, ring, outward)
			.find(|at| {
				self.open(site, *at)
					&& squad
						.members
						.iter()
						.all(|member| member.distance(*at) >= TRAINING_PLAYER_CLEARANCE_M)
			})
			.unwrap_or(self.player.xz())
	}

	/// Ring seats around `center` from `radius` outward, fanning both ways
	/// from `bearing`.
	fn rings(center: Vec2, radius: f32, bearing: f32) -> impl Iterator<Item = Vec2> {
		(0..TRAINING_RING_STEPS).flat_map(move |ring| {
			let radius = radius + ring as f32 * TRAINING_RING_STEP_M;
			Self::fan(bearing).map(move |dir| center + dir * radius)
		})
	}

	/// Seats on `building` grown by `standoff` and then ring by ring, cast
	/// from `poi` along bearings fanning both ways from `bearing`.
	fn rect_rings(
		poi: Vec2,
		(min, max): (Vec2, Vec2),
		standoff: f32,
		bearing: f32,
	) -> impl Iterator<Item = Vec2> {
		(0..TRAINING_RING_STEPS).flat_map(move |ring| {
			let grow = Vec2::splat(standoff + ring as f32 * TRAINING_RING_STEP_M);
			let (lo, hi) = (min - grow, max + grow);
			Self::fan(bearing).map(move |dir| {
				let exit = |d: f32, p: f32, lo: f32, hi: f32| match d {
					d if d > 1e-6 => (hi - p) / d,
					d if d < -1e-6 => (lo - p) / d,
					_ => f32::INFINITY,
				};
				let t = exit(dir.x, poi.x, lo.x, hi.x).min(exit(dir.y, poi.y, lo.y, hi.y));
				poi + dir * t
			})
		})
	}

	fn fan(bearing: f32) -> impl Iterator<Item = Vec2> {
		let step = std::f32::consts::TAU / TRAINING_RING_BEARINGS as f32;
		(0..TRAINING_RING_BEARINGS as i32).map(move |fan| {
			let turn = (fan + 1) / 2 * if fan % 2 == 0 { 1 } else { -1 };
			Vec2::from_angle(bearing + turn as f32 * step)
		})
	}

	/// Courtyard band midline seats, nearest `poi` first.
	fn band_seats(&self, poi: Vec2) -> impl Iterator<Item = Vec2> {
		let perimeter = 4.0 * (self.band.x + self.band.y);
		let stations = (perimeter / TRAINING_RING_STEP_M).ceil().max(1.0) as usize;
		let mut seats: Vec<Vec2> = (0..stations)
			.map(|station| {
				let arc = station as f32 * TRAINING_RING_STEP_M;
				self.center + Self::midline_point(self.band, arc)
			})
			.collect();
		seats.sort_by(|a, b| a.distance_squared(poi).total_cmp(&b.distance_squared(poi)));
		seats.into_iter()
	}

	fn open(&self, site: &TrainingSite, at: Vec2) -> bool {
		let inner = self.half - Vec2::splat(TRAINING_OBSTACLE_MARGIN_M);
		(at - self.center).abs().cmplt(inner).all() && !site.blocked(at)
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

	/// Toward the nearest squad, so the fight starts on screen.
	fn player_facing(&self) -> Vec3 {
		let target = self.mobs.first().map_or(self.center, |mob| mob.host.xz());
		let toward = Vec3::new(target.x - self.player.x, 0.0, target.y - self.player.z);
		toward.try_normalize().unwrap_or(Vec3::Z)
	}

	fn brawlers(&self) -> usize {
		self.mobs.iter().map(|mob| mob.members.len()).sum()
	}
}

/// 300 m cell centered on the FinePatch.
fn training_development_cell(center: Vec2) -> Aabb3d {
	let half = DEVELOPMENT_CELL_SIZE * 0.5;
	Aabb3d::from_min_max(
		Vec3::new(center.x - half, 0.0, center.y - half),
		Vec3::new(center.x + half, 1.0, center.y + half),
	)
}

/// Fit the round's Richmond development once the FinePatch collider exists.
/// A site that fits none rerolls to another site for the same round.
pub(crate) fn mount_training_plaza(
	grounds: Res<TrainingGrounds>,
	mut round: ResMut<TrainingRound>,
	ready: Res<WorldSurfaceReady>,
	mounted: Option<Res<TrainingPlazaMounted>>,
	stamped: Option<Res<TrainingPlazaStamped>>,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	mut developments: ResMut<DevelopmentEntryStore>,
	mut commands: Commands,
) {
	// A cell admitted after the stamp stays raw, unstamped hillside inside the courtyard.
	let waiting = !grounds.0
		|| !ready.0
		|| *layout != round.layout()
		|| !store.fills_layout(&layout);
	if waiting || mounted.is_some() || stamped.is_some() {
		return;
	}
	let center = layout.region_center_xz().xz();
	let cell = training_development_cell(center);
	let plaza_y = store
		.composed_height_at(&layout, center.x, center.y)
		.unwrap_or_else(|| base.0.height_at(center.x, center.y));
	let config = DevelopmentConfig::from_world_seed(round.development_seed());
	let Some((kind, filled, built, arena)) =
		stamp_training_development(&store, &layout, cell, &config, plaza_y)
	else {
		let site = round.site();
		if round.site_exhausted() {
			warn!(target: "world.training", "no Richmond development fitted at site {site}");
			commands.insert_resource(TrainingPlazaMounted(*round));
		} else {
			info!(target: "world.training", "no development fitted at site {site}; rerolling");
			*round = round.reroll_site();
		}
		return;
	};
	let hosts = built.hosts();
	let site = TrainingSite::of_hosts(&hosts, arena.center, arena.footprint);
	let arena = arena.with_roster(&site);
	info!(
		target: "world.training",
		"stamped {kind:?} at site {} (seed {:016x}): {} brawlers in {} squads around {} POIs",
		round.site(),
		round.seed,
		arena.brawlers(),
		arena.mobs.len(),
		site.pois.len(),
	);
	let cell_id = Id::from_cell(filled.cell);
	let terrain_ids = stamp_training_terrain(&mut commands, &store, &mut developments, &filled);
	for host in &hosts {
		for entity in host.spawn(&mut commands) {
			commands.entity(entity).insert(TrainingPlaza);
		}
	}
	spawn_training_wall(&mut commands, &store, &developments, &layout, &base, &arena, config.seed);
	if terrain_ids.is_empty() {
		warn!(target: "world.training", "no FinePatch cells overlapped the development pads");
	}
	commands.insert_resource(TrainingPlazaStamped { round: *round, cell_id, terrain_ids, arena });
}

/// Point the surface probe at a new round's site and move the player there
/// while the patch streams, so the camera, vegetation, and LOD follow it. A
/// body still being respawned is parked once it exists.
pub(crate) fn park_on_training_site(
	grounds: Res<TrainingGrounds>,
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
	if !grounds.0 {
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
	grounds: Res<TrainingGrounds>,
	round: Res<TrainingRound>,
	mounted: Option<Res<TrainingPlazaMounted>>,
	stamped: Option<Res<TrainingPlazaStamped>>,
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
	if !grounds.0 || !mounted.is_some_and(|mounted| mounted.serves(*round)) {
		return;
	}
	let Some(stamped) = stamped else {
		return;
	};
	let Ok(player) = unanchored.single() else {
		return;
	};
	let arena = &stamped.arena;
	seat_player_at(&mut players, &mut cameras, arena.player, arena.player_facing());
	commands.entity(player).insert(OffTerrainAnchor { translation: arena.player });
}

/// Once the padded fills carry colliders, every raw FinePatch cell they cover
/// is hidden and superseded, including cells Durham re-presents later. A raw
/// collider left under the courtyard is a second, unstamped floor.
pub(crate) fn supersede_training_raw_terrain(
	grounds: Res<TrainingGrounds>,
	stamped: Option<Res<TrainingPlazaStamped>>,
	ready_fills: Query<(), (With<TrainingPaddedFill>, With<TerrainTrimeshCollider>)>,
	mut raw: Query<(Entity, &PresentedTerrainScene, &mut Visibility), Without<TerrainSuperseded>>,
	mut commands: Commands,
) {
	if !grounds.0 {
		return;
	}
	let Some(stamped) = stamped else {
		return;
	};
	if !stamped.fills_ready(ready_fills.iter().count()) {
		return;
	}
	for (entity, presented, mut visibility) in &mut raw {
		if !stamped.terrain_ids.contains(&presented.0) {
			continue;
		}
		*visibility = Visibility::Hidden;
		// The Durham strip runs in its own set; physics must not step with both floors.
		// Durham may despawn the raw cell in the same frame.
		commands
			.entity(entity)
			.try_insert(TerrainSuperseded)
			.try_remove::<(Collider, RigidBody, TerrainTrimeshCollider)>();
	}
}

/// Seat the roster and player once the padded fills carry colliders.
pub(crate) fn promote_training_plaza(
	grounds: Res<TrainingGrounds>,
	stamped: Option<Res<TrainingPlazaStamped>>,
	mounted: Option<Res<TrainingPlazaMounted>>,
	ready_fills: Query<(), (With<TrainingPaddedFill>, With<TerrainTrimeshCollider>)>,
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
	if !stamped.fills_ready(ready_fills.iter().count()) {
		return;
	}
	let arena = &stamped.arena;
	spawn_xz.0 = Some(arena.player.xz());
	for (index, mob) in arena.mobs.iter().enumerate() {
		let host = mob
			.scene(stamped.round.mob_seed() + index as f32)
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
	commands.insert_resource(TrainingPlazaMounted(stamped.round));
}

/// Tear the plaza down when Training ends or the round moves to a new map. Leaving
/// parks the player on Discovery's default spawn, so Discovery resumes the
/// character's saved trail instead of starting at the last Training site.
pub(crate) fn clear_training_plaza(
	grounds: Res<TrainingGrounds>,
	round: Res<TrainingRound>,
	base: Res<WorldBaseTerrain>,
	mounted: Option<Res<TrainingPlazaMounted>>,
	stamped: Option<Res<TrainingPlazaStamped>>,
	mut developments: ResMut<DevelopmentEntryStore>,
	mut spawn_xz: ResMut<PlayerSpawnXz>,
	mut commands: Commands,
	fixtures: Query<Entity, Or<(With<TrainingPlaza>, With<TrainingBrawler>)>>,
	brawler_hosts: Query<(), With<TrainingBrawler>>,
	members: Query<(Entity, &MemberOf)>,
	anchored: Query<Entity, (With<Player>, With<OffTerrainAnchor>)>,
	mut superseded: Query<(Entity, &mut Visibility), With<TerrainSuperseded>>,
	mut players: Query<
		(&mut Transform, &mut GlobalTransform, Option<&mut Position>, Option<&mut LinearVelocity>),
		With<Player>,
	>,
	mut cameras: Query<
		(&mut Transform, &mut GlobalTransform, &FollowCamera),
		(With<Camera3d>, Without<Player>),
	>,
) {
	let stale = |of: TrainingRound| !grounds.0 || of.map() != round.map();
	let mounted_stale = mounted.as_deref().is_some_and(|mounted| stale(mounted.0));
	let stamped_stale = stamped.as_deref().is_some_and(|stamped| stale(stamped.round));
	if !mounted_stale && !stamped_stale {
		return;
	}
	// Leaving also resets Durham's layout, which despawns raw cells in the same
	// frame, and fixtures nest under one another; every teardown command must
	// tolerate a target already gone.
	for (entity, mut visibility) in &mut superseded {
		*visibility = Visibility::Inherited;
		commands.entity(entity).try_remove::<TerrainSuperseded>();
	}
	if let Some(stamped) = stamped.as_deref() {
		developments.remove_cell(stamped.cell_id);
	}
	// Respawned members are not tied to a roster stub, so dropping the host
	// alone would strand them.
	for (entity, member) in &members {
		if brawler_hosts.contains(member.mob) {
			commands.entity(entity).try_despawn();
		}
	}
	for entity in &fixtures {
		commands.entity(entity).try_despawn();
	}
	for player in &anchored {
		commands.entity(player).try_remove::<OffTerrainAnchor>();
	}
	commands.remove_resource::<TrainingPlazaStamped>();
	commands.remove_resource::<TrainingPlazaMounted>();
	if grounds.0 {
		return;
	}
	spawn_xz.0 = None;
	let home = Vec2::ZERO;
	let at = player_spawn_point_at(home, holding_elevation(&base.0, home.x, home.y));
	seat_player_at(&mut players, &mut cameras, at, Vec3::Z);
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

	fn mob_seed() -> f32 {
		TrainingRound::default().mob_seed()
	}

	fn base_terrain() -> WorldBaseTerrain {
		use durham_terrain_models::{BaseTerrainNoise, TerrainConfig};
		WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(42)))
	}

	#[test]
	fn training_cell_is_centered_on_the_site() {
		for center in [Vec2::ZERO, Vec2::new(-4_800.0, 1_120.0)] {
			let cell = training_development_cell(center);
			assert!(((cell.min.x + cell.max.x) * 0.5 - center.x).abs() < 1e-3);
			assert!(((cell.min.z + cell.max.z) * 0.5 - center.y).abs() < 1e-3);
			assert!((cell.max.x - cell.min.x - DEVELOPMENT_CELL_SIZE).abs() < 1e-3);
		}
	}

	#[test]
	fn round_sites_pick_filled_kinds() {
		for seed in 0..8 {
			let round = TrainingRound::new(seed);
			let cell = training_development_cell(round.layout().region_center_xz().xz());
			let config = DevelopmentConfig::from_world_seed(round.development_seed());
			assert_ne!(DevelopmentKind::pick_filled(cell, &config), DevelopmentKind::Empty);
		}
	}

	fn member_feet(arena: &TrainingArena) -> impl Iterator<Item = Vec3> + '_ {
		arena
			.mobs
			.iter()
			.flat_map(|mob| mob.members.iter().map(|xz| Vec3::new(xz.x, arena.plaza_y, xz.y)))
	}

	fn seated(center: Vec2, footprint: Vec2, plaza_y: f32) -> TrainingArena {
		TrainingArena::around(center, footprint, plaza_y)
			.with_roster(&TrainingSite::single(center, footprint))
	}

	/// Four huts spread over a 60 m pad, each with its own POI.
	fn hamlet() -> (Vec2, TrainingSite) {
		let footprint = Vec2::splat(30.0);
		let mut site = TrainingSite::default();
		for at in [(-18.0, -18.0), (18.0, -18.0), (-18.0, 18.0), (18.0, 18.0)].map(Vec2::from) {
			let rect = (at - Vec2::splat(5.0), at + Vec2::splat(5.0));
			site.obstacles.push(rect);
			site.add_poi(at, rect);
		}
		(footprint, site)
	}

	fn nearest(from: Vec2, members: impl Iterator<Item = Vec2>) -> f32 {
		members.map(|at| at.distance(from)).fold(f32::INFINITY, f32::min)
	}

	#[test]
	fn everyone_spawns_inside_the_wall_and_outside_the_building() {
		let footprint = Vec2::new(36.0, 28.0);
		let arena = seated(Vec2::ZERO, footprint, 10.0);
		assert_eq!(arena.brawlers(), TRAINING_ROSTER);
		for at in std::iter::once(arena.player)
			.chain(member_feet(&arena))
			.chain(arena.mobs.iter().map(|mob| mob.host))
		{
			assert!(inside(&arena, at), "{at} is outside the wall");
			assert!(outside_footprint(Vec2::ZERO, footprint, at), "{at} is inside the building");
		}
	}

	#[test]
	fn one_poi_seats_two_squads_on_opposite_sides() {
		let arena = seated(Vec2::ZERO, Vec2::splat(36.0), 0.0);
		assert_eq!(arena.mobs.len(), TRAINING_MIN_SQUADS);
		let (a, b) = (arena.mobs[0].host.xz(), arena.mobs[1].host.xz());
		assert!(a.normalize().dot(b.normalize()) < -0.5, "squads at {a} and {b}");
	}

	#[test]
	fn hamlet_seats_a_squad_beside_every_poi() {
		let (footprint, site) = hamlet();
		let arena = TrainingArena::around(Vec2::ZERO, footprint, 0.0).with_roster(&site);
		assert_eq!(arena.mobs.len(), TRAINING_MAX_SQUADS);
		assert!((12..=16).contains(&arena.brawlers()));
		for (mob, (poi, (min, max))) in arena.mobs.iter().zip(&site.pois) {
			let spread = TrainingMob::spread(mob.members.len());
			let far = TRAINING_POI_STANDOFF_M
				+ TRAINING_OBSTACLE_MARGIN_M
				+ spread + TRAINING_RING_STEPS as f32 * TRAINING_RING_STEP_M;
			let (mid, extent) = ((*min + *max) * 0.5, (*max - *min) * 0.5);
			let off = ((mob.host.xz() - mid).abs() - extent).max(Vec2::ZERO);
			assert!(off.length() <= far, "squad {} is {off} off POI {poi}'s building", mob.host);
			for at in &mob.members {
				assert!(arena.open(&site, *at), "{at} is in a building or the wall");
			}
		}
	}

	#[test]
	fn squads_keep_apart_and_the_player_is_at_close_quarters() {
		let (footprint, site) = hamlet();
		for arena in [
			seated(Vec2::ZERO, Vec2::new(36.0, 28.0), 0.0),
			TrainingArena::around(Vec2::ZERO, footprint, 0.0).with_roster(&site),
		] {
			for (i, a) in arena.mobs.iter().enumerate() {
				for b in &arena.mobs[i + 1..] {
					let gap = a.members.iter().map(|at| nearest(*at, b.members.iter().copied()));
					let gap = gap.fold(f32::INFINITY, f32::min);
					assert!(gap >= TRAINING_SQUAD_GAP_M - 1e-3, "squads overlap: {gap}");
				}
			}
			let player = arena.player.xz();
			let all = arena.mobs.iter().flat_map(|mob| mob.members.iter().copied());
			let closest = nearest(player, all);
			assert!(
				(TRAINING_PLAYER_CLEARANCE_M - 1e-3..=20.0).contains(&closest),
				"nearest brawler {closest}"
			);
			let first = nearest(player, arena.mobs[0].members.iter().copied());
			assert!(first <= 20.0, "first squad {first} m away");
			let toward = (arena.mobs[0].host.xz() - player).normalize();
			assert!(arena.player_facing().xz().dot(toward) > 0.99, "player should face squad 0");
		}
	}

	#[test]
	fn scene_pads_a_squad_past_the_rolled_roster() {
		let seat = Vec2::new(40.0, 0.0);
		let members = TrainingMob::sunflower(seat, 14);
		let mob = TrainingMob { host: Vec3::new(seat.x, 0.0, seat.y), members };
		let scene = mob.scene(mob_seed());
		assert_eq!(scene.mob.roster.members.len(), 14);
		assert!(scene
			.mob
			.roster
			.members
			.iter()
			.all(|member| TRAINING_SPECIES.contains(&member.character.species)));
	}

	#[test]
	fn mob_scene_seats_members_where_the_arena_planned() {
		let arena = seated(Vec2::ZERO, Vec2::new(30.0, 30.0), 4.0);
		let mob = &arena.mobs[0];
		let scene = mob.scene(mob_seed());
		assert_eq!(scene.mob.kind, MobKind::Brawler);
		assert_eq!(scene.mob.roster.members.len(), mob.members.len());
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
		let scene = seated(Vec2::ZERO, Vec2::splat(30.0), 0.0).mobs[0].scene(mob_seed());
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
		let arena = seated(Vec2::ZERO, Vec2::splat(120.0), 0.0);
		let reach = arena.courtyard_half() + Vec2::splat(TRAINING_COURTYARD_EASE_M);
		assert!(reach.max_element() <= 160.0, "courtyard reach {reach}");
		assert_eq!(arena.brawlers(), TRAINING_ROSTER);
		for at in member_feet(&arena) {
			assert!(inside(&arena, at));
		}
	}

	#[test]
	fn empty_cell_has_no_pad_influence() {
		let empty = DevelopmentCell::empty(training_development_cell(Vec2::ZERO));
		assert!(pad_influence_region(&empty).is_none());
	}

	#[test]
	fn les_halles_courtyard_covers_the_wall() -> Result<(), String> {
		let cell = training_development_cell(Vec2::ZERO);
		let config = DevelopmentConfig::from_world_seed(42);
		let filled = DevelopmentCell::with_les_halles(cell, 20.0, &config);
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
	fn les_halles_squads_stand_clear_of_its_buildings() -> Result<(), String> {
		let cell = training_development_cell(Vec2::ZERO);
		let config = DevelopmentConfig::from_world_seed(42);
		let filled = DevelopmentCell::with_les_halles(cell, 20.0, &config);
		let footprint = filled.footprint_half_extents().ok_or("footprint")?;
		let built = filled.built(config.seed as i32).ok_or("built")?;
		let arena = TrainingArena::around(Vec2::ZERO, footprint, 20.0);
		let site = TrainingSite::of_hosts(&built.hosts(), arena.center, arena.footprint);
		if site.pois.is_empty() {
			return Err("Les Halles exposes no POI".into());
		}
		let arena = arena.with_roster(&site);
		if arena.brawlers() != TRAINING_ROSTER {
			return Err(format!("{} brawlers", arena.brawlers()));
		}
		for at in arena.mobs.iter().flat_map(|mob| mob.members.iter()).chain([&arena.player.xz()]) {
			if !arena.open(&site, *at) {
				return Err(format!("{at} is inside a building or the wall"));
			}
		}
		Ok(())
	}

	#[test]
	fn raw_cells_under_the_courtyard_stop_colliding_once_the_pads_do()
	-> Result<(), bevy::ecs::system::RunSystemError> {
		use bevy::ecs::system::RunSystemOnce;
		let covered = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
		let elsewhere = Id::from_cell(Aabb3d::from_min_max(Vec3::splat(500.0), Vec3::splat(501.0)));
		let mut world = World::new();
		world.insert_resource(TrainingGrounds(true));
		world.insert_resource(TrainingPlazaStamped {
			round: TrainingRound::default(),
			cell_id: covered,
			terrain_ids: vec![covered],
			arena: TrainingArena::around(Vec2::ZERO, Vec2::splat(30.0), 0.0),
		});
		let raw = world.spawn((PresentedTerrainScene(covered), Visibility::Inherited)).id();
		let other = world.spawn((PresentedTerrainScene(elsewhere), Visibility::Inherited)).id();
		let fill = world.spawn(TrainingPaddedFill).id();

		world.run_system_once(supersede_training_raw_terrain)?;
		assert!(world.get::<TerrainSuperseded>(raw).is_none(), "raw floors until pads cook");

		world.entity_mut(fill).insert(TerrainTrimeshCollider);
		world.run_system_once(supersede_training_raw_terrain)?;
		assert!(world.get::<TerrainSuperseded>(raw).is_some());
		assert_eq!(world.get::<Visibility>(raw), Some(&Visibility::Hidden));
		assert!(world.get::<TerrainSuperseded>(other).is_none());

		let respawned = world.spawn((PresentedTerrainScene(covered), Visibility::Inherited)).id();
		world.run_system_once(supersede_training_raw_terrain)?;
		assert!(world.get::<TerrainSuperseded>(respawned).is_some(), "re-presented raw cells too");
		Ok(())
	}

	fn plaza_world(grounds: bool, round: TrainingRound) -> World {
		let mut world = World::new();
		world.insert_resource(TrainingGrounds(grounds));
		world.insert_resource(round);
		world.insert_resource(base_terrain());
		world.insert_resource(DevelopmentEntryStore::default());
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
	fn a_new_round_tears_the_plaza_down_in_place() -> Result<(), bevy::ecs::system::RunSystemError>
	{
		use bevy::ecs::system::RunSystemOnce;
		let round = TrainingRound::new(8);
		let mut world = plaza_world(true, round.next());
		world.insert_resource(TrainingPlazaMounted(round));
		let seat = Vec3::new(900.0, 30.0, -400.0);
		let player = seated_player(&mut world, seat);
		let host = world.spawn(TrainingBrawler).id();
		world.run_system_once(clear_training_plaza)?;
		assert!(world.get_resource::<TrainingPlazaMounted>().is_none());
		assert!(world.get_entity(host).is_err());
		assert!(world.get::<OffTerrainAnchor>(player).is_none());
		assert_eq!(world.get::<Transform>(player).map(|t| t.translation), Some(seat));
		assert_eq!(world.resource::<PlayerSpawnXz>().0, Some(Vec2::ONE));

		world.insert_resource(TrainingPlazaMounted(round.next()));
		let host = world.spawn(TrainingBrawler).id();
		world.run_system_once(clear_training_plaza)?;
		assert!(world.get_entity(host).is_ok(), "the live round keeps its plaza");

		world.insert_resource(round.next().next_life());
		world.run_system_once(clear_training_plaza)?;
		assert!(world.get_entity(host).is_ok(), "a new life keeps the map's plaza");
		Ok(())
	}

	#[test]
	fn a_new_life_takes_the_arena_seat() -> Result<(), bevy::ecs::system::RunSystemError> {
		use bevy::ecs::system::RunSystemOnce;
		let round = TrainingRound::new(13);
		let mut world = plaza_world(true, round.next_life());
		let arena = TrainingArena::around(Vec2::new(40.0, -20.0), Vec2::splat(30.0), 6.0)
			.with_roster(&TrainingSite::single(Vec2::new(40.0, -20.0), Vec2::splat(30.0)));
		let seat = arena.player;
		world.insert_resource(TrainingPlazaMounted(round));
		world.insert_resource(TrainingPlazaStamped {
			round,
			cell_id: Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE)),
			terrain_ids: Vec::new(),
			arena,
		});
		let transform = Transform::from_translation(Vec3::new(0.0, 90.0, 0.0));
		let body = world.spawn((Player, transform, GlobalTransform::from(transform))).id();
		world.run_system_once(reseat_training_life)?;
		assert_eq!(world.get::<Transform>(body).map(|t| t.translation), Some(seat));
		assert_eq!(world.get::<OffTerrainAnchor>(body).map(|a| a.translation), Some(seat));

		let moved = Vec3::new(1.0, 2.0, 3.0);
		if let Some(mut at) = world.get_mut::<Transform>(body) {
			at.translation = moved;
		}
		world.run_system_once(reseat_training_life)?;
		assert_eq!(world.get::<Transform>(body).map(|t| t.translation), Some(moved));

		world.entity_mut(body).remove::<OffTerrainAnchor>();
		world.insert_resource(round.next());
		world.run_system_once(reseat_training_life)?;
		assert_eq!(
			world.get::<Transform>(body).map(|t| t.translation),
			Some(moved),
			"a new map seats through its own promote"
		);
		Ok(())
	}

	#[test]
	fn a_new_round_parks_the_player_on_its_site() -> Result<(), bevy::ecs::system::RunSystemError>
	{
		use bevy::ecs::system::RunSystemOnce;
		let round = TrainingRound::new(21);
		let mut world = plaza_world(true, round);
		let player = seated_player(&mut world, Vec3::ZERO);
		world.run_system_once(park_on_training_site)?;
		let center = round.layout().region_center_xz().xz();
		assert_eq!(world.resource::<PlayerSpawnXz>().0, Some(center));
		let at = world.get::<Transform>(player).map(|t| t.translation.xz());
		assert!(at.is_some_and(|at| at.distance(center) < 1e-3), "{at:?} vs {center}");
		Ok(())
	}

	#[test]
	fn leaving_training_drops_the_seat_anchor() -> Result<(), bevy::ecs::system::RunSystemError> {
		use bevy::ecs::system::RunSystemOnce;
		let round = TrainingRound::default();
		let mut world = plaza_world(false, round);
		world.insert_resource(TrainingPlazaMounted(round));
		let player = seated_player(&mut world, Vec3::new(4_000.0, 12.0, -2_000.0));
		let host = world.spawn(TrainingBrawler).id();
		let member = world.spawn(MemberOf { mob: host, slot: 0 }).id();
		let stranger_host = world.spawn_empty().id();
		let stranger = world.spawn(MemberOf { mob: stranger_host, slot: 0 }).id();
		let raw = world.spawn((TerrainSuperseded, Visibility::Hidden)).id();
		world.run_system_once(clear_training_plaza)?;
		assert!(world.get::<TerrainSuperseded>(raw).is_none(), "raw cells get their floor back");
		assert_eq!(world.get::<Visibility>(raw), Some(&Visibility::Inherited));
		assert!(world.get::<OffTerrainAnchor>(player).is_none());
		assert_eq!(world.resource::<PlayerSpawnXz>().0, None);
		let home = world.get::<Transform>(player).map(|t| t.translation.xz());
		assert_eq!(home, Some(Vec2::ZERO), "Discovery resumes from its default spawn");
		assert!(world.get_entity(host).is_err());
		assert!(world.get_entity(member).is_err(), "respawned members must leave with the mob");
		assert!(world.get_entity(stranger).is_ok());
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
		let host = world.spawn((TrainingBrawler, DoomedThisFrame)).id();
		let member = world.spawn((MemberOf { mob: host, slot: 0 }, ChildOf(host))).id();
		let raw = world.spawn((TerrainSuperseded, Visibility::Hidden, DoomedThisFrame)).id();

		// Both systems see the targets alive; the despawns apply first.
		let mut schedule = Schedule::default();
		schedule.add_systems((despawn_doomed, clear_training_plaza).chain_ignore_deferred());
		schedule.run(&mut world);

		for entity in [player, host, member, raw] {
			assert!(world.get_entity(entity).is_err());
		}
		assert!(world.get_resource::<TrainingPlazaMounted>().is_none());
		Ok(())
	}

	#[test]
	fn supersede_stands_down_once_training_ends() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;
		let covered = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
		let mut world = World::new();
		world.insert_resource(TrainingGrounds(false));
		world.insert_resource(TrainingPlazaStamped {
			round: TrainingRound::default(),
			cell_id: covered,
			terrain_ids: vec![covered],
			arena: TrainingArena::around(Vec2::ZERO, Vec2::splat(30.0), 0.0),
		});
		world.spawn((TrainingPaddedFill, TerrainTrimeshCollider));
		let raw = world.spawn((PresentedTerrainScene(covered), Visibility::Inherited)).id();
		world
			.run_system_once(supersede_training_raw_terrain)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get::<TerrainSuperseded>(raw).is_none());
		Ok(())
	}
}
