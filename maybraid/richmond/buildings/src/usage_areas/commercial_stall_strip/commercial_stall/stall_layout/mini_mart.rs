//! MiniMart packing: passage clearances, office (+door), register, aisles, optional shelves.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use procedural_common::{
	aabb2_area, aabb3_to_plan, clamp_min_size2, intersects_aabb2, max_empty_rect2, plan_to_aabb3,
	Aabb2dPack, OptionalFaceBand, PlanAxes, PlanOpeningFace,
};

use crate::bedroom::shell::{face_rectangle, face_span_rectangle};
use crate::constraints::FaceKind;
use crate::fit::{Confines, FitError};
use crate::openings::{Opening, OpeningId};
use crate::paneling::{Rectangle, DEFAULT_PANEL_THICKNESS};

use crate::usage_areas::clearance::{PassageClearance, PlanHost, PASSAGE_CLEARANCE};

/// Scope prefix for [`OpeningId::scoped`] openings authored by MiniMart.
pub const SCOPE: &str = "mini_mart";

/// Inward clearance kept free in front of every customer passage (and office door).
pub const MINI_MART_PASSAGE_CLEARANCE: f32 = PASSAGE_CLEARANCE;
/// Office: longer plan axis must be at least this.
pub const MINI_MART_OFFICE_LONG_MIN: f32 = 3.0;
/// Office: shorter plan axis must be at least this.
pub const MINI_MART_OFFICE_SHORT_MIN: f32 = 2.0;
/// Register plan minimum (both axes).
pub const MINI_MART_REGISTER_MIN: f32 = 2.0;
/// Primary aisles plan minimum (both axes) — at least one region must meet this.
pub const MINI_MART_AISLES_MIN: f32 = 4.0;
/// Extra discontiguous aisle pockets may be this small (both axes).
pub const MINI_MART_AISLES_EXTRA_MIN: f32 = 2.0;
/// Shelf depth sample / pack range.
pub const MINI_MART_SHELF_DEPTH_MIN: f32 = 0.5;
pub const MINI_MART_SHELF_DEPTH_MAX: f32 = 1.0;
/// Minimum along-length for an opportunistic shelf segment.
pub const MINI_MART_SHELF_ALONG_MIN: f32 = 0.75;
/// Default place-rate for optional wall shelves.
pub const MINI_MART_SHELF_PLACE_RATE: f32 = 0.55;
/// Office door along-width range.
pub const MINI_MART_DOOR_WIDTH_MIN: f32 = 0.9;
pub const MINI_MART_DOOR_WIDTH_MAX: f32 = 1.2;
/// Office door clear opening height (leaves a header to the ceiling when host is taller).
pub const MINI_MART_DOOR_HEIGHT_MIN: f32 = 2.0;
pub const MINI_MART_DOOR_HEIGHT_MAX: f32 = 2.4;
/// Minimum header band above the office door.
pub const MINI_MART_DOOR_HEADER_MIN: f32 = 0.25;
/// Minimum office face contact when seeding.
pub const MINI_MART_OFFICE_CONTACT: f32 = 2.0;

/// Per free-wall shelf choice.
pub type MiniMartShelfChoice = OptionalFaceBand;

/// Free host wall snapshotted with its shelf choice.
#[derive(Debug, Clone, PartialEq)]
pub struct MiniMartShelfSpec {
	pub face: PlanOpeningFace,
	pub shelf: MiniMartShelfChoice,
}

/// Noise knobs consumed by [`MiniMartRegions::pack`].
#[derive(Debug, Clone, PartialEq)]
pub struct MiniMartRegions {
	pub office_area_target: f32,
	pub office_seed_depth: f32,
	pub office_along_t: f32,
	/// Plan area reserved for aisles (+ register floor) when clamping office grow.
	pub aisles_area_reserve: f32,
	pub door_width: f32,
	pub door_along_t: f32,
	/// Clear opening height for the office door (m); header fills to the stall ceiling.
	pub door_height: f32,
	pub register_along_t: f32,
	pub register_seed_depth: f32,
	pub shelves: Vec<MiniMartShelfSpec>,
}

