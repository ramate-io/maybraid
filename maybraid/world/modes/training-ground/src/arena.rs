//! Courtyard seats for the Training roster. Published once the stamp's pads
//! can bear weight; the mob scheme only converts the squads.

use barking::{GroupKind, MobCell, MobCellExtent, MobEnvironmentSample, MobGroup, PlacedMob};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use characters::LocomotionCapsule;
use durham::TerrainTrimeshCollider;
use durham::{Durham, HcsgStorage};
use layer_stack::{ActiveGenerationMode, GenerationModeSystems};
use mob_characters::CharacterSpecies;
use mob_scenes::{Mob, MobKind, MobScene};
use richmond::{Built, DevelopmentHost, DevelopmentHosts, PresentedPaddedTerrainScene};
use terrain_layer_model::OnTerrain;

use crate::{TrainingGround, TrainingMap, TrainingPlazaStamped};

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

/// Player seat, facing, and squads. Published when the courtyard can bear weight.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct TrainingArena {
	map: TrainingMap,
	seed: f32,
	center: Vec2,
	half: Vec2,
	footprint: Vec2,
	band: Vec2,
	plaza_y: f32,
	pub player: Vec3,
	pub facing: Vec3,
	mobs: Vec<TrainingMob>,
}

impl TrainingArena {
	/// Seat only. Plaza tests that do not need squads.
	pub fn at_seat(player: Vec3, facing: Vec3) -> Self {
		Self {
			map: crate::TrainingRound::default().map(),
			seed: 0.0,
			center: player.xz(),
			half: Vec2::ONE,
			footprint: Vec2::ONE,
			band: Vec2::ONE,
			plaza_y: player.y,
			player,
			facing,
			mobs: Vec::new(),
		}
	}

	pub fn map(&self) -> TrainingMap {
		self.map
	}

	pub(crate) fn xz_bounds(&self) -> (Vec2, Vec2) {
		(self.center - self.half, self.center + self.half)
	}

	pub(crate) fn group_extent(&self) -> f32 {
		self.half.max_element() * 2.0
	}

	fn around(center: Vec2, footprint: Vec2, plaza_y: f32) -> Self {
		let half = (footprint + Vec2::splat(crate::TRAINING_ARENA_MARGIN_M))
			.min(Vec2::splat(crate::TRAINING_ARENA_MAX_HALF_M));
		let footprint = footprint.min(half);
		let band = (footprint + half) * 0.5;
		let player = center + Self::midline_point(band, 0.0);
		let player =
			Vec3::new(player.x, plaza_y + LocomotionCapsule::HUMANOID.spawn_height(), player.y);
		Self {
			map: crate::TrainingRound::default().map(),
			seed: 0.0,
			center,
			half,
			footprint,
			band,
			plaza_y,
			player,
			facing: Vec3::Z,
			mobs: Vec::new(),
		}
	}

	fn from_stamp(stamped: &TrainingPlazaStamped, site: &TrainingSite) -> Self {
		let mut arena = Self::around(stamped.center(), stamped.footprint(), stamped.plaza_y())
			.with_roster(site);
		arena.map = stamped.round().map();
		arena.seed = stamped.round().mob_seed();
		arena
	}

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
			self.mobs
				.push(TrainingMob { host: Vec3::new(seat.x, self.plaza_y, seat.y), members });
			if player.is_none() {
				player = Some(self.seat_player(site, poi));
			}
		}
		if let Some(at) = player {
			self.player =
				Vec3::new(at.x, self.plaza_y + LocomotionCapsule::HUMANOID.spawn_height(), at.y);
		}
		self.facing = self.player_facing();
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

	fn rings(center: Vec2, radius: f32, bearing: f32) -> impl Iterator<Item = Vec2> {
		(0..TRAINING_RING_STEPS).flat_map(move |ring| {
			let radius = radius + ring as f32 * TRAINING_RING_STEP_M;
			Self::fan(bearing).map(move |dir| center + dir * radius)
		})
	}

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

	fn player_facing(&self) -> Vec3 {
		let target = self.mobs.first().map_or(self.center, |mob| mob.host.xz());
		let toward = Vec3::new(target.x - self.player.x, 0.0, target.y - self.player.z);
		toward.try_normalize().unwrap_or(Vec3::Z)
	}

	#[cfg(test)]
	fn brawlers(&self) -> usize {
		self.mobs.iter().map(|mob| mob.members.len()).sum()
	}

	pub(crate) fn mob_cell(&self) -> MobCell {
		let (min, max) = self.xz_bounds();
		let extent =
			MobCellExtent::from_bounds(Vec3::new(min.x, 0.0, min.y), Vec3::new(max.x, 1.0, max.y));
		let extent_m = self.group_extent();
		let groups = self
			.mobs
			.iter()
			.enumerate()
			.map(|(index, mob)| MobGroup {
				kind: GroupKind::Placed,
				seed: self.seed.to_bits() as u64 ^ index as u64,
				origin: mob.host.xz(),
				extent: extent_m,
				mobs: vec![PlacedMob {
					scene: mob.scene(self.seed + index as f32),
					transform: Transform::from_translation(mob.host),
					environment: MobEnvironmentSample::default(),
				}],
			})
			.collect();
		MobCell { extent, groups }
	}
}

