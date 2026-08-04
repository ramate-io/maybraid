//! Parts stall packing: passage clearances, office (+door), parts pockets.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use procedural_common::{
	aabb2_area, aabb3_to_plan, clamp_min_size2, max_empty_rect2, plan_to_aabb3, Aabb2dPack,
	PlanAxes, PlanOpeningFace,
};

use crate::bedroom::shell::{face_rectangle, face_span_rectangle};
use crate::constraints::FaceKind;
use crate::fit::{Confines, FitError};
use crate::openings::{Opening, OpeningId};
use crate::paneling::{Rectangle, DEFAULT_PANEL_THICKNESS};

use super::clearance::{PassageClearance, StallPlanHost, STALL_PASSAGE_CLEARANCE};

/// Scope prefix for [`OpeningId::scoped`] openings authored by Parts.
pub const SCOPE: &str = "parts_stall";

pub const PARTS_OFFICE_MIN: f32 = 2.0;
pub const PARTS_OFFICE_CONTACT: f32 = 2.0;
pub const PARTS_REGION_MIN: f32 = 2.0;
pub const PARTS_REGION_EXTRA_MIN: f32 = 1.5;
pub const PARTS_DOOR_WIDTH_MIN: f32 = 0.9;
pub const PARTS_DOOR_WIDTH_MAX: f32 = 1.2;
pub const PARTS_DOOR_HEIGHT_MIN: f32 = 2.0;
pub const PARTS_DOOR_HEIGHT_MAX: f32 = 2.4;
pub const PARTS_DOOR_HEADER_MIN: f32 = 0.25;

