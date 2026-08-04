//! Shared “room against a host wall + sales-face door + enclosure panels” packer.
//!
//! Used by MiniMart / Parts offices and PublicRestroom toilet stalls. Callers keep
//! domain mins, reserves, door catalogs, and post-pack fill (aisles, sinks, …).

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use procedural_common::{
	aabb2_area, clamp_min_size2, plan_to_aabb3, Aabb2dPack, PlanAxes, PlanOpeningFace,
};

use crate::bedroom::shell::{face_rectangle, face_span_rectangle};
use crate::constraints::FaceKind;
use crate::openings::{Opening, OpeningId};
use crate::paneling::{Rectangle, DEFAULT_PANEL_THICKNESS};
use crate::usage_areas::clearance::PlanHost;

/// Plan minimums for an enclosed room (long/short axes after packing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnclosedRoomMins {
	pub long: f32,
	pub short: f32,
}

impl EnclosedRoomMins {
	pub fn square(min: f32) -> Self {
		Self {
			long: min,
			short: min,
		}
	}

	pub fn ok(self, room: Aabb2d) -> bool {
		let w = room.max.x - room.min.x;
		let d = room.max.y - room.min.y;
		let (long, short) = if w >= d { (w, d) } else { (d, w) };
		long + 1e-3 >= self.long && short + 1e-3 >= self.short
	}

	fn min_area(self) -> f32 {
		self.long * self.short
	}
}

/// Knobs for [`EnclosedRoomParams::pack`] / [`EnclosedRoomParams::pack_filtered`].
#[derive(Debug, Clone, PartialEq)]
pub struct EnclosedRoomParams {
	pub mins: EnclosedRoomMins,
	pub contact: f32,
	pub seed_depth: f32,
	pub along_t: f32,
	pub area_target: f32,
	/// Area reserved for the rest of the bay when clamping the grow target.
	/// Caller should already fold domain floors (aisles, sinks, …) into this value.
	pub area_reserve: f32,
	/// Cap on [`Self::area_reserve`] as a fraction of host area.
	pub reserve_cap_frac: f32,
	/// Also `grow_into` after `grow_toward_area` (restroom fills harder).
	pub grow_into: bool,
	/// Pull the free/sales face back so a door clearance band fits in-host.
	/// Restroom usually leaves a reserved strip instead and sets this false.
	pub shrink_sales_for_door_clear: bool,
	pub door_width: f32,
	pub door_width_min: f32,
	pub door_width_max: f32,
	pub door_along_t: f32,
	pub door_height: f32,
	pub door_height_min: f32,
	pub door_height_max: f32,
	pub door_header_min: f32,
	/// Inward clearance depth for the sales-face door keep-out (usually [`PASSAGE_CLEARANCE`]).
	pub door_clearance: f32,
	pub door_id: OpeningId,
}

/// Packed enclosed room on the plan + enclosure panels / door.
#[derive(Debug, Clone, PartialEq)]
pub struct EnclosedRoom {
	pub room: Aabb2d,
	pub seed_face: PlanOpeningFace,
	pub walls: Vec<Rectangle>,
	pub door_clear: Aabb2d,
	pub door_id: OpeningId,
	pub door: Opening,
}

impl EnclosedRoomParams {
	/// Pack with no per-face seed extras and no extra accept filter.
	pub fn pack(
		&self,
		host3: &Aabb3d,
		host: Aabb2d,
		clearances: &[Aabb2d],
	) -> Option<EnclosedRoom> {
		self.pack_filtered(
			host3,
			host,
			clearances,
			|_| Some(Vec::new()),
			|_, _| true,
		)
	}

	/// Like [`Self::pack`], but:
	/// - `face_seed_extra` may add hard excludes per candidate face (`None` skips the face)
	/// - `accept` may reject a grown room after mins / contact checks (e.g. restroom door-side zone)
	pub fn pack_filtered(
		&self,
		host3: &Aabb3d,
		host: Aabb2d,
		clearances: &[Aabb2d],
		mut face_seed_extra: impl FnMut(PlanOpeningFace) -> Option<Vec<Aabb2d>>,
		accept: impl Fn(Aabb2d, PlanOpeningFace) -> bool,
	) -> Option<EnclosedRoom> {
		let (mut room, seed_face) =
			self.pack_region(host, clearances, &mut face_seed_extra, &accept)?;
		if self.shrink_sales_for_door_clear {
			room = shrink_for_door_clearance(
				host,
				room,
				seed_face,
				self.door_clearance + 0.05,
				self.mins,
				self.contact,
			)?;
		}
		let enclosure = self.enclose(host3, host, room, seed_face)?;
		Some(EnclosedRoom {
			room,
			seed_face,
			walls: enclosure.walls,
			door_clear: enclosure.door_clear,
			door_id: self.door_id.clone(),
			door: enclosure.door,
		})
	}