/// Development building rects and POI anchors, in world XZ.
#[derive(Clone, Debug, Default, PartialEq)]
struct TrainingSite {
	obstacles: Vec<(Vec2, Vec2)>,
	pois: Vec<(Vec2, (Vec2, Vec2))>,
}

impl TrainingSite {
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
	host: Vec3,
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

pub(crate) fn install(app: &mut App) {
	app.add_systems(
		Update,
		publish_training_arena
			.in_set(GenerationModeSystems::<TrainingGround>::default())
			.before(lod::LodGenerateSystems::Produce)
			.before(lod::LodPresentSystems::Produce),
	);
	app.add_systems(OnExit(ActiveGenerationMode::of::<TrainingGround>()), clear_training_arena);
}

fn clear_training_arena(mut commands: Commands) {
	commands.remove_resource::<TrainingArena>();
}

pub(crate) fn publish_training_arena(
	stamped: Option<Res<TrainingPlazaStamped>>,
	arena: Option<Res<TrainingArena>>,
	storage: Res<HcsgStorage>,
	ready_pads: Query<&PresentedPaddedTerrainScene, With<TerrainTrimeshCollider>>,
	mut commands: Commands,
) {
	let Some(stamped) = stamped.as_deref() else {
		if arena.is_some() {
			commands.remove_resource::<TrainingArena>();
		}
		return;
	};
	if arena.as_deref().is_some_and(|arena| arena.map == stamped.round().map()) {
		return;
	}
	let Some(built) = storage.get::<Built<OnTerrain<Durham>>>(stamped.cell_id()) else {
		if arena.is_some() {
			commands.remove_resource::<TrainingArena>();
		}
		return;
	};
	let cooked = ready_pads
		.iter()
		.filter(|scene| stamped.terrain_ids().contains(&scene.0))
		.count();
	if !stamped.fills_ready(cooked) {
		if arena.is_some() {
			commands.remove_resource::<TrainingArena>();
		}
		return;
	}
	let hosts = built.development.hosts();
	let site = TrainingSite::of_hosts(&hosts, stamped.center(), stamped.footprint());
	commands.insert_resource(TrainingArena::from_stamp(stamped, &site));
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::World;
	use lod::gen::Id;
	use threat_intelligence::ThreatId;

	use crate::urbanization::{author_for_test, les_halles_for_test, TrainingDevelopment};
	use crate::{training_development_cell, TrainingRound, TRAINING_COURTYARD_EASE_M};
	use mob_scenes::player_affiliations;

	fn inside(arena: &TrainingArena, at: Vec3) -> bool {
		let (min, max) = (arena.center - arena.half, arena.center + arena.half);
		at.x > min.x && at.x < max.x && at.z > min.y && at.z < max.y
	}

	fn outside_footprint(center: Vec2, footprint: Vec2, at: Vec3) -> bool {
		let d = (at.xz() - center).abs();
		d.x > footprint.x || d.y > footprint.y
	}

	fn mob_seed() -> f32 {
		TrainingRound::default().mob_seed()
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
	fn everyone_spawns_inside_the_wall_and_outside_the_building() -> anyhow::Result<()> {
		let footprint = Vec2::new(36.0, 28.0);
		let arena = seated(Vec2::ZERO, footprint, 10.0);
		anyhow::ensure!(arena.brawlers() == TRAINING_ROSTER);
		for at in std::iter::once(arena.player)
			.chain(member_feet(&arena))
			.chain(arena.mobs.iter().map(|mob| mob.host))
		{
			anyhow::ensure!(inside(&arena, at), "{at} is outside the wall");
			anyhow::ensure!(
				outside_footprint(Vec2::ZERO, footprint, at),
				"{at} is inside the building"
			);
		}
		Ok(())
	}

	#[test]
	fn one_poi_seats_two_squads_on_opposite_sides() -> anyhow::Result<()> {
		let arena = seated(Vec2::ZERO, Vec2::splat(36.0), 0.0);
		anyhow::ensure!(arena.mobs.len() == TRAINING_MIN_SQUADS);
		let (a, b) = (arena.mobs[0].host.xz(), arena.mobs[1].host.xz());
		anyhow::ensure!(a.normalize().dot(b.normalize()) < -0.5, "squads at {a} and {b}");
		Ok(())
	}

	#[test]
	fn hamlet_seats_a_squad_beside_every_poi() -> anyhow::Result<()> {
		let (footprint, site) = hamlet();
		let arena = TrainingArena::around(Vec2::ZERO, footprint, 0.0).with_roster(&site);
		anyhow::ensure!(arena.mobs.len() == TRAINING_MAX_SQUADS);
		anyhow::ensure!((12..=16).contains(&arena.brawlers()));
		for (mob, (poi, (min, max))) in arena.mobs.iter().zip(&site.pois) {
			let spread = TrainingMob::spread(mob.members.len());
			let far = TRAINING_POI_STANDOFF_M
				+ TRAINING_OBSTACLE_MARGIN_M
				+ spread + TRAINING_RING_STEPS as f32 * TRAINING_RING_STEP_M;
			let (mid, extent) = ((*min + *max) * 0.5, (*max - *min) * 0.5);
			let off = ((mob.host.xz() - mid).abs() - extent).max(Vec2::ZERO);
			anyhow::ensure!(
				off.length() <= far,
				"squad {} is {off} off POI {poi}'s building",
				mob.host
			);
			for at in &mob.members {
				anyhow::ensure!(arena.open(&site, *at), "{at} is in a building or the wall");
			}
		}
		Ok(())
	}

	#[test]
	fn squads_keep_apart_and_the_player_is_at_close_quarters() -> anyhow::Result<()> {
		let (footprint, site) = hamlet();
		for arena in [
			seated(Vec2::ZERO, Vec2::new(36.0, 28.0), 0.0),
			TrainingArena::around(Vec2::ZERO, footprint, 0.0).with_roster(&site),
		] {
			for (i, a) in arena.mobs.iter().enumerate() {
				for b in &arena.mobs[i + 1..] {
					let gap = a.members.iter().map(|at| nearest(*at, b.members.iter().copied()));
					let gap = gap.fold(f32::INFINITY, f32::min);
					anyhow::ensure!(gap >= TRAINING_SQUAD_GAP_M - 1e-3, "squads overlap: {gap}");
				}
			}
			let player = arena.player.xz();
			let all = arena.mobs.iter().flat_map(|mob| mob.members.iter().copied());
			let closest = nearest(player, all);
			anyhow::ensure!(
				(TRAINING_PLAYER_CLEARANCE_M - 1e-3..=20.0).contains(&closest),
				"nearest brawler {closest}"
			);
			let first = nearest(player, arena.mobs[0].members.iter().copied());
			anyhow::ensure!(first <= 20.0, "first squad {first} m away");
			let toward = (arena.mobs[0].host.xz() - player).normalize();
			anyhow::ensure!(
				arena.player_facing().xz().dot(toward) > 0.99,
				"player should face squad 0"
			);
		}
		Ok(())
	}

	#[test]
	fn scene_pads_a_squad_past_the_rolled_roster() -> anyhow::Result<()> {
		let seat = Vec2::new(40.0, 0.0);
		let members = TrainingMob::sunflower(seat, 14);
		let mob = TrainingMob { host: Vec3::new(seat.x, 0.0, seat.y), members };
		let scene = mob.scene(mob_seed());
		anyhow::ensure!(scene.mob.roster.members.len() == 14);
		anyhow::ensure!(scene
			.mob
			.roster
			.members
			.iter()
			.all(|member| TRAINING_SPECIES.contains(&member.character.species)));
		Ok(())
	}

	#[test]
	fn mob_scene_seats_members_where_the_arena_planned() -> anyhow::Result<()> {
		let arena = seated(Vec2::ZERO, Vec2::new(30.0, 30.0), 4.0);
		let mob = &arena.mobs[0];
		let scene = mob.scene(mob_seed());
		anyhow::ensure!(scene.mob.kind == MobKind::Brawler);
		anyhow::ensure!(scene.mob.roster.members.len() == mob.members.len());
		for (member, xz) in scene.mob.roster.members.iter().zip(&mob.members) {
			let feet = mob.host.xz() + member.offset.xz();
			anyhow::ensure!(feet.distance(*xz) < 1e-3, "{feet} vs {xz}");
			anyhow::ensure!(member.offset.y > 0.0);
			anyhow::ensure!(member.character.armed());
		}
		Ok(())
	}

	#[test]
	fn brawler_mobs_fight_each_other_and_the_player() -> anyhow::Result<()> {
		let scene = seated(Vec2::ZERO, Vec2::splat(30.0), 0.0).mobs[0].scene(mob_seed());
		let pack = &scene.mob.intelligence.affiliations;
		let a = pack.for_member(ThreatId(1));
		let b = pack.for_member(ThreatId(2));
		let player = player_affiliations(ThreatId(3));
		anyhow::ensure!(a.threat_weight(&b, 0.0) >= 1.0, "same-mob brawlers must be FFA");
		anyhow::ensure!(a.threat_weight(&player, 0.0) >= 1.0);
		anyhow::ensure!(player.threat_weight(&a, 0.0) >= 1.0);
		Ok(())
	}

	#[test]
	fn arena_stays_on_the_fine_patch_for_wide_developments() -> anyhow::Result<()> {
		let arena = seated(Vec2::ZERO, Vec2::splat(120.0), 0.0);
		let reach = arena.half
			+ Vec2::splat(crate::TRAINING_COURTYARD_OVERHANG_M)
			+ Vec2::splat(TRAINING_COURTYARD_EASE_M);
		anyhow::ensure!(reach.max_element() <= 160.0, "courtyard reach {reach}");
		anyhow::ensure!(arena.brawlers() == TRAINING_ROSTER);
		for at in member_feet(&arena) {
			anyhow::ensure!(inside(&arena, at));
		}
		Ok(())
	}

	#[test]
	fn les_halles_squads_stand_clear_of_its_buildings() -> anyhow::Result<()> {
		let cell = training_development_cell(Vec2::ZERO);
		let mut storage = HcsgStorage::default();
		let id = author_for_test(&mut storage, les_halles_for_test(cell, 20.0, 42))?;
		let footprint = storage
			.get::<TrainingDevelopment>(id)
			.and_then(TrainingDevelopment::footprint_half_extents)
			.ok_or_else(|| anyhow::anyhow!("footprint"))?;
		let built = storage
			.get::<Built<OnTerrain<Durham>>>(id)
			.ok_or_else(|| anyhow::anyhow!("built"))?;
		let arena = TrainingArena::around(Vec2::ZERO, footprint, 20.0);
		let site =
			TrainingSite::of_hosts(&built.development.hosts(), arena.center, arena.footprint);
		anyhow::ensure!(!site.pois.is_empty(), "Les Halles exposes no POI");
		let arena = arena.with_roster(&site);
		anyhow::ensure!(arena.brawlers() == TRAINING_ROSTER, "{} brawlers", arena.brawlers());
		for at in arena.mobs.iter().flat_map(|mob| mob.members.iter()).chain([&arena.player.xz()]) {
			anyhow::ensure!(arena.open(&site, *at), "{at} is inside a building or the wall");
		}
		Ok(())
	}

	fn stamp_for(
		round: TrainingRound,
		center: Vec2,
		footprint: Vec2,
		terrain_ids: Vec<Id>,
	) -> TrainingPlazaStamped {
		let cell = training_development_cell(center);
		TrainingPlazaStamped::new(
			round,
			Id::from_cell(cell),
			terrain_ids,
			center,
			footprint + Vec2::splat(16.0),
			footprint,
			4.0,
		)
	}

	fn arena_world(
		round: TrainingRound,
		center: Vec2,
		footprint: Vec2,
		terrain_ids: Vec<Id>,
	) -> anyhow::Result<World> {
		let mut world = World::new();
		let cell = training_development_cell(center);
		let mut storage = HcsgStorage::default();
		author_for_test(&mut storage, les_halles_for_test(cell, 4.0, round.development_seed()))?;
		world.insert_resource(storage);
		world.insert_resource(stamp_for(round, center, footprint, terrain_ids));
		Ok(world)
	}

	fn expected_arena(world: &World) -> anyhow::Result<TrainingArena> {
		let stamped = world.resource::<TrainingPlazaStamped>();
		let built = world
			.resource::<HcsgStorage>()
			.get::<Built<OnTerrain<Durham>>>(stamped.cell_id())
			.ok_or_else(|| anyhow::anyhow!("built missing"))?;
		let site = TrainingSite::of_hosts(
			&built.development.hosts(),
			stamped.center(),
			stamped.footprint(),
		);
		Ok(TrainingArena::from_stamp(stamped, &site))
	}

	#[test]
	fn nothing_is_published_until_the_stamp_colliders_are_ready() -> anyhow::Result<()> {
		let pad = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
		let mut world =
			arena_world(TrainingRound::new(42), Vec2::ZERO, Vec2::splat(36.0), vec![pad])?;
		world
			.run_system_once(publish_training_arena)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(world.get_resource::<TrainingArena>().is_none());

		let pad_entity = world.spawn(PresentedPaddedTerrainScene(pad)).id();
		world
			.run_system_once(publish_training_arena)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(world.get_resource::<TrainingArena>().is_none());

		world.entity_mut(pad_entity).insert(TerrainTrimeshCollider);
		world
			.run_system_once(publish_training_arena)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let published = world
			.get_resource::<TrainingArena>()
			.ok_or_else(|| anyhow::anyhow!("arena was not published"))?
			.clone();
		let expected = expected_arena(&world)?;
		anyhow::ensure!(published.player == expected.player);
		anyhow::ensure!(published.facing == expected.facing);
		anyhow::ensure!(published.mobs == expected.mobs);
		Ok(())
	}

	#[test]
	fn a_new_map_drops_the_arena_and_a_new_life_keeps_it() -> anyhow::Result<()> {
		let mut world =
			arena_world(TrainingRound::new(42), Vec2::ZERO, Vec2::splat(36.0), Vec::new())?;
		world
			.run_system_once(publish_training_arena)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let first = world
			.get_resource::<TrainingArena>()
			.ok_or_else(|| anyhow::anyhow!("arena was not published"))?
			.clone();
		world.insert_resource(stamp_for(
			TrainingRound::new(42).next_life(),
			Vec2::ZERO,
			Vec2::splat(36.0),
			Vec::new(),
		));
		world
			.run_system_once(publish_training_arena)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let kept = world
			.get_resource::<TrainingArena>()
			.ok_or_else(|| anyhow::anyhow!("a new life dropped the arena"))?;
		anyhow::ensure!(kept.player == first.player);
		anyhow::ensure!(kept.mobs == first.mobs);

		world.remove_resource::<TrainingPlazaStamped>();
		world
			.run_system_once(publish_training_arena)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(world.get_resource::<TrainingArena>().is_none());
		Ok(())
	}
}
