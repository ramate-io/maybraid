//! Public restroom plan packer: strip reserve → walled stalls (+door) → sinks.
//!
//! Stalls seed against clearances∪door-side strip so they cannot eat the sink
//! zone; sinks use [`pack_abutting_clearance`] against the stalls-door keep-out.
//! Door id scope: `public_restroom`.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use procedural_common::{
	aabb2_area, aabb3_to_plan, clamp_min_size2, plan_to_aabb3, Aabb2dPack, PlanAxes, PlanOpeningFace,
};

use crate::bedroom::shell::{face_rectangle, face_span_rectangle};
use crate::constraints::FaceKind;
use crate::fit::{Confines, FitError};
use crate::openings::{Opening, OpeningId};
use crate::paneling::{Rectangle, DEFAULT_PANEL_THICKNESS};
use crate::usage_areas::clearance::{
	abuts_clearance, pack_abutting_clearance, PassageClearance, PlanHost, PASSAGE_CLEARANCE,
};

/// Scope prefix for [`OpeningId::scoped`] openings authored by PublicRestroom.
pub const SCOPE: &str = "public_restroom";

pub const RESTROOM_STALLS_MIN: f32 = 2.0;
pub const RESTROOM_STALLS_CONTACT: f32 = 2.0;
pub const RESTROOM_SINK_MIN: f32 = 0.5;
pub const RESTROOM_SINK_DEPTH_MIN: f32 = 0.5;
pub const RESTROOM_SINK_DEPTH_MAX: f32 = 0.9;
pub const RESTROOM_DOOR_WIDTH_MIN: f32 = 0.9;
pub const RESTROOM_DOOR_WIDTH_MAX: f32 = 1.2;
pub const RESTROOM_DOOR_HEIGHT_MIN: f32 = 2.0;
pub const RESTROOM_DOOR_HEIGHT_MAX: f32 = 2.4;
pub const RESTROOM_DOOR_HEADER_MIN: f32 = 0.25;