#[derive(Debug, Clone, PartialEq)]
pub struct PartsOfficeDoor {
	pub id: OpeningId,
	pub opening: Opening,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartsRegions {
	pub office_area_target: f32,
	pub office_seed_depth: f32,
	pub office_along_t: f32,
	pub parts_area_reserve: f32,
	pub door_width: f32,
	pub door_along_t: f32,
	pub door_height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartsPacked {
	pub office: Aabb3d,
	pub parts: Vec<Aabb3d>,
	pub office_walls: Vec<Rectangle>,
	pub office_door: PartsOfficeDoor,
}

struct OfficeEnclosure {
	walls: Vec<Rectangle>,
	door_clear: Aabb2d,
	office_door: PartsOfficeDoor,
}

impl PartsRegions {
	pub fn pack(&self, confines: &Confines) -> Result<PartsPacked, FitError> {
		let host3 = &confines.bounds;
		let host = aabb3_to_plan(host3, PlanAxes::XZ);
		let passage_faces = PassageClearance::collect_faces(confines, host);
		if passage_faces.is_empty() {
			return Err(FitError::TooSmall {
				reason: "parts passage",
			});
		}

		let mut clearances = PassageClearance::bands_std(host, &passage_faces);
		let (office2, seed_face) = self
			.pack_office(host, &clearances)
			.ok_or(FitError::TooSmall {
				reason: "parts office",
			})?;
		let office2 = Self::shrink_office_for_door_clearance(host, office2, seed_face).ok_or(
			FitError::TooSmall {
				reason: "parts office door",
			},
		)?;

		let enclosure = self
			.office_enclosure(host3, host, office2, seed_face)
			.ok_or(FitError::TooSmall {
				reason: "parts office door",
			})?;
		clearances.push(enclosure.door_clear);

		let parts = self
			.pack_parts(host, &clearances, office2)
			.ok_or(FitError::TooSmall {
				reason: "parts region",
			})?;

		Ok(PartsPacked {
			office: plan_to_aabb3(host3, office2, PlanAxes::XZ),
			parts: parts
				.into_iter()
				.map(|p| plan_to_aabb3(host3, p, PlanAxes::XZ))
				.collect(),
			office_walls: enclosure.walls,
			office_door: enclosure.office_door,
		})
	}

	fn office_dims_ok(office: Aabb2d) -> bool {
		let w = office.max.x - office.min.x;
		let d = office.max.y - office.min.y;
		w + 1e-3 >= PARTS_OFFICE_MIN && d + 1e-3 >= PARTS_OFFICE_MIN
	}

	fn shrink_office_for_door_clearance(
		host: Aabb2d,
		mut office: Aabb2d,
		seed_face: PlanOpeningFace,
	) -> Option<Aabb2d> {
		let need = STALL_PASSAGE_CLEARANCE + 0.05;
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
		if seed_face.shared_border_len(office) + 1e-3 < PARTS_OFFICE_CONTACT {
			return None;
		}
		Some(office)
	}

	fn pack_office(
		&self,
		host: Aabb2d,
		clearances: &[Aabb2d],
	) -> Option<(Aabb2d, PlanOpeningFace)> {
		let mut candidates: Vec<PlanOpeningFace> = StallPlanHost::faces(host).into_iter().collect();
		candidates.sort_by(|a, b| {
			let blocked = |wall: PlanOpeningFace| {
				clearances.iter().any(|c| {
					wall.shared_border_len(*c) + 1e-3
						>= PARTS_OFFICE_CONTACT.min(wall.along_len())
				}) as u8
			};
			blocked(*a).cmp(&blocked(*b)).then_with(|| {
				b.along_len()
					.partial_cmp(&a.along_len())
					.unwrap_or(std::cmp::Ordering::Equal)
			})
		});

		let contact = PARTS_OFFICE_CONTACT;
		let depth = self.office_seed_depth.max(PARTS_OFFICE_MIN);
		let usable = aabb2_area(host);
		let reserve = self
			.parts_area_reserve
			.max(PARTS_REGION_MIN * PARTS_REGION_MIN)
			.min(usable * 0.7);
		let target = self
			.office_area_target
			.max(PARTS_OFFICE_MIN * PARTS_OFFICE_MIN)
			.min((usable - reserve).max(PARTS_OFFICE_MIN * PARTS_OFFICE_MIN));

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
			let Some(office) = clamp_min_size2(grown, Vec2::splat(PARTS_OFFICE_MIN)) else {
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
			.clamp(PARTS_DOOR_WIDTH_MIN, PARTS_DOOR_WIDTH_MAX)
			.min(max_door)
			.min(along_len * 0.85);
		if door_w + 1e-3 < PARTS_DOOR_WIDTH_MIN.min(along_len * 0.45) || along_len + 1e-3 < door_w {
			return None;
		}

		let (door0, door1, door_clear) = {
			let mut placed = None;
			for &t in &[self.door_along_t, 0.5, 0.25, 0.75] {
				let Some((d0, d1)) = place_along(door_face.along0, door_face.along1, door_w, t)
				else {
					continue;
				};
				if let Some(clear) = door_face.band(host, door_w, STALL_PASSAGE_CLEARANCE, t) {
					placed = Some((d0, d1, clear));
					break;
				}
			}
			placed?
		};

		let host_h = (host3.max.y - host3.min.y).max(1.0);
		let door_h = self
			.door_height
			.clamp(PARTS_DOOR_HEIGHT_MIN, PARTS_DOOR_HEIGHT_MAX)
			.min((host_h - PARTS_DOOR_HEADER_MIN).max(PARTS_DOOR_HEIGHT_MIN));
		let u0 = ((door0 - door_face.along0) / along_len).clamp(0.0, 1.0);
		let u1 = ((door1 - door_face.along0) / along_len).clamp(0.0, 1.0);

		let mut walls = Vec::new();
		for face in [
			FaceKind::Front,
			FaceKind::Back,
			FaceKind::Left,
			FaceKind::Right,
		] {
			if face == sales_face || Self::office_side_on_host(office2, host, face) {
				continue;
			}
			if let Some(r) = face_rectangle(&office3, face, DEFAULT_PANEL_THICKNESS) {
				walls.push(r);
			}
		}

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
		if door_h + PARTS_DOOR_HEADER_MIN <= host_h + 1e-3 {
			let omin = Vec3::from(office3.min);
			let omax = Vec3::from(office3.max);
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

		Some(OfficeEnclosure {
			walls,
			door_clear,
			office_door: PartsOfficeDoor {
				id: OpeningId::scoped(SCOPE, "office_door", "0"),
				opening: Opening::passage(door_bounds),
			},
		})
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

	fn pack_parts(
		&self,
		host: Aabb2d,
		clearances: &[Aabb2d],
		office2: Aabb2d,
	) -> Option<Vec<Aabb2d>> {
		let mut hard = clearances.to_vec();
		hard.push(office2);
		let seed = max_empty_rect2(host, &hard)?;
		let grown = seed.grow_into(host, &hard);
		let primary = clamp_min_size2(grown, Vec2::splat(PARTS_REGION_MIN))?;
		let mut parts = vec![primary];
		hard.push(primary);
		for _ in 0..6 {
			let Some(seed) = max_empty_rect2(host, &hard) else {
				break;
			};
			let grown = seed.grow_into(host, &hard);
			let Some(extra) = clamp_min_size2(grown, Vec2::splat(PARTS_REGION_EXTRA_MIN)) else {
				break;
			};
			if !extra.is_clear_of(&hard) {
				break;
			}
			parts.push(extra);
			hard.push(extra);
		}
		Some(parts)
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
