//! Plan packing for [`super::CommonBedroom`]: passage clearances, beds /
//! nightstands, then wall-seeded closets and ensuites via [`EnclosedRoom`].
//!
//! # Program tiers (prefer enclosures over fillers)
//!
//! Greedy packing samples a [`Concept`] each step, but concepts sit in tiers:
//!
//! 1. **Enclosure** — ensuite, walk-in, closet (private rooms with doors)
//! 2. **Appointed** — bed, nightstand, wardrobe, dresser
//! 3. **Filler** — [`Concept::BedroomFurniture`] (and free small boxes)
//!
//! Fillers are **gated** until an enclosure soft-goal is met (ensuite and/or at
//! least one closet-like room), and are capped. That is the same *prefer
//! structure, then residual fill* idea as commercial-stall first-fit catalogs,
//! expressed as in-loop gates rather than a separate pass.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec3;
use procedural_common::{
	aabb2_area, aabb3_to_plan, inflate_aabb2, intersects_aabb2, plan_to_aabb3, touches_aabb2,
	NoiseConfig, NoiseParams, PlanAxes,
};

use crate::fit::{Confines, FitError};
use crate::openings::{Opening, OpeningId};
use crate::paneling::Rectangle;
use crate::usage_areas::clearance::PassageClearance;
use crate::usage_areas::enclosed_room::{EnclosedRoom, EnclosedRoomMins, EnclosedRoomParams};

use super::parameterized::SCOPE;

const DOOR_WIDTH_MIN: f32 = 0.65;
const DOOR_WIDTH_MAX: f32 = 1.15;
const DOOR_HEIGHT_MIN: f32 = 1.8;
const DOOR_HEIGHT_MAX: f32 = 2.3;
const DOOR_HEADER_MIN: f32 = 0.2;
/// Plan pad kept between closet↔closet and closet↔ensuite (avoids thin wall slivers).
const PARTITION_SEP: f32 = 1.0;
/// Plan pad between free-standing storage (wardrobe↔dresser / same-kind pairs).
const STORAGE_SEP: f32 = 1.0;
/// Inward door keep-out depth for closet / walk-in / ensuite sales-face doors.
const PARTITION_DOOR_CLEARANCE: f32 = 1.25;
/// Extra plan pad around authored door clear bands (lateral breathing room).
const DOOR_CLEAR_PAD: f32 = 0.3;
/// Host floor area (m²) above which walk-ins / bedroom furniture may be considered.
const LARGE_ROOM_AREA: f32 = 70.0;
/// Soft cap on residual [`Concept::BedroomFurniture`] placements.
const MAX_BEDROOM_FURNITURE: usize = 1;

/// One closet or ensuite: enclosure panels + authored door + room AABB.
#[derive(Debug, Clone, PartialEq)]
pub struct BedroomPartition {
	pub bounds: Aabb3d,
	pub walls: Vec<Rectangle>,
	pub door_id: OpeningId,
	pub door: Opening,
	pub door_clear: Aabb2d,
}

/// Packed bedroom program inside host confines.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CommonBedroomPacked {
	pub beds: Vec<Aabb3d>,
	/// Small boxes placed adjacent to a bed.
	pub nightstands: Vec<Aabb3d>,
	/// Same scale as nightstands, but not bed-adjacent (chests / stools / …).
	pub small_bedroom_furniture: Vec<Aabb3d>,
	/// Free-standing tall storage (not inside a closet cell).
	pub wardrobes: Vec<Aabb3d>,
	/// Free-standing low storage.
	pub dressers: Vec<Aabb3d>,
	/// Mid-size free furniture for roomy hosts.
	pub bedroom_furniture: Vec<Aabb3d>,
	pub closets: Vec<BedroomPartition>,
	pub walk_in_closets: Vec<BedroomPartition>,
	pub ensuites: Vec<BedroomPartition>,
}