#[derive(Debug, Clone, PartialEq)]
pub struct RestroomStallsDoor {
	pub id: OpeningId,
	pub opening: Opening,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PublicRestroomRegions {
	pub stalls_area_target: f32,
	pub stalls_seed_depth: f32,
	pub stalls_along_t: f32,
	/// Plan area reserved for sinks when clamping stalls grow (bites kitchen-style).
	pub sink_area_reserve: f32,
	/// Grow target for the primary sink pocket after seeding against the door clear.
	pub sink_area_target: f32,
	pub door_width: f32,
	pub door_along_t: f32,
	pub door_height: f32,
	pub sink_depth: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PublicRestroomPacked {
	pub stalls: Aabb3d,
	pub sinks: Vec<Aabb3d>,
	pub stall_walls: Vec<Rectangle>,
	pub stalls_door: RestroomStallsDoor,
}

struct StallsEnclosure {
	walls: Vec<Rectangle>,
	door_clear: Aabb2d,
	stalls_door: RestroomStallsDoor,
}

impl PublicRestroomRegions {
	pub fn pack(&self, confines: &Confines) -> Result<PublicRestroomPacked, FitError> {
		let host3 = &confines.bounds;
		let host = aabb3_to_plan(host3, PlanAxes::XZ);
		let passage_faces = PassageClearance::collect_faces(confines, host);
		if passage_faces.is_empty() {
			return Err(FitError::TooSmall {
				reason: "restroom passage",
			});
		}

		let mut clearances = PassageClearance::bands_std(host, &passage_faces);
		let sink_depth = self
			.sink_depth
			.clamp(RESTROOM_SINK_DEPTH_MIN, RESTROOM_SINK_DEPTH_MAX);
		// Door-side strip toward the entry: stalls-door keep-out + sink pocket + customer
		// passage keep-out can stack on the same wall, so reserve all three.
		let free_strip_depth = 2.0 * PASSAGE_CLEARANCE + sink_depth;

		let (stalls2, seed_face) = self
			.pack_stalls(host, &clearances, free_strip_depth)
			.ok_or(FitError::TooSmall {
				reason: "restroom stalls",
			})?;

		let enclosure = self
			.stalls_enclosure(host3, host, stalls2, seed_face)
			.ok_or(FitError::TooSmall {
				reason: "restroom stalls door",
			})?;
		clearances.push(enclosure.door_clear);

		let sinks = self
			.pack_sinks(host, &clearances, stalls2, seed_face, enclosure.door_clear)
			.ok_or(FitError::TooSmall {
				reason: "restroom sink",
			})?;

		Ok(PublicRestroomPacked {
			stalls: plan_to_aabb3(host3, stalls2, PlanAxes::XZ),
			sinks: sinks
				.into_iter()
				.map(|s| plan_to_aabb3(host3, s, PlanAxes::XZ))
				.collect(),
			stall_walls: enclosure.walls,
			stalls_door: enclosure.stalls_door,
		})
	}

	fn stalls_dims_ok(stalls: Aabb2d) -> bool {
		let w = stalls.max.x - stalls.min.x;
		let d = stalls.max.y - stalls.min.y;
		w + 1e-3 >= RESTROOM_STALLS_MIN && d + 1e-3 >= RESTROOM_STALLS_MIN
	}

	/// Free strip on the door side of stalls (between stalls free edge and opposite host wall).
	fn door_side_zone(host: Aabb2d, stalls2: Aabb2d, seed_face: PlanOpeningFace) -> Option<Aabb2d> {
		const EPS: f32 = 1e-3;
		let zone = if seed_face.thru_is_x {
			if seed_face.inward_positive {
				Aabb2d {
					min: Vec2::new(stalls2.max.x + EPS, host.min.y),
					max: Vec2::new(host.max.x, host.max.y),
				}
			} else {
				Aabb2d {
					min: Vec2::new(host.min.x, host.min.y),
					max: Vec2::new(stalls2.min.x - EPS, host.max.y),
				}
			}
		} else if seed_face.inward_positive {
			Aabb2d {
				min: Vec2::new(host.min.x, stalls2.max.y + EPS),
				max: Vec2::new(host.max.x, host.max.y),
			}
		} else {
			Aabb2d {
				min: Vec2::new(host.min.x, host.min.y),
				max: Vec2::new(host.max.x, stalls2.min.y - EPS),
			}
		};
		if zone.max.x - zone.min.x < RESTROOM_SINK_MIN - 1e-3
			|| zone.max.y - zone.min.y < RESTROOM_SINK_MIN - 1e-3
		{
			return None;
		}
		Some(zone)
	}

	/// Inward band from the host wall opposite `seed_face`, reserved for door clearance + sinks.
	fn free_strip_block(host: Aabb2d, seed_face: PlanOpeningFace, depth: f32) -> Option<Aabb2d> {
		let opposite = PlanOpeningFace {
			thru_is_x: seed_face.thru_is_x,
			thru: if seed_face.thru_is_x {
				if seed_face.inward_positive {
					host.max.x
				} else {
					host.min.x
				}
			} else if seed_face.inward_positive {
				host.max.y
			} else {
				host.min.y
			},
			along0: if seed_face.thru_is_x {
				host.min.y
			} else {
				host.min.x
			},
			along1: if seed_face.thru_is_x {
				host.max.y
			} else {
				host.max.x
			},
			inward_positive: !seed_face.inward_positive,
		};
		opposite.band(host, opposite.along_len(), depth, 0.5)
	}

	fn pack_stalls(
		&self,
		host: Aabb2d,
		clearances: &[Aabb2d],
		free_strip_depth: f32,
	) -> Option<(Aabb2d, PlanOpeningFace)> {
		let mut candidates: Vec<PlanOpeningFace> = PlanHost::faces(host).into_iter().collect();
		candidates.sort_by(|a, b| {
			let blocked = |wall: PlanOpeningFace| {
				clearances.iter().any(|c| {
					wall.shared_border_len(*c) + 1e-3
						>= RESTROOM_STALLS_CONTACT.min(wall.along_len())
				}) as u8
			};
			blocked(*a).cmp(&blocked(*b)).then_with(|| {
				b.along_len()
					.partial_cmp(&a.along_len())
					.unwrap_or(std::cmp::Ordering::Equal)
			})
		});

		let contact = RESTROOM_STALLS_CONTACT;
		let depth = self.stalls_seed_depth.max(RESTROOM_STALLS_MIN);
		let usable = aabb2_area(host);
		// Prefer filling most of the bay; only a thin door/sink strip is reserved.
		let strip_area = free_strip_depth
			* (host.max.x - host.min.x).max(host.max.y - host.min.y);
		let reserve = self
			.sink_area_reserve
			.max(RESTROOM_SINK_MIN * RESTROOM_SINK_MIN)
			.max(strip_area * 0.5)
			.min(usable * 0.35);
		let target = self
			.stalls_area_target
			.max(RESTROOM_STALLS_MIN * RESTROOM_STALLS_MIN)
			.min((usable - reserve).max(RESTROOM_STALLS_MIN * RESTROOM_STALLS_MIN));

		for face in candidates {
			if face.along_len() + 1e-3 < contact {
				continue;
			}
			let Some(strip) = Self::free_strip_block(host, face, free_strip_depth) else {
				continue;
			};
			// Seed against clearances *and* the door/sink strip so the seed cannot
			// start inside the reserved zone (grow_* will not shrink an overlap out).
			let mut seed_hard = clearances.to_vec();
			seed_hard.push(strip);
			let Some(seed) = face
				.seed_from_free(host, &seed_hard, contact, depth, self.stalls_along_t)
				.or_else(|| face.seed_from_free(host, &seed_hard, contact, depth, 0.5))
			else {
				continue;
			};
			let mut hard = seed_hard;
			hard.push(face.outward_block(host));
			let grown = seed
				.grow_toward_area(host, &hard, target)
				.grow_into(host, &hard);
			let Some(stalls) = clamp_min_size2(grown, Vec2::splat(RESTROOM_STALLS_MIN)) else {
				continue;
			};
			if !stalls.is_clear_of(&hard) {
				continue;
			}
			if !Self::stalls_dims_ok(stalls) {
				continue;
			}
			if face.shared_border_len(stalls) + 1e-3 < contact {
				continue;
			}
			// Must still leave a door-side zone for sinks.
			let Some(zone) = Self::door_side_zone(host, stalls, face) else {
				continue;
			};
			// Zone must be deep enough for a min sink on at least one axis after door clear.
			let zone_short = (zone.max.x - zone.min.x).min(zone.max.y - zone.min.y);
			if zone_short + 1e-3 < RESTROOM_SINK_MIN {
				continue;
			}
			return Some((stalls, face));
		}
		None
	}

	fn stalls_enclosure(
		&self,
		host3: &Aabb3d,
		host: Aabb2d,
		stalls2: Aabb2d,
		seed_face: PlanOpeningFace,
	) -> Option<StallsEnclosure> {
		let stalls3 = plan_to_aabb3(host3, stalls2, PlanAxes::XZ);
		let entry_face = Self::entry_face_kind(seed_face);
		let divider_thru = if seed_face.thru_is_x {
			if seed_face.inward_positive {
				stalls2.max.x
			} else {
				stalls2.min.x
			}
		} else if seed_face.inward_positive {
			stalls2.max.y
		} else {
			stalls2.min.y
		};

		let door_face = PlanOpeningFace {
			thru_is_x: seed_face.thru_is_x,
			thru: divider_thru,
			along0: if seed_face.thru_is_x {
				stalls2.min.y
			} else {
				stalls2.min.x
			},
			along1: if seed_face.thru_is_x {
				stalls2.max.y
			} else {
				stalls2.max.x
			},
			inward_positive: seed_face.inward_positive,
		};
		let door_face = door_face.clip_to_host(host)?;
		let along_len = door_face.along_len();
		let max_door = (along_len - 0.4).max(along_len * 0.5);
		let door_w = self
			.door_width
			.clamp(RESTROOM_DOOR_WIDTH_MIN, RESTROOM_DOOR_WIDTH_MAX)
			.min(max_door)
			.min(along_len * 0.85);
		if door_w + 1e-3 < RESTROOM_DOOR_WIDTH_MIN.min(along_len * 0.45) || along_len + 1e-3 < door_w
		{
			return None;
		}

		let (door0, door1, door_clear) = {
			let mut placed = None;
			for &t in &[self.door_along_t, 0.5, 0.25, 0.75] {
				let Some((d0, d1)) = place_along(door_face.along0, door_face.along1, door_w, t)
				else {
					continue;
				};
				if let Some(clear) = door_face.band(host, door_w, PASSAGE_CLEARANCE, t) {
					placed = Some((d0, d1, clear));
					break;
				}
			}
			placed?
		};

		let host_h = (host3.max.y - host3.min.y).max(1.0);
		let door_h = self
			.door_height
			.clamp(RESTROOM_DOOR_HEIGHT_MIN, RESTROOM_DOOR_HEIGHT_MAX)
			.min((host_h - RESTROOM_DOOR_HEADER_MIN).max(RESTROOM_DOOR_HEIGHT_MIN));
		let u0 = ((door0 - door_face.along0) / along_len).clamp(0.0, 1.0);
		let u1 = ((door1 - door_face.along0) / along_len).clamp(0.0, 1.0);

		let mut walls = Vec::new();
		for face in [
			FaceKind::Front,
			FaceKind::Back,
			FaceKind::Left,
			FaceKind::Right,
		] {
			if face == entry_face || Self::stalls_side_on_host(stalls2, host, face) {
				continue;
			}
			if let Some(r) = face_rectangle(&stalls3, face, DEFAULT_PANEL_THICKNESS) {
				walls.push(r);
			}
		}

		let mut door_panels = 0usize;
		if u0 > 0.02 {
			if let Some(r) =
				face_span_rectangle(&stalls3, entry_face, 0.0, u0, DEFAULT_PANEL_THICKNESS)
			{
				walls.push(r);
				door_panels += 1;
			}
		}
		if u1 < 0.98 {
			if let Some(r) =
				face_span_rectangle(&stalls3, entry_face, u1, 1.0, DEFAULT_PANEL_THICKNESS)
			{
				walls.push(r);
				door_panels += 1;
			}
		}
		if door_h + RESTROOM_DOOR_HEADER_MIN <= host_h + 1e-3 {
			let omin = Vec3::from(stalls3.min);
			let omax = Vec3::from(stalls3.max);
			let header_aabb =
				Aabb3d::from_min_max(Vec3::new(omin.x, omin.y + door_h, omin.z), omax);
			if let Some(r) =
				face_span_rectangle(&header_aabb, entry_face, u0, u1, DEFAULT_PANEL_THICKNESS)
			{
				walls.push(r);
				door_panels += 1;
			}
		}
		if door_panels == 0 {
			return None;
		}

		let band = (DEFAULT_PANEL_THICKNESS * 0.5 + 0.08).max(0.12);
		let a0 = door0.min(door1);
		let a1 = door0.max(door1);
		let y0 = host3.min.y;
		let y1 = y0 + door_h.max(0.5);
		let door_bounds = if seed_face.thru_is_x {
			Aabb3d::from_min_max(
				Vec3::new(divider_thru - band, y0, a0),
				Vec3::new(divider_thru + band, y1, a1),
			)
		} else {
			Aabb3d::from_min_max(
				Vec3::new(a0, y0, divider_thru - band),
				Vec3::new(a1, y1, divider_thru + band),
			)
		};

		Some(StallsEnclosure {
			walls,
			door_clear,
			stalls_door: RestroomStallsDoor {
				id: OpeningId::scoped(SCOPE, "stalls_door", "0"),
				opening: Opening::passage(door_bounds),
			},
		})
	}

	fn entry_face_kind(seed_face: PlanOpeningFace) -> FaceKind {
		match (seed_face.thru_is_x, seed_face.inward_positive) {
			(true, true) => FaceKind::Right,
			(true, false) => FaceKind::Left,
			(false, true) => FaceKind::Back,
			(false, false) => FaceKind::Front,
		}
	}

	fn stalls_side_on_host(stalls2: Aabb2d, host: Aabb2d, face: FaceKind) -> bool {
		const EPS: f32 = 0.08;
		match face {
			FaceKind::Front => (stalls2.min.y - host.min.y).abs() < EPS,
			FaceKind::Back => (stalls2.max.y - host.max.y).abs() < EPS,
			FaceKind::Left => (stalls2.min.x - host.min.x).abs() < EPS,
			FaceKind::Right => (stalls2.max.x - host.max.x).abs() < EPS,
			FaceKind::Top | FaceKind::Bottom => true,
		}
	}

	/// Sinks live in the door-side free strip, abutting the stalls-door clearance
	/// (kitchen-style: seed against keep-out, then grow toward a target).
	fn pack_sinks(
		&self,
		_host: Aabb2d,
		clearances: &[Aabb2d],
		stalls2: Aabb2d,
		seed_face: PlanOpeningFace,
		door_clear: Aabb2d,
	) -> Option<Vec<Aabb2d>> {
		let zone = Self::door_side_zone(_host, stalls2, seed_face)?;
		let mut hard = clearances.to_vec();
		hard.push(stalls2);

		let min = Vec2::splat(RESTROOM_SINK_MIN);
		let zone_area = aabb2_area(zone);
		let target = self
			.sink_area_target
			.max(RESTROOM_SINK_MIN * RESTROOM_SINK_MIN)
			.min(zone_area.max(RESTROOM_SINK_MIN * RESTROOM_SINK_MIN));

		let primary = pack_abutting_clearance(zone, &hard, door_clear, min, target)?;
		debug_assert!(abuts_clearance(primary, door_clear));
		let mut sinks = vec![primary];
		hard.push(primary);

		// Opportunistic second pocket (same abutting grow) if space remains.
		let extra_target = (target * 0.55)
			.max(RESTROOM_SINK_MIN * RESTROOM_SINK_MIN)
			.min((zone_area - aabb2_area(primary)).max(RESTROOM_SINK_MIN * RESTROOM_SINK_MIN));
		if let Some(extra) = pack_abutting_clearance(zone, &hard, door_clear, min, extra_target) {
			sinks.push(extra);
		}

		Some(sinks)
	}
}

fn place_along(a0: f32, a1: f32, len: f32, t: f32) -> Option<(f32, f32)> {
	let span = a1 - a0;
	if span + 1e-4 < len {
		return None;
	}
	let t = t.clamp(0.0, 1.0);
	let start = a0 + (span - len) * t;
	Some((start, start + len))
}
