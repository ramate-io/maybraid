//! Noise-driven bedroom fill: bed-first, then more concepts under fill budgets.
//!
//! - **Spaciousness** scales each concept's base footprint (claims more floor per item).
//! - **Occupancy** is the maximum fraction of room floor area to allocate; filling
//!   stops so roughly `1 - occupancy` stays empty.
//! - Layouts avoid circulation exclusion volumes.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::constraints::circulation::aabb3d_intersects;
use crate::constraints::face::FACE_EPS;
use crate::CellConstraints;

/// Authoring knobs for how densely a bedroom is packed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BedroomFillParams {
	/// Multiplier on each concept's base XZ bounds (`1.0` = nominal). Higher →
	/// each bed/closet/… reserves more floor (“renders space”).
	pub spaciousness: f32,
	/// Maximum fraction of room floor area to allocate. Filling stops once
	/// occupied / total would exceed this, leaving about `1.0 - occupancy` empty.
	pub occupancy: f32,
}

impl Default for BedroomFillParams {
	fn default() -> Self {
		Self {
			spaciousness: 1.0,
			occupancy: 0.55,
		}
	}
}

impl BedroomFillParams {
	pub fn clamped(self) -> Self {
		Self {
			spaciousness: self.spaciousness.max(1e-3),
			occupancy: self.occupancy.clamp(0.05, 1.0),
		}
	}
}

/// Allocated child AABBs inside a room footprint (zero or more of each concept).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BedroomLayout {
	pub beds: Vec<Aabb3d>,
	pub nightstands: Vec<Aabb3d>,
	pub closets: Vec<Aabb3d>,
	pub ensuites: Vec<Aabb3d>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Concept {
	Bed,
	Nightstand,
	Closet,
	Ensuite,
}

impl BedroomLayout {
	/// Fill `constraints` using `noise` and fill budgets.
	///
	/// Always places at least one bed (when any valid bed pose exists), then
	/// greedily adds nightstands / closets / ensuites / further beds until
	/// occupancy is met or no candidate fits.
	pub fn fit(
		constraints: &CellConstraints,
		noise: NoiseParams,
		params: BedroomFillParams,
	) -> Self {
		let params = params.clamped();
		let room = constraints.aabb;
		let exclusions = constraints.circulation_exclusion_zones();
		let cfg = NoiseConfig::new(noise);
		let room_area = xz_area(&room).max(FACE_EPS);

		let mut layout = Self::default();
		let mut occupied_area = 0.0_f32;

		// 1) Mandatory first bed.
		if let Some(bed) = place_bed(&room, &exclusions, &layout, &cfg, 0, params.spaciousness) {
			occupied_area += xz_area(&bed);
			layout.beds.push(bed);
		}

		// 2) Further concepts while under occupancy.
		for step in 1..24u32 {
			if occupied_area / room_area >= params.occupancy {
				break;
			}
			let concept = pick_concept(&cfg, step, layout.beds.len());
			let Some(candidate) =
				place_concept(concept, &room, &exclusions, &layout, &cfg, step, params.spaciousness)
			else {
				continue;
			};
			let add = xz_area(&candidate);
			if (occupied_area + add) / room_area > params.occupancy + FACE_EPS {
				continue;
			}
			occupied_area += add;
			match concept {
				Concept::Bed => layout.beds.push(candidate),
				Concept::Nightstand => layout.nightstands.push(candidate),
				Concept::Closet => layout.closets.push(candidate),
				Concept::Ensuite => layout.ensuites.push(candidate),
			}
		}

		layout
	}

	fn all_aabbs(&self) -> impl Iterator<Item = &Aabb3d> {
		self.beds
			.iter()
			.chain(self.nightstands.iter())
			.chain(self.closets.iter())
			.chain(self.ensuites.iter())
	}