/// Authored office-door passage + enclosure panels.
#[derive(Debug, Clone, PartialEq)]
pub struct MiniMartOfficeDoor {
	pub id: OpeningId,
	pub opening: Opening,
}

struct OfficeEnclosure {
	walls: Vec<Rectangle>,
	door_clear: Aabb2d,
	office_door: MiniMartOfficeDoor,
}

/// Geometry produced by [`MiniMartRegions::pack`].
#[derive(Debug, Clone, PartialEq)]
pub struct MiniMartPacked {
	pub office: Aabb3d,
	pub register: Aabb3d,
	/// One or more aisle pockets; the first meets [`MINI_MART_AISLES_MIN`].
	pub aisles: Vec<Aabb3d>,
	pub shelves: Vec<Aabb3d>,
	pub office_walls: Vec<Rectangle>,
	/// Passage through the office sales divider (tracked id + void bounds).
	pub office_door: MiniMartOfficeDoor,
}

impl MiniMartRegions {
	pub fn pack(&self, confines: &Confines) -> Result<MiniMartPacked, FitError> {
		let host3 = &confines.bounds;
		let host = aabb3_to_plan(host3, PlanAxes::XZ);
		let passage_faces = PassageClearance::collect_faces(confines, host);
		if passage_faces.is_empty() {
			return Err(FitError::TooSmall {
				reason: "mini mart passage",
			});
		}

		let mut clearances = PassageClearance::bands_std(host, &passage_faces);
		let (office2, seed_face) = self
			.pack_office(host, &clearances)
			.ok_or(FitError::TooSmall {
				reason: "mini mart office",
			})?;
		// Leave ≥1m beyond the sales face so the office door clearance can extrude.
		let office2 = Self::shrink_office_for_door_clearance(host, office2, seed_face).ok_or(
			FitError::TooSmall {
				reason: "mini mart office door",
			},
		)?;

		let enclosure = self
			.office_enclosure(host3, host, office2, seed_face)
			.ok_or(FitError::TooSmall {
				reason: "mini mart office door",
			})?;
		clearances.push(enclosure.door_clear);

		let register2 = self
			.pack_register(host, &passage_faces, &clearances, office2)
			.ok_or(FitError::TooSmall {
				reason: "mini mart register",
			})?;

		let aisles2 = self
			.pack_aisles(host, &clearances, office2, register2)
			.ok_or(FitError::TooSmall {
				reason: "mini mart aisles",
			})?;

		let shelves = self.pack_shelves(host, &clearances, office2, register2, &aisles2);

		Ok(MiniMartPacked {
			office: plan_to_aabb3(host3, office2, PlanAxes::XZ),
			register: plan_to_aabb3(host3, register2, PlanAxes::XZ),
			aisles: aisles2
				.into_iter()
				.map(|a| plan_to_aabb3(host3, a, PlanAxes::XZ))
				.collect(),
			shelves: shelves
				.into_iter()
				.map(|s| plan_to_aabb3(host3, s, PlanAxes::XZ))
				.collect(),
			office_walls: enclosure.walls,
			office_door: enclosure.office_door,
		})
	}

	fn office_dims_ok(office: Aabb2d) -> bool {
		let w = office.max.x - office.min.x;
		let d = office.max.y - office.min.y;
		let (long, short) = if w >= d { (w, d) } else { (d, w) };
		long + 1e-3 >= MINI_MART_OFFICE_LONG_MIN && short + 1e-3 >= MINI_MART_OFFICE_SHORT_MIN
	}