/// Runtime knobs used by [`CommonBedroomRegions::pack`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommonBedroomRegions {
	pub spaciousness: f32,
	pub occupancy: f32,
	pub bed_against_wall: bool,
	pub ensuite_area_target: f32,
	pub bedroom_area_reserve: f32,
	pub walk_in_area_target: f32,
	pub closet_along_t: f32,
	pub walk_in_along_t: f32,
	pub ensuite_along_t: f32,
	pub door_width: f32,
	pub door_along_t: f32,
	pub door_height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Concept {
	Bed,
	Nightstand,
	Wardrobe,
	Dresser,
	BedroomFurniture,
	Closet,
	WalkInCloset,
	Ensuite,
}

enum Placed {
	Solid(Aabb3d),
	Nightstand(Aabb3d),
	SmallFurniture(Aabb3d),
	Wardrobe(Aabb3d),
	Dresser(Aabb3d),
	BedroomFurniture(Aabb3d),
	Partition(BedroomPartition),
}

impl CommonBedroomRegions {
	pub fn pack(
		&self,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<CommonBedroomPacked, FitError> {
		let host3 = &confines.bounds;
		let host = aabb3_to_plan(host3, PlanAxes::XZ);
		let passage_faces = PassageClearance::collect_faces(confines, host);
		let mut clearances = PassageClearance::bands_std(host, &passage_faces);
		let room_area = aabb2_area(host).max(1e-4);
		let cfg = NoiseConfig::new(noise);

		let mut packed = CommonBedroomPacked::default();
		let mut occupied = 0.0_f32;

		if let Some(bed) = place_bed(
			host3,
			host,
			&clearances,
			&packed,
			&cfg,
			0,
			self.spaciousness,
			self.bed_against_wall,
		) {
			occupied += xz_area(&bed);
			clearances.push(aabb3_to_plan(&bed, PlanAxes::XZ));
			packed.beds.push(bed);
		} else {
			return Err(FitError::TooSmall {
				reason: "common bedroom bed",
			});
		}

		for step in 1..24u32 {
			if occupied / room_area >= self.occupancy {
				break;
			}
			let concept = pick_concept(
				&cfg,
				step,
				PickCtx {
					bed_count: packed.beds.len(),
					has_ensuite: !packed.ensuites.is_empty(),
					closet_count: packed.closets.len(),
					walk_in_count: packed.walk_in_closets.len(),
					wardrobe_count: packed.wardrobes.len(),
					dresser_count: packed.dressers.len(),
					bedroom_furniture_count: packed.bedroom_furniture.len(),
					room_area,
				},
			);
			let Some(placed) = self.try_place(concept, host3, host, &clearances, &packed, &cfg, step)
			else {
				continue;
			};
			let add = match &placed {
				Placed::Solid(a)
				| Placed::Nightstand(a)
				| Placed::SmallFurniture(a)
				| Placed::Wardrobe(a)
				| Placed::Dresser(a)
				| Placed::BedroomFurniture(a) => xz_area(a),
				Placed::Partition(p) => xz_area(&p.bounds),
			};
			if (occupied + add) / room_area > self.occupancy + 1e-3 {
				continue;
			}
			occupied += add;
			match placed {
				Placed::Solid(a) => {
					clearances.push(aabb3_to_plan(&a, PlanAxes::XZ));
					packed.beds.push(a);
				}
				Placed::Nightstand(a) => {
					clearances.push(aabb3_to_plan(&a, PlanAxes::XZ));
					packed.nightstands.push(a);
				}
				Placed::SmallFurniture(a) => {
					clearances.push(aabb3_to_plan(&a, PlanAxes::XZ));
					packed.small_bedroom_furniture.push(a);
				}
				Placed::Wardrobe(a) => {
					clearances.push(aabb3_to_plan(&a, PlanAxes::XZ));
					packed.wardrobes.push(a);
				}
				Placed::Dresser(a) => {
					clearances.push(aabb3_to_plan(&a, PlanAxes::XZ));
					packed.dressers.push(a);
				}
				Placed::BedroomFurniture(a) => {
					clearances.push(aabb3_to_plan(&a, PlanAxes::XZ));
					packed.bedroom_furniture.push(a);
				}
				Placed::Partition(part) => {
					clearances.push(aabb3_to_plan(&part.bounds, PlanAxes::XZ));
					clearances.push(inflate_aabb2(part.door_clear, DOOR_CLEAR_PAD));
					match concept {
						Concept::Closet => packed.closets.push(part),
						Concept::WalkInCloset => packed.walk_in_closets.push(part),
						Concept::Ensuite => packed.ensuites.push(part),
						_ => unreachable!("non-partition via Partition"),
					}
				}
			}
		}

		Ok(packed)
	}

	fn try_place(
		&self,
		concept: Concept,
		host3: &Aabb3d,
		host: Aabb2d,
		clearances: &[Aabb2d],
		packed: &CommonBedroomPacked,
		cfg: &NoiseConfig,
		salt: u32,
	) -> Option<Placed> {
		match concept {
			Concept::Bed => place_bed(
				host3,
				host,
				clearances,
				packed,
				cfg,
				salt,
				self.spaciousness,
				self.bed_against_wall,
			)
			.map(Placed::Solid),
			Concept::Nightstand => {
				place_small_box(host3, host, clearances, packed, cfg, salt, self.spaciousness)
			}
			Concept::Wardrobe => {
				if !packed.wardrobes.is_empty() {
					return None;
				}
				let clears = storage_pack_clearances(clearances, packed);
				place_storage_long_on_wall(
					host3,
					host,
					&clears,
					packed,
					cfg,
					salt,
					base_wardrobe_extent(self.spaciousness),
				)
				.map(Placed::Wardrobe)
			}
			Concept::Dresser => {
				if !packed.dressers.is_empty() {
					return None;
				}
				let clears = storage_pack_clearances(clearances, packed);
				place_storage_long_on_wall(
					host3,
					host,
					&clears,
					packed,
					cfg,
					salt,
					base_dresser_extent(self.spaciousness),
				)
				.map(Placed::Dresser)
			}
			Concept::BedroomFurniture => {
				if packed.bedroom_furniture.len() >= MAX_BEDROOM_FURNITURE {
					return None;
				}
				place_free_extent(
					host3,
					host,
					clearances,
					packed,
					cfg,
					salt,
					base_bedroom_furniture_extent(self.spaciousness),
					42.0,
				)
				.map(Placed::BedroomFurniture)
			}
			Concept::Closet => {
				let clears = partition_pack_clearances(clearances, packed);
				let mins = self.closet_mins();
				let area_target = (mins.long * mins.short * 1.15).min(aabb2_area(host) * 0.18);
				self.pack_partition(
					host3,
					host,
					&clears,
					mins,
					area_target,
					self.bedroom_floor_reserve(host),
					self.closet_along_t,
					OpeningId::scoped(SCOPE, "closet_door", packed.closets.len().to_string()),
				)
				.map(Placed::Partition)
			}
			Concept::WalkInCloset => {
				if !packed.walk_in_closets.is_empty() {
					return None;
				}
				let clears = partition_pack_clearances(clearances, packed);
				let mins = self.walk_in_mins();
				self.pack_partition(
					host3,
					host,
					&clears,
					mins,
					self.walk_in_area_target.max(mins.long * mins.short),
					self.bedroom_floor_reserve(host),
					self.walk_in_along_t,
					OpeningId::scoped(SCOPE, "walk_in_door", "0"),
				)
				.map(Placed::Partition)
			}
			Concept::Ensuite => {
				if !packed.ensuites.is_empty() {
					return None;
				}
				let clears = partition_pack_clearances(clearances, packed);
				let mins = self.ensuite_mins();
				self.pack_partition(
					host3,
					host,
					&clears,
					mins,
					self.ensuite_area_target.max(mins.long * mins.short),
					self.bedroom_area_reserve
						.max(self.bedroom_floor_reserve(host)),
					self.ensuite_along_t,
					OpeningId::scoped(SCOPE, "ensuite_door", "0"),
				)
				.map(Placed::Partition)
			}
		}
	}

	fn closet_mins(&self) -> EnclosedRoomMins {
		EnclosedRoomMins {
			long: base_closet_length(self.spaciousness),
			short: base_closet_depth(self.spaciousness),
		}
	}

	fn walk_in_mins(&self) -> EnclosedRoomMins {
		EnclosedRoomMins {
			long: base_walk_in_length(self.spaciousness),
			short: base_walk_in_depth(self.spaciousness),
		}
	}

	fn ensuite_mins(&self) -> EnclosedRoomMins {
		EnclosedRoomMins {
			long: base_ensuite_length(self.spaciousness),
			short: base_ensuite_depth(self.spaciousness),
		}
	}

	fn bedroom_floor_reserve(&self, host: Aabb2d) -> f32 {
		let bed_floor = 2.0 * 1.6 * self.spaciousness * self.spaciousness;
		bed_floor.max(aabb2_area(host) * (1.0 - self.occupancy))
	}

	fn pack_partition(
		&self,
		host3: &Aabb3d,
		host: Aabb2d,
		clearances: &[Aabb2d],
		mins: EnclosedRoomMins,
		area_target: f32,
		area_reserve: f32,
		along_t: f32,
		door_id: OpeningId,
	) -> Option<BedroomPartition> {
		let contact = mins.short.min(mins.long * 0.55).max(0.55);
		let enclosed = EnclosedRoomParams {
			mins,
			contact,
			seed_depth: mins.short.max(0.6),
			along_t,
			area_target,
			area_reserve,
			reserve_cap_frac: 0.85,
			grow_into: false,
			shrink_sales_for_door_clear: true,
			door_width: self.door_width,
			door_width_min: DOOR_WIDTH_MIN,
			door_width_max: DOOR_WIDTH_MAX,
			door_along_t: self.door_along_t,
			door_height: self.door_height,
			door_height_min: DOOR_HEIGHT_MIN,
			door_height_max: DOOR_HEIGHT_MAX,
			door_header_min: DOOR_HEADER_MIN,
			door_clearance: PARTITION_DOOR_CLEARANCE,
			door_id: door_id.clone(),
		}
		.pack(host3, host, clearances)?;

		Some(partition_from_enclosed(host3, enclosed))
	}
}

fn partition_from_enclosed(host3: &Aabb3d, enclosed: EnclosedRoom) -> BedroomPartition {
	BedroomPartition {
		bounds: plan_to_aabb3(host3, enclosed.room, PlanAxes::XZ),
		walls: enclosed.walls,
		door_id: enclosed.door_id,
		door: enclosed.door,
		door_clear: enclosed.door_clear,
	}
}

/// Base clearances plus a [`PARTITION_SEP`] halo around existing closets / ensuites.
fn partition_pack_clearances(
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
) -> Vec<Aabb2d> {
	let mut out = clearances.to_vec();
	for part in packed
		.closets
		.iter()
		.chain(packed.walk_in_closets.iter())
		.chain(packed.ensuites.iter())
	{
		let plan = aabb3_to_plan(&part.bounds, PlanAxes::XZ);
		out.push(inflate_aabb2(plan, PARTITION_SEP));
	}
	out
}

/// Base clearances plus a [`STORAGE_SEP`] halo around existing wardrobes / dressers.
fn storage_pack_clearances(
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
) -> Vec<Aabb2d> {
	let mut out = clearances.to_vec();
	for solid in packed.wardrobes.iter().chain(packed.dressers.iter()) {
		out.push(inflate_aabb2(aabb3_to_plan(solid, PlanAxes::XZ), STORAGE_SEP));
	}
	out
}

fn xz_area(a: &Aabb3d) -> f32 {
	let e = a.max - a.min;
	e.x.max(0.0) * e.z.max(0.0)
}

fn base_bed_extent(spaciousness: f32) -> Vec3 {
	Vec3::new(2.0, 0.55, 1.6) * Vec3::new(spaciousness, 1.0, spaciousness)
}

fn base_nightstand_extent(spaciousness: f32) -> Vec3 {
	let s = 0.45 * spaciousness;
	Vec3::new(s, 0.5 * spaciousness.min(1.2), s)
}

fn base_wardrobe_extent(spaciousness: f32) -> Vec3 {
	Vec3::new(
		(1.1 * spaciousness).clamp(0.8, 2.0),
		(2.1 * spaciousness.min(1.15)).clamp(1.8, 2.4),
		(0.6 * spaciousness).clamp(0.45, 1.0),
	)
}

fn base_dresser_extent(spaciousness: f32) -> Vec3 {
	Vec3::new(
		(1.3 * spaciousness).clamp(0.9, 2.2),
		(0.9 * spaciousness.min(1.2)).clamp(0.7, 1.2),
		(0.5 * spaciousness).clamp(0.4, 0.85),
	)
}

fn base_closet_depth(spaciousness: f32) -> f32 {
	(0.75 * spaciousness).clamp(0.45, 2.0)
}

fn base_closet_length(spaciousness: f32) -> f32 {
	(1.6 * spaciousness).clamp(0.9, 4.0)
}

fn base_walk_in_depth(spaciousness: f32) -> f32 {
	(1.5 * spaciousness).clamp(1.2, 3.0)
}

fn base_walk_in_length(spaciousness: f32) -> f32 {
	(2.4 * spaciousness).clamp(2.0, 5.0)
}

fn base_ensuite_depth(spaciousness: f32) -> f32 {
	(1.8 * spaciousness).clamp(1.5, 3.2)
}

fn base_ensuite_length(spaciousness: f32) -> f32 {
	(2.6 * spaciousness).clamp(2.2, 5.5)
}

fn base_bedroom_furniture_extent(spaciousness: f32) -> Vec3 {
	Vec3::new(
		(1.5 * spaciousness).clamp(1.1, 2.4),
		(0.9 * spaciousness.min(1.2)).clamp(0.7, 1.2),
		(0.85 * spaciousness).clamp(0.65, 1.5),
	)
}

struct PickCtx {
	bed_count: usize,
	has_ensuite: bool,
	closet_count: usize,
	walk_in_count: usize,
	wardrobe_count: usize,
	dresser_count: usize,
	bedroom_furniture_count: usize,
	room_area: f32,
}

fn enclosure_soft_goal(ctx: &PickCtx) -> bool {
	ctx.has_ensuite || ctx.closet_count + ctx.walk_in_count >= 1
}

/// Sample the next concept under **program tiers** (enclosure → appointed → filler).
///
/// See module docs. Fillers stay gated until [`enclosure_soft_goal`] holds so
/// ensuite / closets claim floor before BedroomFurniture piles up. Appointed
/// storage still interleaves once the soft-goal is met (or mid-loop).
fn pick_concept(noise: &NoiseConfig, step: u32, ctx: PickCtx) -> Concept {
	let t = noise.sample_unit_4d(step as f32, 0.0, 0.0, 10.0);
	let large = ctx.room_area + 1e-3 >= LARGE_ROOM_AREA;
	let goal = enclosure_soft_goal(&ctx);

	// Enclosure-first while the soft-goal is unmet (not for the whole early loop).
	if !goal {
		if !ctx.has_ensuite && t < 0.45 {
			return Concept::Ensuite;
		}
		if large && ctx.walk_in_count == 0 && t < 0.62 {
			return Concept::WalkInCloset;
		}
		if t < 0.88 {
			return Concept::Closet;
		}
		if ctx.bed_count == 1 {
			return Concept::Nightstand;
		}
		return Concept::Closet;
	}

	// Soft-goal met: appointed first, then enclosure top-up, filler last.
	if ctx.bed_count == 1 && t < 0.28 {
		return Concept::Nightstand;
	}
	if t < 0.14 {
		return Concept::Bed;
	}
	if t < 0.34 {
		return Concept::Nightstand;
	}
	if t < 0.46 && ctx.wardrobe_count == 0 {
		return Concept::Wardrobe;
	}
	if t < 0.56 && ctx.dresser_count == 0 {
		return Concept::Dresser;
	}
	if !ctx.has_ensuite && t < 0.66 {
		return Concept::Ensuite;
	}
	if large && ctx.walk_in_count == 0 && t < 0.76 {
		return Concept::WalkInCloset;
	}
	if t < 0.86 {
		return Concept::Closet;
	}

	// Filler: gated + capped; only in large rooms once structure exists.
	if large && ctx.bedroom_furniture_count < MAX_BEDROOM_FURNITURE && step >= 8 {
		return Concept::BedroomFurniture;
	}

	Concept::Closet
}

fn collides_clearances(candidate: Aabb2d, clearances: &[Aabb2d]) -> bool {
	clearances.iter().any(|c| intersects_aabb2(candidate, *c))
}

fn collides_solids(candidate: &Aabb3d, packed: &CommonBedroomPacked) -> bool {
	packed
		.beds
		.iter()
		.chain(packed.nightstands.iter())
		.chain(packed.small_bedroom_furniture.iter())
		.chain(packed.wardrobes.iter())
		.chain(packed.dressers.iter())
		.chain(packed.bedroom_furniture.iter())
		.chain(packed.closets.iter().map(|p| &p.bounds))
		.chain(packed.walk_in_closets.iter().map(|p| &p.bounds))
		.chain(packed.ensuites.iter().map(|p| &p.bounds))
		.any(|a| aabb3_intersects(candidate, a))
}

fn aabb3_intersects(a: &Aabb3d, b: &Aabb3d) -> bool {
	a.min.x < b.max.x - 1e-4
		&& a.max.x > b.min.x + 1e-4
		&& a.min.y < b.max.y - 1e-4
		&& a.max.y > b.min.y + 1e-4
		&& a.min.z < b.max.z - 1e-4
		&& a.max.z > b.min.z + 1e-4
}

fn fits(
	candidate: &Aabb3d,
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
) -> bool {
	const EPS: f32 = 1e-3;
	if candidate.min.x < host3.min.x - EPS
		|| candidate.min.y < host3.min.y - EPS
		|| candidate.min.z < host3.min.z - EPS
		|| candidate.max.x > host3.max.x + EPS
		|| candidate.max.y > host3.max.y + EPS
		|| candidate.max.z > host3.max.z + EPS
	{
		return false;
	}
	let plan = aabb3_to_plan(candidate, PlanAxes::XZ);
	if plan.min.x < host.min.x - EPS
		|| plan.min.y < host.min.y - EPS
		|| plan.max.x > host.max.x + EPS
		|| plan.max.y > host.max.y + EPS
	{
		return false;
	}
	if collides_clearances(plan, clearances) {
		return false;
	}
	!collides_solids(candidate, packed)
}

fn place_bed(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
	noise: &NoiseConfig,
	salt: u32,
	spaciousness: f32,
	against_wall: bool,
) -> Option<Aabb3d> {
	let extent = base_bed_extent(spaciousness);
	let size = host3.max - host3.min;
	if extent.x > size.x + 1e-3 || extent.z > size.z + 1e-3 {
		return None;
	}
	if against_wall {
		if let Some(bed) =
			place_bed_against_wall(host3, host, clearances, packed, noise, salt, extent)
		{
			return Some(bed);
		}
	}
	place_bed_free(host3, host, clearances, packed, noise, salt, extent)
}

fn place_bed_against_wall(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
	noise: &NoiseConfig,
	salt: u32,
	extent: Vec3,
) -> Option<Aabb3d> {
	let size = host3.max - host3.min;
	let max_u = (size.x - extent.x).max(0.0);
	let max_v = (size.z - extent.z).max(0.0);
	// Prefer a noise-picked wall, then rotate through the other three.
	let start = (noise.sample_unit_4d(salt as f32, 0.0, 0.0, 22.0) * 4.0).floor() as u32 % 4;
	for k in 0..4u32 {
		let wall = (start + k) % 4;
		for attempt in 0..8u32 {
			let t = noise.sample_unit_4d(salt as f32, attempt as f32, wall as f32, 23.0);
			let min = match wall {
				0 => Vec3::new(host3.min.x + t * max_u, host3.min.y, host3.min.z), // −Z
				1 => Vec3::new(host3.min.x + t * max_u, host3.min.y, host3.max.z - extent.z), // +Z
				2 => Vec3::new(host3.min.x, host3.min.y, host3.min.z + t * max_v), // −X
				_ => Vec3::new(host3.max.x - extent.x, host3.min.y, host3.min.z + t * max_v), // +X
			};
			let candidate = Aabb3d::from_min_max(min, min + extent);
			if fits(&candidate, host3, host, clearances, packed)
				&& abuts_host_wall(aabb3_to_plan(&candidate, PlanAxes::XZ), host)
			{
				return Some(candidate);
			}
		}
	}
	None
}

fn place_bed_free(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
	noise: &NoiseConfig,
	salt: u32,
	extent: Vec3,
) -> Option<Aabb3d> {
	let size = host3.max - host3.min;
	for attempt in 0..12u32 {
		let u = noise.sample_unit_4d(salt as f32, attempt as f32, 0.0, 20.0);
		let v = noise.sample_unit_4d(salt as f32, attempt as f32, 0.0, 21.0);
		let max_u = (size.x - extent.x).max(0.0);
		let max_v = (size.z - extent.z).max(0.0);
		let min = Vec3::new(host3.min.x + u * max_u, host3.min.y, host3.min.z + v * max_v);
		let candidate = Aabb3d::from_min_max(min, min + extent);
		if fits(&candidate, host3, host, clearances, packed) {
			return Some(candidate);
		}
	}
	None
}

fn abuts_host_wall(plan: Aabb2d, host: Aabb2d) -> bool {
	const EPS: f32 = 0.06;
	(plan.min.x - host.min.x).abs() < EPS
		|| (plan.max.x - host.max.x).abs() < EPS
		|| (plan.min.y - host.min.y).abs() < EPS
		|| (plan.max.y - host.max.y).abs() < EPS
}

/// Free-standing storage with the **long** plan face flush on a host wall.
///
/// `extent` is authored as `(long, height, short)` in local furniture space.
fn place_storage_long_on_wall(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
	noise: &NoiseConfig,
	salt: u32,
	extent: Vec3,
) -> Option<Aabb3d> {
	let long = extent.x;
	let short = extent.z;
	let height = extent.y;
	let size = host3.max - host3.min;
	if long.min(short) > size.x.min(size.z) + 1e-3 {
		return None;
	}

	let start = (noise.sample_unit_4d(salt as f32, 0.0, 0.0, 40.0) * 4.0).floor() as u32 % 4;
	for k in 0..4u32 {
		let wall = (start + k) % 4;
		// −Z/+Z walls take long along X; −X/+X walls take long along Z.
		let e = match wall {
			0 | 1 => Vec3::new(long, height, short),
			_ => Vec3::new(short, height, long),
		};
		if e.x > size.x + 1e-3 || e.z > size.z + 1e-3 {
			continue;
		}
		let max_u = (size.x - e.x).max(0.0);
		let max_v = (size.z - e.z).max(0.0);
		for attempt in 0..8u32 {
			let t = noise.sample_unit_4d(salt as f32, attempt as f32, wall as f32, 41.0);
			let min = match wall {
				0 => Vec3::new(host3.min.x + t * max_u, host3.min.y, host3.min.z), // −Z
				1 => Vec3::new(host3.min.x + t * max_u, host3.min.y, host3.max.z - e.z), // +Z
				2 => Vec3::new(host3.min.x, host3.min.y, host3.min.z + t * max_v), // −X
				_ => Vec3::new(host3.max.x - e.x, host3.min.y, host3.min.z + t * max_v), // +X
			};
			let candidate = Aabb3d::from_min_max(min, min + e);
			let plan = aabb3_to_plan(&candidate, PlanAxes::XZ);
			if fits(&candidate, host3, host, clearances, packed)
				&& long_face_on_host_wall(plan, host)
			{
				return Some(candidate);
			}
		}
	}
	None
}

/// True when the longer plan edge of `plan` lies on a host wall.
fn long_face_on_host_wall(plan: Aabb2d, host: Aabb2d) -> bool {
	const EPS: f32 = 0.06;
	let w = plan.max.x - plan.min.x;
	let d = plan.max.y - plan.min.y;
	if w + EPS >= d {
		(plan.min.y - host.min.y).abs() < EPS || (plan.max.y - host.max.y).abs() < EPS
	} else {
		(plan.min.x - host.min.x).abs() < EPS || (plan.max.x - host.max.x).abs() < EPS
	}
}

/// Place a nightstand-scale box: adjacent to a bed when possible, else free-standing.
fn place_small_box(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
	noise: &NoiseConfig,
	salt: u32,
	spaciousness: f32,
) -> Option<Placed> {
	let extent = base_nightstand_extent(spaciousness);
	let gap = 0.08_f32 * spaciousness;
	if let Some(ns) = place_adjacent_to_bed(host3, host, clearances, packed, noise, salt, extent, gap)
	{
		return Some(Placed::Nightstand(ns));
	}
	place_free_extent(host3, host, clearances, packed, noise, salt, extent, 31.0)
		.map(Placed::SmallFurniture)
}

fn place_adjacent_to_bed(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
	noise: &NoiseConfig,
	salt: u32,
	extent: Vec3,
	gap: f32,
) -> Option<Aabb3d> {
	for (bi, bed) in packed.beds.iter().enumerate() {
		let start = (noise.sample_unit_4d(salt as f32, bi as f32, 0.0, 30.0) * 4.0).floor() as u32 % 4;
		for k in 0..4u32 {
			let side = (start + k) % 4;
			let mid_x = bed.min.x + (bed.max.x - bed.min.x) * 0.5 - extent.x * 0.5;
			let mid_z = bed.min.z + (bed.max.z - bed.min.z) * 0.5 - extent.z * 0.5;
			let min = match side {
				0 => Vec3::new(bed.max.x + gap, host3.min.y, mid_z), // +X
				1 => Vec3::new(bed.min.x - gap - extent.x, host3.min.y, mid_z), // −X
				2 => Vec3::new(mid_x, host3.min.y, bed.max.z + gap), // +Z
				_ => Vec3::new(mid_x, host3.min.y, bed.min.z - gap - extent.z), // −Z
			};
			let candidate = Aabb3d::from_min_max(min, min + extent);
			if fits(&candidate, host3, host, clearances, packed)
				&& abuts_bed(&candidate, bed, gap)
			{
				return Some(candidate);
			}
		}
	}
	None
}

fn abuts_bed(candidate: &Aabb3d, bed: &Aabb3d, gap: f32) -> bool {
	let c = aabb3_to_plan(candidate, PlanAxes::XZ);
	let b = aabb3_to_plan(bed, PlanAxes::XZ);
	// Close enough to touch after accounting for the authored gap.
	touches_aabb2(c, inflate_aabb2(b, gap + 0.05)) && !intersects_aabb2(c, b)
}

fn place_free_extent(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
	noise: &NoiseConfig,
	salt: u32,
	extent: Vec3,
	channel: f32,
) -> Option<Aabb3d> {
	let size = host3.max - host3.min;
	if extent.x > size.x + 1e-3 || extent.z > size.z + 1e-3 {
		return None;
	}
	for attempt in 0..10u32 {
		let u = noise.sample_unit_4d(salt as f32, attempt as f32, 0.0, channel);
		let v = noise.sample_unit_4d(salt as f32, attempt as f32, 0.0, channel + 1.0);
		let max_u = (size.x - extent.x).max(0.0);
		let max_v = (size.z - extent.z).max(0.0);
		let min = Vec3::new(host3.min.x + u * max_u, host3.min.y, host3.min.z + v * max_v);
		let candidate = Aabb3d::from_min_max(min, min + extent);
		if fits(&candidate, host3, host, clearances, packed) {
			return Some(candidate);
		}
	}
	None
}
