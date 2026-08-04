//! Plan packing for [`super::CommonBedroom`]: passage clearances, beds /
//! nightstands, then wall-seeded closets and ensuites via [`EnclosedRoom`].

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec3;
use procedural_common::{
	aabb2_area, aabb3_to_plan, intersects_aabb2, plan_to_aabb3, NoiseConfig, NoiseParams, PlanAxes,
};

use crate::fit::{Confines, FitError};
use crate::openings::{Opening, OpeningId};
use crate::paneling::Rectangle;
use crate::usage_areas::clearance::{PassageClearance, PASSAGE_CLEARANCE};
use crate::usage_areas::enclosed_room::{EnclosedRoom, EnclosedRoomMins, EnclosedRoomParams};

use super::parameterized::SCOPE;

const DOOR_WIDTH_MIN: f32 = 0.65;
const DOOR_WIDTH_MAX: f32 = 1.15;
const DOOR_HEIGHT_MIN: f32 = 1.8;
const DOOR_HEIGHT_MAX: f32 = 2.3;
const DOOR_HEADER_MIN: f32 = 0.2;

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
	pub nightstands: Vec<Aabb3d>,
	pub closets: Vec<BedroomPartition>,
	pub ensuites: Vec<BedroomPartition>,
}

/// Runtime knobs used by [`CommonBedroomRegions::pack`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommonBedroomRegions {
	pub spaciousness: f32,
	pub occupancy: f32,
	pub closet_along_t: f32,
	pub ensuite_along_t: f32,
	pub door_width: f32,
	pub door_along_t: f32,
	pub door_height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Concept {
	Bed,
	Nightstand,
	Closet,
	Ensuite,
}

enum Placed {
	Solid(Aabb3d),
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