	/// Pull the sales face back so a [`MINI_MART_PASSAGE_CLEARANCE`] band fits in-host.
	fn shrink_office_for_door_clearance(
		host: Aabb2d,
		mut office: Aabb2d,
		seed_face: PlanOpeningFace,
	) -> Option<Aabb2d> {
		let need = MINI_MART_PASSAGE_CLEARANCE + 0.05;
		if seed_face.thru_is_x {
			if seed_face.inward_positive {
				office.max.x = office.max.x.min(host.max.x - need);
			} else {
				office.min.x = office.min.x.max(host.min.x + need);
			}
		} else if seed_face.inward_positive {
			office.max.y = office.max.y.min(host.max.y - need);
		} else {
			office.min.y = office.min.y.max(host.min.y + need);
		}
		if office.max.x - office.min.x < 1e-3 || office.max.y - office.min.y < 1e-3 {
			return None;
		}
		if !Self::office_dims_ok(office) {
			return None;
		}
		if seed_face.shared_border_len(office) + 1e-3 < MINI_MART_OFFICE_CONTACT {
			return None;
		}
		Some(office)
	}

	fn pack_office(
		&self,
		host: Aabb2d,
		clearances: &[Aabb2d],
	) -> Option<(Aabb2d, PlanOpeningFace)> {
		// Prefer walls without a clearance band glued to them, then longer faces.
		let mut candidates: Vec<PlanOpeningFace> = PlanHost::faces(host).into_iter().collect();
		candidates.sort_by(|a, b| {
			let blocked = |wall: PlanOpeningFace| {
				clearances.iter().any(|c| {
					wall.shared_border_len(*c) + 1e-3
						>= MINI_MART_OFFICE_CONTACT.min(wall.along_len())
				}) as u8
			};
			blocked(*a)
				.cmp(&blocked(*b))
				.then_with(|| {
					b.along_len()
						.partial_cmp(&a.along_len())
						.unwrap_or(std::cmp::Ordering::Equal)
				})
		});

		let contact = MINI_MART_OFFICE_CONTACT;
		let depth = self.office_seed_depth.max(MINI_MART_OFFICE_SHORT_MIN);
		let usable = aabb2_area(host);
		let sales_floor = (MINI_MART_AISLES_MIN * MINI_MART_AISLES_MIN)
			+ (MINI_MART_REGISTER_MIN * MINI_MART_REGISTER_MIN);
		let reserve = self
			.aisles_area_reserve
			.max(sales_floor)
			.min(usable * 0.75);
		let target = self
			.office_area_target
			.max(MINI_MART_OFFICE_LONG_MIN * MINI_MART_OFFICE_SHORT_MIN)
			.min((usable - reserve).max(MINI_MART_OFFICE_LONG_MIN * MINI_MART_OFFICE_SHORT_MIN));

		for face in candidates {
			if face.along_len() + 1e-3 < contact {
				continue;
			}
			let Some(seed) = face
				.seed_from_free(host, clearances, contact, depth, self.office_along_t)
				.or_else(|| face.seed_from_free(host, clearances, contact, depth, 0.5))
			else {
				continue;
			};
			let mut hard = clearances.to_vec();
			hard.push(face.outward_block(host));
			let grown = seed.grow_toward_area(host, &hard, target);
			let Some(office) = clamp_min_size2(
				grown,
				Vec2::new(MINI_MART_OFFICE_SHORT_MIN, MINI_MART_OFFICE_SHORT_MIN),
			) else {
				continue;
			};
			if !Self::office_dims_ok(office) {
				continue;
			}
			if face.shared_border_len(office) + 1e-3 < contact {
				continue;
			}
			return Some((office, face));
		}
		None
	}