	fn pack_region(
		&self,
		host: Aabb2d,
		clearances: &[Aabb2d],
		face_seed_extra: &mut impl FnMut(PlanOpeningFace) -> Option<Vec<Aabb2d>>,
		accept: &impl Fn(Aabb2d, PlanOpeningFace) -> bool,
	) -> Option<(Aabb2d, PlanOpeningFace)> {
		let mut candidates: Vec<PlanOpeningFace> = PlanHost::faces(host).into_iter().collect();
		candidates.sort_by(|a, b| {
			let blocked = |wall: PlanOpeningFace| {
				clearances.iter().any(|c| {
					wall.shared_border_len(*c) + 1e-3 >= self.contact.min(wall.along_len())
				}) as u8
			};
			blocked(*a).cmp(&blocked(*b)).then_with(|| {
				b.along_len()
					.partial_cmp(&a.along_len())
					.unwrap_or(std::cmp::Ordering::Equal)
			})
		});

		let contact = self.contact;
		let depth = self.seed_depth.max(self.mins.short);
		let usable = aabb2_area(host);
		let reserve = self.area_reserve.min(usable * self.reserve_cap_frac);
		let target = self
			.area_target
			.max(self.mins.min_area())
			.min((usable - reserve).max(self.mins.min_area()));

		for face in candidates {
			if face.along_len() + 1e-3 < contact {
				continue;
			}
			let Some(extra) = face_seed_extra(face) else {
				continue;
			};
			let mut seed_hard = clearances.to_vec();
			seed_hard.extend(extra);
			let Some(seed) = face
				.seed_from_free(host, &seed_hard, contact, depth, self.along_t)
				.or_else(|| face.seed_from_free(host, &seed_hard, contact, depth, 0.5))
			else {
				continue;
			};
			let mut hard = seed_hard;
			hard.push(face.outward_block(host));
			let mut grown = seed.grow_toward_area(host, &hard, target);
			if self.grow_into {
				grown = grown.grow_into(host, &hard);
			}
			let Some(room) = clamp_min_size2(grown, Vec2::splat(self.mins.short)) else {
				continue;
			};
			if !room.is_clear_of(&hard) {
				continue;
			}
			if !self.mins.ok(room) {
				continue;
			}
			if face.shared_border_len(room) + 1e-3 < contact {
				continue;
			}
			if !accept(room, face) {
				continue;
			}
			return Some((room, face));
		}
		None
	}