		if let Some(bed) = place_bed(host3, host, &clearances, &packed, &cfg, 0, self.spaciousness)
		{
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
			let concept = pick_concept(&cfg, step, packed.beds.len());
			let Some(placed) = self.try_place(concept, host3, host, &clearances, &packed, &cfg, step)
			else {
				continue;
			};
			let add = match &placed {
				Placed::Solid(a) => xz_area(a),
				Placed::Partition(p) => xz_area(&p.bounds),
			};
			if (occupied + add) / room_area > self.occupancy + 1e-3 {
				continue;
			}
			occupied += add;
			match placed {
				Placed::Solid(a) => {
					clearances.push(aabb3_to_plan(&a, PlanAxes::XZ));
					match concept {
						Concept::Bed => packed.beds.push(a),
						Concept::Nightstand => packed.nightstands.push(a),
						Concept::Closet | Concept::Ensuite => unreachable!("partition via Solid"),
					}
				}
				Placed::Partition(part) => {
					clearances.push(aabb3_to_plan(&part.bounds, PlanAxes::XZ));
					clearances.push(part.door_clear);
					match concept {
						Concept::Closet => packed.closets.push(part),
						Concept::Ensuite => packed.ensuites.push(part),
						Concept::Bed | Concept::Nightstand => unreachable!("solid via Partition"),
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
			Concept::Bed => place_bed(host3, host, clearances, packed, cfg, salt, self.spaciousness)
				.map(Placed::Solid),
			Concept::Nightstand => place_nightstand(
				host3,
				host,
				clearances,
				packed,
				cfg,
				salt,
				self.spaciousness,
			)
			.map(Placed::Solid),
			Concept::Closet => self
				.pack_partition(
					host3,
					host,
					clearances,
					self.closet_mins(),
					self.closet_along_t,
					OpeningId::scoped(SCOPE, "closet_door", packed.closets.len().to_string()),
				)
				.map(Placed::Partition),
			Concept::Ensuite => self
				.pack_partition(
					host3,
					host,
					clearances,
					self.ensuite_mins(),
					self.ensuite_along_t,
					OpeningId::scoped(SCOPE, "ensuite_door", packed.ensuites.len().to_string()),
				)
				.map(Placed::Partition),
		}
	}

	fn closet_mins(&self) -> EnclosedRoomMins {
		EnclosedRoomMins {
			long: base_closet_length(self.spaciousness),
			short: base_closet_depth(self.spaciousness),
		}
	}

	fn ensuite_mins(&self) -> EnclosedRoomMins {
		EnclosedRoomMins {
			long: base_ensuite_length(self.spaciousness),
			short: base_ensuite_depth(self.spaciousness),
		}
	}

	fn pack_partition(
		&self,
		host3: &Aabb3d,
		host: Aabb2d,
		clearances: &[Aabb2d],
		mins: EnclosedRoomMins,
		along_t: f32,
		door_id: OpeningId,
	) -> Option<BedroomPartition> {
		let contact = mins.short.min(mins.long * 0.55).max(0.55);
		let area_target = (mins.long * mins.short * 1.35).min(aabb2_area(host) * 0.35);
		let bed_floor = 2.0 * 1.6 * self.spaciousness * self.spaciousness;
		let enclosed = EnclosedRoomParams {
			mins,
			contact,
			seed_depth: mins.short.max(0.6),
			along_t,
			area_target,
			area_reserve: bed_floor.max(aabb2_area(host) * (1.0 - self.occupancy)),
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
			door_clearance: PASSAGE_CLEARANCE,
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

fn base_closet_depth(spaciousness: f32) -> f32 {
	(0.75 * spaciousness).clamp(0.45, 2.0)
}

fn base_closet_length(spaciousness: f32) -> f32 {
	(1.6 * spaciousness).clamp(0.9, 4.0)
}

fn base_ensuite_depth(spaciousness: f32) -> f32 {
	(1.1 * spaciousness).clamp(0.7, 2.5)
}

fn base_ensuite_length(spaciousness: f32) -> f32 {
	(2.0 * spaciousness).clamp(1.2, 5.0)
}

fn pick_concept(noise: &NoiseConfig, step: u32, bed_count: usize) -> Concept {
	let t = noise.sample_unit_4d(step as f32, 0.0, 0.0, 10.0);
	if bed_count == 1 && t < 0.35 {
		Concept::Nightstand
	} else if t < 0.25 {
		Concept::Bed
	} else if t < 0.5 {
		Concept::Nightstand
	} else if t < 0.75 {
		Concept::Closet
	} else {
		Concept::Ensuite
	}
}

fn collides_clearances(candidate: Aabb2d, clearances: &[Aabb2d]) -> bool {
	clearances.iter().any(|c| intersects_aabb2(candidate, *c))
}

fn collides_solids(candidate: &Aabb3d, packed: &CommonBedroomPacked) -> bool {
	packed
		.beds
		.iter()
		.chain(packed.nightstands.iter())
		.chain(packed.closets.iter().map(|p| &p.bounds))
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
) -> Option<Aabb3d> {
	let extent = base_bed_extent(spaciousness);
	let size = host3.max - host3.min;
	if extent.x > size.x + 1e-3 || extent.z > size.z + 1e-3 {
		return None;
	}
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

fn place_nightstand(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
	noise: &NoiseConfig,
	salt: u32,
	spaciousness: f32,
) -> Option<Aabb3d> {
	let extent = base_nightstand_extent(spaciousness);
	let gap = 0.08_f32 * spaciousness;
	for (bi, bed) in packed.beds.iter().enumerate() {
		let side = noise.sample_unit_4d(salt as f32, bi as f32, 0.0, 30.0) >= 0.5;
		let x = if side {
			bed.max.x + gap
		} else {
			bed.min.x - gap - extent.x
		};
		let z = bed.min.z + (bed.max.z - bed.min.z) * 0.5 - extent.z * 0.5;
		let min = Vec3::new(x, host3.min.y, z);
		let candidate = Aabb3d::from_min_max(min, min + extent);
		if fits(&candidate, host3, host, clearances, packed) {
			return Some(candidate);
		}
	}
	place_free_extent(host3, host, clearances, packed, noise, salt, extent, 31.0)
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