	/// Walls on every office side that is not already on the host shell, including a
	/// sales-face divider with a tracked [`Opening::passage`] door (below-ceiling).
	fn office_enclosure(
		&self,
		host3: &Aabb3d,
		host: Aabb2d,
		office2: Aabb2d,
		seed_face: PlanOpeningFace,
	) -> Option<OfficeEnclosure> {
		let office3 = plan_to_aabb3(host3, office2, PlanAxes::XZ);
		let sales_face = Self::sales_face_kind(seed_face);
		let divider_thru = if seed_face.thru_is_x {
			if seed_face.inward_positive {
				office2.max.x
			} else {
				office2.min.x
			}
		} else if seed_face.inward_positive {
			office2.max.y
		} else {
			office2.min.y
		};

		let door_face = PlanOpeningFace {
			thru_is_x: seed_face.thru_is_x,
			thru: divider_thru,
			along0: if seed_face.thru_is_x {
				office2.min.y
			} else {
				office2.min.x
			},
			along1: if seed_face.thru_is_x {
				office2.max.y
			} else {
				office2.max.x
			},
			inward_positive: seed_face.inward_positive,
		};
		let door_face = door_face.clip_to_host(host)?;
		let along_len = door_face.along_len();
		let max_door = (along_len - 0.4).max(along_len * 0.5);
		let door_w = self
			.door_width
			.clamp(MINI_MART_DOOR_WIDTH_MIN, MINI_MART_DOOR_WIDTH_MAX)
			.min(max_door)
			.min(along_len * 0.85);
		if door_w + 1e-3 < MINI_MART_DOOR_WIDTH_MIN.min(along_len * 0.45) {
			return None;
		}
		if along_len + 1e-3 < door_w {
			return None;
		}

		let (door0, door1, door_clear) = {
			let mut placed = None;
			for &t in &[self.door_along_t, 0.5, 0.25, 0.75] {
				let Some((d0, d1)) = place_along(door_face.along0, door_face.along1, door_w, t)
				else {
					continue;
				};
				if let Some(clear) = door_face.band(host, door_w, MINI_MART_PASSAGE_CLEARANCE, t) {
					placed = Some((d0, d1, clear));
					break;
				}
			}
			placed?
		};

		let host_h = (host3.max.y - host3.min.y).max(1.0);
		let door_h = self
			.door_height
			.clamp(MINI_MART_DOOR_HEIGHT_MIN, MINI_MART_DOOR_HEIGHT_MAX)
			.min((host_h - MINI_MART_DOOR_HEADER_MIN).max(MINI_MART_DOOR_HEIGHT_MIN));
		let u0 = ((door0 - door_face.along0) / along_len).clamp(0.0, 1.0);
		let u1 = ((door1 - door_face.along0) / along_len).clamp(0.0, 1.0);

		let mut walls = Vec::new();
		// Solid walls on office sides that are not already the host shell.
		for face in [
			FaceKind::Front,
			FaceKind::Back,
			FaceKind::Left,
			FaceKind::Right,
		] {
			if face == sales_face {
				continue;
			}
			if Self::office_side_on_host(office2, host, face) {
				continue;
			}
			if let Some(r) = face_rectangle(&office3, face, DEFAULT_PANEL_THICKNESS) {
				walls.push(r);
			}
		}

		// Sales divider: full-height jambs + header above a below-ceiling door.
		let mut door_panels = 0usize;
		if u0 > 0.02 {
			if let Some(r) =
				face_span_rectangle(&office3, sales_face, 0.0, u0, DEFAULT_PANEL_THICKNESS)
			{
				walls.push(r);
				door_panels += 1;
			}
		}
		if u1 < 0.98 {
			if let Some(r) =
				face_span_rectangle(&office3, sales_face, u1, 1.0, DEFAULT_PANEL_THICKNESS)
			{
				walls.push(r);
				door_panels += 1;
			}
		}
		if door_h + MINI_MART_DOOR_HEADER_MIN <= host_h + 1e-3 {
			let omin = Vec3::from(office3.min);
			let omax = Vec3::from(office3.max);
			let header_aabb = Aabb3d::from_min_max(
				Vec3::new(omin.x, omin.y + door_h, omin.z),
				omax,
			);
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

		let door_id = OpeningId::scoped(SCOPE, "office_door", "0");
		let door_bounds = Self::office_door_bounds(
			host3,
			divider_thru,
			seed_face.thru_is_x,
			door0,
			door1,
			door_h,
		);
		Some(OfficeEnclosure {
			walls,
			door_clear,
			office_door: MiniMartOfficeDoor {
				id: door_id,
				opening: Opening::passage(door_bounds),
			},
		})
	}

	fn office_door_bounds(
		host3: &Aabb3d,
		divider_thru: f32,
		thru_is_x: bool,
		door0: f32,
		door1: f32,
		door_h: f32,
	) -> Aabb3d {
		let y0 = host3.min.y;
		let y1 = y0 + door_h.max(0.5);
		let band = (DEFAULT_PANEL_THICKNESS * 0.5 + 0.08).max(0.12);
		let a0 = door0.min(door1);
		let a1 = door0.max(door1);
		if thru_is_x {
			Aabb3d::from_min_max(
				Vec3::new(divider_thru - band, y0, a0),
				Vec3::new(divider_thru + band, y1, a1),
			)
		} else {
			Aabb3d::from_min_max(
				Vec3::new(a0, y0, divider_thru - band),
				Vec3::new(a1, y1, divider_thru + band),
			)
		}
	}

	fn sales_face_kind(seed_face: PlanOpeningFace) -> FaceKind {
		match (seed_face.thru_is_x, seed_face.inward_positive) {
			(true, true) => FaceKind::Right,
			(true, false) => FaceKind::Left,
			(false, true) => FaceKind::Back,
			(false, false) => FaceKind::Front,
		}
	}

	fn office_side_on_host(office2: Aabb2d, host: Aabb2d, face: FaceKind) -> bool {
		const EPS: f32 = 0.08;
		match face {
			FaceKind::Front => (office2.min.y - host.min.y).abs() < EPS,
			FaceKind::Back => (office2.max.y - host.max.y).abs() < EPS,
			FaceKind::Left => (office2.min.x - host.min.x).abs() < EPS,
			FaceKind::Right => (office2.max.x - host.max.x).abs() < EPS,
			FaceKind::Top | FaceKind::Bottom => true,
		}
	}

	fn pack_register(
		&self,
		host: Aabb2d,
		passage_faces: &[PlanOpeningFace],
		clearances: &[Aabb2d],
		office2: Aabb2d,
	) -> Option<Aabb2d> {
		let min = MINI_MART_REGISTER_MIN;
		let depth = self.register_seed_depth.max(min);
		// Passage clearances abut the inset seed face — use them only as open-overlap
		// excludes (not free-segment blockers on that face).
		let mut hard = clearances.to_vec();
		hard.push(office2);

		let mut order: Vec<usize> = (0..passage_faces.len()).collect();
		order.sort_by(|a, b| {
			passage_faces[*b]
				.along_len()
				.partial_cmp(&passage_faces[*a].along_len())
				.unwrap_or(std::cmp::Ordering::Equal)
		});

		for &i in &order {
			let face = passage_faces[i];
			let inset = inset_face(face, MINI_MART_PASSAGE_CLEARANCE);
			for &t in &[self.register_along_t, 0.5, 0.0, 1.0] {
				let Some(seed) = inset.seed(host, min, depth, t) else {
					continue;
				};
				if !seed.is_clear_of(&hard) {
					continue;
				}
				let grown = seed.grow_toward_area(host, &hard, min * min);
				let Some(reg) = clamp_min_size2(grown, Vec2::splat(min)) else {
					continue;
				};
				if !reg.is_clear_of(&hard) {
					continue;
				}
				return Some(reg);
			}
		}

		// Fallback: largest empty ≥2×2 in the sales floor.
		let seed = max_empty_rect2(host, &hard)?;
		let grown = seed.grow_into(host, &hard);
		let reg = clamp_min_size2(grown, Vec2::splat(min))?;
		reg.is_clear_of(&hard).then_some(reg)
	}

	fn pack_aisles(
		&self,
		host: Aabb2d,
		clearances: &[Aabb2d],
		office2: Aabb2d,
		register2: Aabb2d,
	) -> Option<Vec<Aabb2d>> {
		let mut hard = clearances.to_vec();
		hard.push(office2);
		hard.push(register2);
		let seed = max_empty_rect2(host, &hard)?;
		let grown = seed.grow_into(host, &hard);
		let primary = clamp_min_size2(grown, Vec2::splat(MINI_MART_AISLES_MIN))?;
		let mut aisles = vec![primary];
		hard.push(primary);

		// Fill leftover pockets with smaller discontiguous aisle regions.
		for _ in 0..6 {
			let Some(seed) = max_empty_rect2(host, &hard) else {
				break;
			};
			let grown = seed.grow_into(host, &hard);
			let Some(extra) = clamp_min_size2(grown, Vec2::splat(MINI_MART_AISLES_EXTRA_MIN)) else {
				break;
			};
			if !extra.is_clear_of(&hard) {
				break;
			}
			aisles.push(extra);
			hard.push(extra);
		}
		Some(aisles)
	}

	fn pack_shelves(
		&self,
		host: Aabb2d,
		clearances: &[Aabb2d],
		office2: Aabb2d,
		register2: Aabb2d,
		aisles: &[Aabb2d],
	) -> Vec<Aabb2d> {
		let mut hard = clearances.to_vec();
		hard.push(office2);
		hard.push(register2);
		hard.extend_from_slice(aisles);
		let mut out = Vec::new();

		// Sampled shelf choices first.
		for spec in &self.shelves {
			let depth = spec
				.shelf
				.depth
				.clamp(MINI_MART_SHELF_DEPTH_MIN, MINI_MART_SHELF_DEPTH_MAX);
			let choice = OptionalFaceBand {
				place: spec.shelf.place,
				along: spec.shelf.along,
				depth,
				along_t: spec.shelf.along_t,
			};
			let Some(band) = choice.resolve(host, spec.face) else {
				continue;
			};
			if !band.is_clear_of(&hard) {
				continue;
			}
			if out.iter().any(|s| intersects_aabb2(band, *s)) {
				continue;
			}
			out.push(band);
			hard.push(band);
		}

		// Opportunistic fill: more discontiguous shelves on free-wall free segments.
		let free_faces: Vec<PlanOpeningFace> = self.shelves.iter().map(|s| s.face).collect();
		for face in free_faces {
			let depth = self
				.shelves
				.iter()
				.find(|s| PlanHost::same_wall(s.face, face))
				.map(|s| {
					s.shelf
						.depth
						.clamp(MINI_MART_SHELF_DEPTH_MIN, MINI_MART_SHELF_DEPTH_MAX)
				})
				.unwrap_or(0.75);
			// Recompute free segments as shelves are accepted.
			for _ in 0..8 {
				let Some((seg0, seg1)) = face.longest_free_segment(&hard, MINI_MART_SHELF_ALONG_MIN)
				else {
					break;
				};
				let avail = seg1 - seg0;
				let seg_face = PlanOpeningFace {
					along0: seg0,
					along1: seg1,
					..face
				};
				let Some(band) = seg_face.band(host, avail, depth, 0.5) else {
					break;
				};
				if !band.is_clear_of(&hard) {
					break;
				}
				if out.iter().any(|s| intersects_aabb2(band, *s)) {
					break;
				}
				out.push(band);
				hard.push(band);
			}
		}
		out
	}
}

fn inset_face(face: PlanOpeningFace, depth: f32) -> PlanOpeningFace {
	let thru = if face.inward_positive {
		face.thru + depth
	} else {
		face.thru - depth
	};
	PlanOpeningFace { thru, ..face }
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