	fn enclose(
		&self,
		host3: &Aabb3d,
		host: Aabb2d,
		room2: Aabb2d,
		seed_face: PlanOpeningFace,
	) -> Option<EnclosureGeom> {
		let room3 = plan_to_aabb3(host3, room2, PlanAxes::XZ);
		let sales_face = sales_face_kind(seed_face);
		let divider_thru = if seed_face.thru_is_x {
			if seed_face.inward_positive {
				room2.max.x
			} else {
				room2.min.x
			}
		} else if seed_face.inward_positive {
			room2.max.y
		} else {
			room2.min.y
		};

		let door_face = PlanOpeningFace {
			thru_is_x: seed_face.thru_is_x,
			thru: divider_thru,
			along0: if seed_face.thru_is_x {
				room2.min.y
			} else {
				room2.min.x
			},
			along1: if seed_face.thru_is_x {
				room2.max.y
			} else {
				room2.max.x
			},
			inward_positive: seed_face.inward_positive,
		};
		let door_face = door_face.clip_to_host(host)?;
		let along_len = door_face.along_len();
		let max_door = (along_len - 0.4).max(along_len * 0.5);
		let door_w = self
			.door_width
			.clamp(self.door_width_min, self.door_width_max)
			.min(max_door)
			.min(along_len * 0.85);
		if door_w + 1e-3 < self.door_width_min.min(along_len * 0.45) || along_len + 1e-3 < door_w {
			return None;
		}

		let (door0, door1, door_clear) = {
			let mut placed = None;
			for &t in &[self.door_along_t, 0.5, 0.25, 0.75] {
				let Some((d0, d1)) = place_along(door_face.along0, door_face.along1, door_w, t)
				else {
					continue;
				};
				if let Some(clear) = door_face.band(host, door_w, self.door_clearance, t) {
					placed = Some((d0, d1, clear));
					break;
				}
			}
			placed?
		};

		let host_h = (host3.max.y - host3.min.y).max(1.0);
		let door_h = self
			.door_height
			.clamp(self.door_height_min, self.door_height_max)
			.min((host_h - self.door_header_min).max(self.door_height_min));
		let u0 = ((door0 - door_face.along0) / along_len).clamp(0.0, 1.0);
		let u1 = ((door1 - door_face.along0) / along_len).clamp(0.0, 1.0);

		let mut walls = Vec::new();
		for face in [
			FaceKind::Front,
			FaceKind::Back,
			FaceKind::Left,
			FaceKind::Right,
		] {
			if face == sales_face || side_on_host(room2, host, face) {
				continue;
			}
			if let Some(r) = face_rectangle(&room3, face, DEFAULT_PANEL_THICKNESS) {
				walls.push(r);
			}
		}

		let mut door_panels = 0usize;
		if u0 > 0.02 {
			if let Some(r) =
				face_span_rectangle(&room3, sales_face, 0.0, u0, DEFAULT_PANEL_THICKNESS)
			{
				walls.push(r);
				door_panels += 1;
			}
		}
		if u1 < 0.98 {
			if let Some(r) =
				face_span_rectangle(&room3, sales_face, u1, 1.0, DEFAULT_PANEL_THICKNESS)
			{
				walls.push(r);
				door_panels += 1;
			}
		}
		if door_h + self.door_header_min <= host_h + 1e-3 {
			let omin = Vec3::from(room3.min);
			let omax = Vec3::from(room3.max);
			let header_aabb =
				Aabb3d::from_min_max(Vec3::new(omin.x, omin.y + door_h, omin.z), omax);
			if let Some(r) =
				face_span_rectangle(&header_aabb, sales_face, u0, u1, DEFAULT_PANEL_THICKNESS)
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

		Some(EnclosureGeom {
			walls,
			door_clear,
			door: Opening::passage(door_bounds),
		})
	}
}

struct EnclosureGeom {
	walls: Vec<Rectangle>,
	door_clear: Aabb2d,
	door: Opening,
}

fn shrink_for_door_clearance(
	host: Aabb2d,
	mut room: Aabb2d,
	seed_face: PlanOpeningFace,
	need: f32,
	mins: EnclosedRoomMins,
	contact: f32,
) -> Option<Aabb2d> {
	if seed_face.thru_is_x {
		if seed_face.inward_positive {
			room.max.x = room.max.x.min(host.max.x - need);
		} else {
			room.min.x = room.min.x.max(host.min.x + need);
		}
	} else if seed_face.inward_positive {
		room.max.y = room.max.y.min(host.max.y - need);
	} else {
		room.min.y = room.min.y.max(host.min.y + need);
	}
	if room.max.x - room.min.x < 1e-3 || room.max.y - room.min.y < 1e-3 {
		return None;
	}
	if !mins.ok(room) {
		return None;
	}
	if seed_face.shared_border_len(room) + 1e-3 < contact {
		return None;
	}
	Some(room)
}

fn sales_face_kind(seed_face: PlanOpeningFace) -> FaceKind {
	match (seed_face.thru_is_x, seed_face.inward_positive) {
		(true, true) => FaceKind::Right,
		(true, false) => FaceKind::Left,
		(false, true) => FaceKind::Back,
		(false, false) => FaceKind::Front,
	}
}

fn side_on_host(room2: Aabb2d, host: Aabb2d, face: FaceKind) -> bool {
	const EPS: f32 = 0.08;
	match face {
		FaceKind::Front => (room2.min.y - host.min.y).abs() < EPS,
		FaceKind::Back => (room2.max.y - host.max.y).abs() < EPS,
		FaceKind::Left => (room2.min.x - host.min.x).abs() < EPS,
		FaceKind::Right => (room2.max.x - host.max.x).abs() < EPS,
		FaceKind::Top | FaceKind::Bottom => true,
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