	#[cfg(test)]
	fn exclusion_hits(&self, exclusions: &[Aabb3d]) -> u32 {
		let mut hits = 0u32;
		for child in self.all_aabbs() {
			for zone in exclusions {
				if aabb3d_intersects(child, zone) {
					hits += 1;
				}
			}
		}
		hits
	}

	fn collides_existing(&self, candidate: &Aabb3d) -> bool {
		self.all_aabbs().any(|a| aabb3d_intersects(a, candidate))
	}
}

fn xz_area(a: &Aabb3d) -> f32 {
	let e = a.max - a.min;
	e.x.max(0.0) * e.z.max(0.0)
}

fn pick_concept(noise: &NoiseConfig, step: u32, bed_count: usize) -> Concept {
	let t = noise.sample_unit_4d(step as f32, 0.0, 0.0, 10.0);
	// Prefer nightstand after the first bed; allow more beds in larger fills.
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

fn place_concept(
	concept: Concept,
	room: &Aabb3d,
	exclusions: &[Aabb3d],
	layout: &BedroomLayout,
	noise: &NoiseConfig,
	salt: u32,
	spaciousness: f32,
) -> Option<Aabb3d> {
	match concept {
		Concept::Bed => place_bed(room, exclusions, layout, noise, salt, spaciousness),
		Concept::Nightstand => place_nightstand(room, exclusions, layout, noise, salt, spaciousness),
		Concept::Closet => place_closet(room, exclusions, layout, noise, salt, spaciousness),
		Concept::Ensuite => place_ensuite(room, exclusions, layout, noise, salt, spaciousness),
	}
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

fn fits(
	candidate: &Aabb3d,
	room: &Aabb3d,
	exclusions: &[Aabb3d],
	layout: &BedroomLayout,
) -> bool {
	if candidate.min.x < room.min.x - FACE_EPS
		|| candidate.min.y < room.min.y - FACE_EPS
		|| candidate.min.z < room.min.z - FACE_EPS
		|| candidate.max.x > room.max.x + FACE_EPS
		|| candidate.max.y > room.max.y + FACE_EPS
		|| candidate.max.z > room.max.z + FACE_EPS
	{
		return false;
	}
	if exclusions.iter().any(|z| aabb3d_intersects(candidate, z)) {
		return false;
	}
	!layout.collides_existing(candidate)
}

fn place_bed(
	room: &Aabb3d,
	exclusions: &[Aabb3d],
	layout: &BedroomLayout,
	noise: &NoiseConfig,
	salt: u32,
	spaciousness: f32,
) -> Option<Aabb3d> {
	let extent = base_bed_extent(spaciousness);
	let size = room.max - room.min;
	if extent.x > size.x + FACE_EPS || extent.z > size.z + FACE_EPS {
		return None;
	}
	for attempt in 0..12u32 {
		let u = noise.sample_unit_4d(salt as f32, attempt as f32, 0.0, 20.0);
		let v = noise.sample_unit_4d(salt as f32, attempt as f32, 0.0, 21.0);
		let max_u = (size.x - extent.x).max(0.0);
		let max_v = (size.z - extent.z).max(0.0);
		let min = Vec3::new(
			room.min.x + u * max_u,
			room.min.y,
			room.min.z + v * max_v,
		);
		let candidate = Aabb3d::from_min_max(min, min + extent);
		if fits(&candidate, room, exclusions, layout) {
			return Some(candidate);
		}
	}
	None
}

fn place_nightstand(
	room: &Aabb3d,
	exclusions: &[Aabb3d],
	layout: &BedroomLayout,
	noise: &NoiseConfig,
	salt: u32,
	spaciousness: f32,
) -> Option<Aabb3d> {
	let extent = base_nightstand_extent(spaciousness);
	let gap = 0.08_f32 * spaciousness;
	// Prefer beside an existing bed; fall back to free floor samples.
	for (bi, bed) in layout.beds.iter().enumerate() {
		let side = noise.sample_unit_4d(salt as f32, bi as f32, 0.0, 30.0) >= 0.5;
		let x = if side {
			bed.max.x + gap
		} else {
			bed.min.x - gap - extent.x
		};
		let z = bed.min.z + (bed.max.z - bed.min.z) * 0.5 - extent.z * 0.5;
		let min = Vec3::new(x, room.min.y, z);
		let candidate = Aabb3d::from_min_max(min, min + extent);
		if fits(&candidate, room, exclusions, layout) {
			return Some(candidate);
		}
	}
	place_free_extent(room, exclusions, layout, noise, salt, extent, 31.0)
}

fn place_closet(
	room: &Aabb3d,
	exclusions: &[Aabb3d],
	layout: &BedroomLayout,
	noise: &NoiseConfig,
	salt: u32,
	spaciousness: f32,
) -> Option<Aabb3d> {
	let depth = base_closet_depth(spaciousness);
	let length = base_closet_length(spaciousness);
	let y0 = room.min.y;
	let y1 = room.max.y;
	let on_front = noise.sample_unit_4d(salt as f32, 0.0, 0.0, 40.0) >= 0.5;
	let size = room.max - room.min;
	if length > size.x + FACE_EPS || depth > size.z + FACE_EPS {
		return None;
	}
	for attempt in 0..8u32 {
		let u = noise.sample_unit_4d(salt as f32, attempt as f32, 0.0, 41.0);
		let max_u = (size.x - length).max(0.0);
		let x0 = room.min.x + u * max_u;
		let candidate = if on_front {
			Aabb3d::from_min_max(
				Vec3::new(x0, y0, room.min.z),
				Vec3::new(x0 + length, y1, room.min.z + depth),
			)
		} else {
			Aabb3d::from_min_max(
				Vec3::new(x0, y0, room.max.z - depth),
				Vec3::new(x0 + length, y1, room.max.z),
			)
		};
		if fits(&candidate, room, exclusions, layout) {
			return Some(candidate);
		}
	}
	None
}

fn place_ensuite(
	room: &Aabb3d,
	exclusions: &[Aabb3d],
	layout: &BedroomLayout,
	noise: &NoiseConfig,
	salt: u32,
	spaciousness: f32,
) -> Option<Aabb3d> {
	let depth = base_ensuite_depth(spaciousness);
	let length = base_ensuite_length(spaciousness);
	let y0 = room.min.y;
	let y1 = room.max.y;
	let on_right = noise.sample_unit_4d(salt as f32, 0.0, 0.0, 50.0) >= 0.5;
	let size = room.max - room.min;
	if length > size.z + FACE_EPS || depth > size.x + FACE_EPS {
		return None;
	}
	for attempt in 0..8u32 {
		let u = noise.sample_unit_4d(salt as f32, attempt as f32, 0.0, 51.0);
		let max_u = (size.z - length).max(0.0);
		let z0 = room.min.z + u * max_u;
		let candidate = if on_right {
			Aabb3d::from_min_max(
				Vec3::new(room.max.x - depth, y0, z0),
				Vec3::new(room.max.x, y1, z0 + length),
			)
		} else {
			Aabb3d::from_min_max(
				Vec3::new(room.min.x, y0, z0),
				Vec3::new(room.min.x + depth, y1, z0 + length),
			)
		};
		if fits(&candidate, room, exclusions, layout) {
			return Some(candidate);
		}
	}
	None
}

fn place_free_extent(
	room: &Aabb3d,
	exclusions: &[Aabb3d],
	layout: &BedroomLayout,
	noise: &NoiseConfig,
	salt: u32,
	extent: Vec3,
	channel: f32,
) -> Option<Aabb3d> {
	let size = room.max - room.min;
	if extent.x > size.x + FACE_EPS || extent.z > size.z + FACE_EPS {
		return None;
	}
	for attempt in 0..10u32 {
		let u = noise.sample_unit_4d(salt as f32, attempt as f32, 0.0, channel);
		let v = noise.sample_unit_4d(salt as f32, attempt as f32, 0.0, channel + 1.0);
		let max_u = (size.x - extent.x).max(0.0);
		let max_v = (size.z - extent.z).max(0.0);
		let min = Vec3::new(
			room.min.x + u * max_u,
			room.min.y,
			room.min.z + v * max_v,
		);
		let candidate = Aabb3d::from_min_max(min, min + extent);
		if fits(&candidate, room, exclusions, layout) {
			return Some(candidate);
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::{bounding::Aabb2d, Vec2};
	use crate::constraints::{CirculationEntry, CirculationRequestStatus};

	#[test]
	fn fit_always_places_at_least_one_bed() -> anyhow::Result<()> {
		let cell = CellConstraints::cell_owned(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(5.0, 3.0, 5.0),
		));
		let layout = BedroomLayout::fit(
			&cell,
			NoiseParams {
				seed: 7,
				..NoiseParams::default()
			},
			BedroomFillParams::default(),
		);
		assert!(!layout.beds.is_empty());
		Ok(())
	}

	#[test]
	fn higher_occupancy_allows_more_items() -> anyhow::Result<()> {
		let cell = CellConstraints::cell_owned(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(8.0, 3.0, 8.0),
		));
		let sparse = BedroomLayout::fit(
			&cell,
			NoiseParams {
				seed: 3,
				..NoiseParams::default()
			},
			BedroomFillParams {
				spaciousness: 1.0,
				occupancy: 0.2,
			},
		);
		let dense = BedroomLayout::fit(
			&cell,
			NoiseParams {
				seed: 3,
				..NoiseParams::default()
			},
			BedroomFillParams {
				spaciousness: 1.0,
				occupancy: 0.85,
			},
		);
		let sparse_n = sparse.beds.len()
			+ sparse.nightstands.len()
			+ sparse.closets.len()
			+ sparse.ensuites.len();
		let dense_n = dense.beds.len()
			+ dense.nightstands.len()
			+ dense.closets.len()
			+ dense.ensuites.len();
		assert!(dense_n >= sparse_n);
		Ok(())
	}

	#[test]
	fn spaciousness_inflates_bed_footprint() -> anyhow::Result<()> {
		let cell = CellConstraints::cell_owned(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(10.0, 3.0, 10.0),
		));
		let tight = BedroomLayout::fit(
			&cell,
			NoiseParams {
				seed: 1,
				..NoiseParams::default()
			},
			BedroomFillParams {
				spaciousness: 1.0,
				occupancy: 0.15,
			},
		);
		let roomy = BedroomLayout::fit(
			&cell,
			NoiseParams {
				seed: 1,
				..NoiseParams::default()
			},
			BedroomFillParams {
				spaciousness: 1.4,
				occupancy: 0.15,
			},
		);
		let t = tight.beds[0].max - tight.beds[0].min;
		let r = roomy.beds[0].max - roomy.beds[0].min;
		assert!(r.x > t.x + 1e-3);
		assert!(r.z > t.z + 1e-3);
		Ok(())
	}

	#[test]
	fn fit_avoids_front_door_exclusion() -> anyhow::Result<()> {
		let mut cell = CellConstraints::cell_owned(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(5.0, 3.0, 5.0),
		));
		cell.circulation.front = Some(CirculationEntry(vec![(
			Aabb2d {
				min: Vec2::new(0.4, 0.0),
				max: Vec2::new(0.55, 0.9),
			},
			vec![CirculationRequestStatus::Required],
		)]));
		let layout = BedroomLayout::fit(
			&cell,
			NoiseParams {
				seed: 42,
				..NoiseParams::default()
			},
			BedroomFillParams::default(),
		);
		let zones = cell.circulation_exclusion_zones();
		assert!(!zones.is_empty());
		assert_eq!(layout.exclusion_hits(&zones), 0);
		assert!(!layout.beds.is_empty());
		Ok(())
	}
}
