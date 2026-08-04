//! Axis-aligned volume packing: opening faces, face bands, area-targeted grow.
//!
//! Complements [`crate::bounds`] (max-empty / inflate primitives) with higher-level
//! placement on plan AABBs.
//!
//! # Core (used by commercial bites packing)
//! - [`PlanOpeningFace`] — long face of a passage into a host
//! - [`OptionalFaceBand`] — optional counter/furniture band on a face
//! - [`Aabb2dPack`] — grow / bipartition helpers on [`Aabb2d`]
//!
//! # Reserved generalizations
//! These stay in the API for multi-region / guillotine-style follow-ups, but are
//! not currently wired into production bites packing:
//! - [`Aabb2dPack::bipartition_by_area`] — single axis area split of a host
//! - [`Aabb2dPack::grow_pair`] — alternate full grow of two seeds
//! - [`Aabb2dPack::grow_pair_toward_areas`] — area-capped pair grow, then scrap fill
//! - [`Aabb2dPack::is_clear_of`] — open-overlap clear test for a candidate band

use bevy_math::bounding::Aabb2d;
use bevy_math::Vec2;

use crate::bounds::{aabb2_area, grow_aabb2, intersects_aabb2};

const EPS: f32 = 1e-4;
const MIN_SPAN: f32 = 1e-3;

/// Infinite line segment on one side of a plan AABB (the “long face” of an opening).
///
/// - If [`Self::thru_is_x`]: face is the vertical line `x = thru`, along = \(Y\).
/// - Else: face is the horizontal line `y = thru`, along = \(X\).
///
/// [`Self::inward_positive`] points into the host along the through axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanOpeningFace {
	pub thru_is_x: bool,
	pub thru: f32,
	pub along0: f32,
	pub along1: f32,
	pub inward_positive: bool,
}

impl PlanOpeningFace {
	/// Long face of `passage` that opens into `host` (closer to host center on the short axis).
	pub fn from_passage(host: Aabb2d, passage: Aabb2d) -> Option<Self> {
		let size = passage.max - passage.min;
		if size.x < MIN_SPAN && size.y < MIN_SPAN {
			return None;
		}
		let c = (host.min + host.max) * 0.5;
		let face = if size.x >= size.y {
			let along0 = passage.min.x;
			let along1 = passage.max.x;
			let d_min = (passage.min.y - c.y).abs();
			let d_max = (passage.max.y - c.y).abs();
			if d_max <= d_min {
				Self {
					thru_is_x: false,
					thru: passage.max.y,
					along0,
					along1,
					inward_positive: c.y >= passage.max.y - EPS,
				}
			} else {
				Self {
					thru_is_x: false,
					thru: passage.min.y,
					along0,
					along1,
					inward_positive: c.y >= passage.min.y - EPS,
				}
			}
		} else {
			let along0 = passage.min.y;
			let along1 = passage.max.y;
			let d_min = (passage.min.x - c.x).abs();
			let d_max = (passage.max.x - c.x).abs();
			if d_max <= d_min {
				Self {
					thru_is_x: true,
					thru: passage.max.x,
					along0,
					along1,
					inward_positive: c.x >= passage.max.x - EPS,
				}
			} else {
				Self {
					thru_is_x: true,
					thru: passage.min.x,
					along0,
					along1,
					inward_positive: c.x >= passage.min.x - EPS,
				}
			}
		};
		face.clip_to_host(host)
	}

	pub fn along_len(self) -> f32 {
		(self.along1 - self.along0).max(0.0)
	}

	pub fn clip_to_host(self, host: Aabb2d) -> Option<Self> {
		let (a0, a1) = if self.thru_is_x {
			(self.along0.max(host.min.y), self.along1.min(host.max.y))
		} else {
			(self.along0.max(host.min.x), self.along1.min(host.max.x))
		};
		if a1 - a0 < MIN_SPAN {
			return None;
		}
		Some(Self {
			along0: a0,
			along1: a1,
			..self
		})
	}

	/// Shared border length with `region` (≥0) when the region sits on the inward side.
	pub fn shared_border_len(self, region: Aabb2d) -> f32 {
		if self.thru_is_x {
			let on_face = if self.inward_positive {
				(region.min.x - self.thru).abs() <= EPS * 10.0
					&& region.max.x > self.thru + MIN_SPAN
			} else {
				(region.max.x - self.thru).abs() <= EPS * 10.0
					&& region.min.x < self.thru - MIN_SPAN
			};
			if !on_face {
				return 0.0;
			}
			overlap_1d(region.min.y, region.max.y, self.along0, self.along1)
		} else {
			let on_face = if self.inward_positive {
				(region.min.y - self.thru).abs() <= EPS * 10.0
					&& region.max.y > self.thru + MIN_SPAN
			} else {
				(region.max.y - self.thru).abs() <= EPS * 10.0
					&& region.min.y < self.thru - MIN_SPAN
			};
			if !on_face {
				return 0.0;
			}
			overlap_1d(region.min.x, region.max.x, self.along0, self.along1)
		}
	}

	pub fn contacts(self, region: Aabb2d, min_len: f32) -> bool {
		self.shared_border_len(region) + EPS >= min_len
	}

	/// Along-intervals not blocked by excludes that sit on this face.
	pub fn free_segments(self, excludes: &[Aabb2d]) -> Vec<(f32, f32)> {
		let mut blocked: Vec<(f32, f32)> = Vec::new();
		for e in excludes {
			let (e0, e1, on) = if self.thru_is_x {
				let on = e.min.x <= self.thru + EPS && e.max.x >= self.thru - EPS;
				(e.min.y, e.max.y, on)
			} else {
				let on = e.min.y <= self.thru + EPS && e.max.y >= self.thru - EPS;
				(e.min.x, e.max.x, on)
			};
			if on {
				let b0 = e0.max(self.along0);
				let b1 = e1.min(self.along1);
				if b1 - b0 > EPS {
					blocked.push((b0, b1));
				}
			}
		}
		blocked.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
		let mut free = Vec::new();
		let mut cursor = self.along0;
		for (b0, b1) in blocked {
			if b0 > cursor + EPS {
				free.push((cursor, b0.min(self.along1)));
			}
			cursor = cursor.max(b1);
		}
		if self.along1 > cursor + EPS {
			free.push((cursor, self.along1));
		}
		free
	}

	pub fn longest_free_segment(self, excludes: &[Aabb2d], min_len: f32) -> Option<(f32, f32)> {
		self.free_segments(excludes)
			.into_iter()
			.filter(|(a, b)| b - a + EPS >= min_len)
			.max_by(|a, b| {
				(a.1 - a.0)
					.partial_cmp(&(b.1 - b.0))
					.unwrap_or(std::cmp::Ordering::Equal)
			})
	}

	fn extrude(self, s0: f32, s1: f32, depth: f32) -> Aabb2d {
		if self.thru_is_x {
			if self.inward_positive {
				Aabb2d {
					min: Vec2::new(self.thru, s0),
					max: Vec2::new(self.thru + depth, s1),
				}
			} else {
				Aabb2d {
					min: Vec2::new(self.thru - depth, s0),
					max: Vec2::new(self.thru, s1),
				}
			}
		} else if self.inward_positive {
			Aabb2d {
				min: Vec2::new(s0, self.thru),
				max: Vec2::new(s1, self.thru + depth),
			}
		} else {
			Aabb2d {
				min: Vec2::new(s0, self.thru - depth),
				max: Vec2::new(s1, self.thru),
			}
		}
	}

	/// Minimal seed extruded inward (`contact_len` × `depth`; `along_t` ∈ [0,1]).
	pub fn seed(self, host: Aabb2d, contact_len: f32, depth: f32, along_t: f32) -> Option<Aabb2d> {
		let face = self.clip_to_host(host)?;
		let contact_len = contact_len.max(MIN_SPAN);
		let depth = depth.max(MIN_SPAN);
		let (s0, s1) = place_segment(face.along0, face.along1, contact_len, along_t)?;
		let seed = face.extrude(s0, s1, depth);
		let clamped = Aabb2d {
			min: seed.min.max(host.min),
			max: seed.max.min(host.max),
		};
		if clamped.max.x - clamped.min.x < MIN_SPAN || clamped.max.y - clamped.min.y < MIN_SPAN {
			return None;
		}
		let need = contact_len.min(face.along_len());
		if face.shared_border_len(clamped) + EPS < need {
			return None;
		}
		Some(clamped)
	}

	/// Seed on the longest free face segment (avoids excludes already on the face).
	pub fn seed_from_free(
		self,
		host: Aabb2d,
		excludes: &[Aabb2d],
		contact_len: f32,
		depth: f32,
		along_t: f32,
	) -> Option<Aabb2d> {
		let face = self.clip_to_host(host)?;
		let (seg0, seg1) = face.longest_free_segment(excludes, contact_len)?;
		let contact_len = contact_len.min(seg1 - seg0).max(MIN_SPAN);
		let depth = depth.max(MIN_SPAN);
		let (s0, s1) = place_segment(seg0, seg1, contact_len, along_t)?;
		let seed = face.extrude(s0, s1, depth);
		let clamped = Aabb2d {
			min: seed.min.max(host.min),
			max: seed.max.min(host.max),
		};
		if clamped.max.x - clamped.min.x < MIN_SPAN || clamped.max.y - clamped.min.y < MIN_SPAN {
			return None;
		}
		if excludes.iter().any(|e| intersects_aabb2(clamped, *e)) {
			return None;
		}
		if face.shared_border_len(clamped) + EPS < contact_len {
			return None;
		}
		Some(clamped)
	}

	/// Counter / furniture band (along × depth into host).
	pub fn band(self, host: Aabb2d, along_len: f32, depth: f32, along_t: f32) -> Option<Aabb2d> {
		self.seed(host, along_len, depth, along_t)
	}

	/// Slab covering the host on the outward side of this face (keeps seeds on the face).
	pub fn outward_block(self, host: Aabb2d) -> Aabb2d {
		if self.thru_is_x {
			if self.inward_positive {
				Aabb2d {
					min: host.min,
					max: Vec2::new(self.thru, host.max.y),
				}
			} else {
				Aabb2d {
					min: Vec2::new(self.thru, host.min.y),
					max: host.max,
				}
			}
		} else if self.inward_positive {
			Aabb2d {
				min: host.min,
				max: Vec2::new(host.max.x, self.thru),
			}
		} else {
			Aabb2d {
				min: Vec2::new(host.min.x, self.thru),
				max: host.max,
			}
		}
	}
}

/// Spec for an optional band on a long opening (counter, bar, …).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptionalFaceBand {
	pub place: bool,
	pub along: f32,
	pub depth: f32,
	pub along_t: f32,
}

impl OptionalFaceBand {
	pub fn resolve(self, host: Aabb2d, face: PlanOpeningFace) -> Option<Aabb2d> {
		if !self.place {
			return None;
		}
		face.band(host, self.along, self.depth, self.along_t)
	}

	/// Resolve bands for paired faces/specs. Skips `place: false` and failed geometry.
	pub fn pack_many(host: Aabb2d, faces: &[PlanOpeningFace], specs: &[Self]) -> Vec<Aabb2d> {
		let n = faces.len().min(specs.len());
		let mut out = Vec::new();
		for i in 0..n {
			if let Some(band) = specs[i].resolve(host, faces[i]) {
				out.push(band);
			}
		}
		out
	}
}

/// Packing operations on plan [`Aabb2d`]s (extension trait — `Aabb2d` is foreign).
pub trait Aabb2dPack: Sized {
	fn pack_area(self) -> f32;
	fn grow_into(self, host: Self, excludes: &[Self]) -> Self;
	fn grow_toward_area(self, host: Self, excludes: &[Self], target_area: f32) -> Self;
	fn is_clear_of(self, excludes: &[Self]) -> bool;

	/// Reserved: axis-aligned bipartition by area fraction.
	fn bipartition_by_area(self, cut_x: bool, first_from_min: bool, first_frac: f32) -> (Self, Self);

	/// Reserved: alternate full grow of two seeds into free space.
	fn grow_pair(
		host: Self,
		a: Self,
		b: Self,
		hard_a: &[Self],
		hard_b: &[Self],
		rounds: usize,
	) -> (Self, Self);

	/// Reserved: grow two seeds toward area targets, then scrap-fill.
	fn grow_pair_toward_areas(
		host: Self,
		a: Self,
		b: Self,
		hard_a: &[Self],
		hard_b: &[Self],
		target_a: f32,
		target_b: f32,
		rounds: usize,
	) -> (Self, Self);
}

impl Aabb2dPack for Aabb2d {
	fn pack_area(self) -> f32 {
		aabb2_area(self)
	}

	fn grow_into(self, host: Self, excludes: &[Self]) -> Self {
		grow_aabb2(host, self, excludes)
	}

	fn grow_toward_area(self, host: Self, excludes: &[Self], target_area: f32) -> Self {
		let full = self.grow_into(host, excludes);
		let seed_area = self.pack_area();
		let full_area = full.pack_area();
		let target = target_area.max(seed_area);
		if full_area <= target + EPS {
			return full;
		}
		if seed_area >= target - EPS {
			return self;
		}
		let mut lo = 0.0_f32;
		let mut hi = 1.0_f32;
		let mut best = self;
		for _ in 0..24 {
			let mid = 0.5 * (lo + hi);
			let cand = lerp_aabb2(self, full, mid);
			if cand.pack_area() < target {
				lo = mid;
				best = cand;
			} else {
				hi = mid;
				best = cand;
			}
		}
		best
	}

	fn is_clear_of(self, excludes: &[Self]) -> bool {
		!excludes.iter().any(|e| intersects_aabb2(self, *e))
	}

	fn bipartition_by_area(self, cut_x: bool, first_from_min: bool, first_frac: f32) -> (Self, Self) {
		let frac = first_frac.clamp(0.05, 0.95);
		if cut_x {
			let span = self.max.x - self.min.x;
			let cut = if first_from_min {
				self.min.x + span * frac
			} else {
				self.max.x - span * frac
			};
			if first_from_min {
				(
					Aabb2d {
						min: self.min,
						max: Vec2::new(cut, self.max.y),
					},
					Aabb2d {
						min: Vec2::new(cut, self.min.y),
						max: self.max,
					},
				)
			} else {
				(
					Aabb2d {
						min: Vec2::new(cut, self.min.y),
						max: self.max,
					},
					Aabb2d {
						min: self.min,
						max: Vec2::new(cut, self.max.y),
					},
				)
			}
		} else {
			let span = self.max.y - self.min.y;
			let cut = if first_from_min {
				self.min.y + span * frac
			} else {
				self.max.y - span * frac
			};
			if first_from_min {
				(
					Aabb2d {
						min: self.min,
						max: Vec2::new(self.max.x, cut),
					},
					Aabb2d {
						min: Vec2::new(self.min.x, cut),
						max: self.max,
					},
				)
			} else {
				(
					Aabb2d {
						min: Vec2::new(self.min.x, cut),
						max: self.max,
					},
					Aabb2d {
						min: self.min,
						max: Vec2::new(self.max.x, cut),
					},
				)
			}
		}
	}

	fn grow_pair(
		host: Self,
		a: Self,
		b: Self,
		hard_a: &[Self],
		hard_b: &[Self],
		rounds: usize,
	) -> (Self, Self) {
		let mut a = a;
		let mut b = b;
		for _ in 0..rounds.max(1) {
			let mut ex_a = hard_a.to_vec();
			ex_a.push(b);
			let a_next = a.grow_into(host, &ex_a);
			let mut ex_b = hard_b.to_vec();
			ex_b.push(a_next);
			let b_next = b.grow_into(host, &ex_b);
			let stable = (a_next.min - a.min).length_squared() < EPS * EPS
				&& (a_next.max - a.max).length_squared() < EPS * EPS
				&& (b_next.min - b.min).length_squared() < EPS * EPS
				&& (b_next.max - b.max).length_squared() < EPS * EPS;
			a = a_next;
			b = b_next;
			if stable {
				break;
			}
		}
		(a, b)
	}

	fn grow_pair_toward_areas(
		host: Self,
		a: Self,
		b: Self,
		hard_a: &[Self],
		hard_b: &[Self],
		target_a: f32,
		target_b: f32,
		rounds: usize,
	) -> (Self, Self) {
		let mut ex_a = hard_a.to_vec();
		ex_a.push(b);
		let mut a = a.grow_toward_area(host, &ex_a, target_a);
		let mut ex_b = hard_b.to_vec();
		ex_b.push(a);
		let mut b = b.grow_toward_area(host, &ex_b, target_b);
		for _ in 0..rounds.max(1) {
			ex_a = hard_a.to_vec();
			ex_a.push(b);
			let a_next = a.grow_into(host, &ex_a);
			ex_b = hard_b.to_vec();
			ex_b.push(a_next);
			let b_next = b.grow_into(host, &ex_b);
			let stable = (a_next.min - a.min).length_squared() < EPS * EPS
				&& (a_next.max - a.max).length_squared() < EPS * EPS
				&& (b_next.min - b.min).length_squared() < EPS * EPS
				&& (b_next.max - b.max).length_squared() < EPS * EPS;
			a = a_next;
			b = b_next;
			if stable {
				break;
			}
		}
		(a, b)
	}
}

fn overlap_1d(a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
	(a1.min(b1) - a0.max(b0)).max(0.0)
}

fn place_segment(a0: f32, a1: f32, len: f32, t: f32) -> Option<(f32, f32)> {
	let span = a1 - a0;
	if span + EPS < len || len < MIN_SPAN {
		return None;
	}
	let t = t.clamp(0.0, 1.0);
	let slack = span - len;
	let s0 = a0 + slack * t;
	Some((s0, s0 + len))
}

fn lerp_aabb2(a: Aabb2d, b: Aabb2d, t: f32) -> Aabb2d {
	let t = t.clamp(0.0, 1.0);
	Aabb2d {
		min: a.min + (b.min - a.min) * t,
		max: a.max + (b.max - a.max) * t,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn south_passage_face_opens_north_into_host() {
		let host = Aabb2d {
			min: Vec2::ZERO,
			max: Vec2::new(10.0, 8.0),
		};
		let passage = Aabb2d {
			min: Vec2::new(1.0, -0.2),
			max: Vec2::new(4.0, 0.2),
		};
		let face = PlanOpeningFace::from_passage(host, passage).unwrap();
		assert!(!face.thru_is_x);
		assert!((face.thru - 0.2).abs() < 1e-3);
		assert!(face.inward_positive);
		assert!(face.along_len() >= 2.9);
	}

	#[test]
	fn seed_shares_one_meter_border() {
		let host = Aabb2d {
			min: Vec2::ZERO,
			max: Vec2::new(10.0, 8.0),
		};
		let passage = Aabb2d {
			min: Vec2::new(1.0, -0.2),
			max: Vec2::new(4.0, 0.2),
		};
		let face = PlanOpeningFace::from_passage(host, passage).unwrap();
		let seed = face.seed(host, 1.0, 1.0, 0.5).unwrap();
		assert!(face.contacts(seed, 1.0));
		assert!((seed.max.y - seed.min.y - 1.0).abs() < 1e-2);
	}

	#[test]
	fn face_band_depth_into_host() {
		let host = Aabb2d {
			min: Vec2::ZERO,
			max: Vec2::new(10.0, 8.0),
		};
		let passage = Aabb2d {
			min: Vec2::new(1.0, -0.2),
			max: Vec2::new(5.0, 0.2),
		};
		let face = PlanOpeningFace::from_passage(host, passage).unwrap();
		let band = face.band(host, 2.0, 0.8, 0.0).unwrap();
		assert!((band.max.y - band.min.y - 0.8).abs() < 1e-2);
		assert!((band.max.x - band.min.x - 2.0).abs() < 1e-2);
	}

	#[test]
	fn grow_toward_area_stops_near_target() {
		let host = Aabb2d {
			min: Vec2::ZERO,
			max: Vec2::new(10.0, 6.0),
		};
		let seed = Aabb2d {
			min: Vec2::new(4.0, 0.0),
			max: Vec2::new(6.0, 1.0),
		};
		let grown = seed.grow_toward_area(host, &[], 12.0);
		let area = grown.pack_area();
		assert!(area >= 11.0 && area <= 14.0, "area {area}");
	}

	#[test]
	fn bipartition_splits_half() {
		let host = Aabb2d {
			min: Vec2::ZERO,
			max: Vec2::new(10.0, 6.0),
		};
		let (a, b) = host.bipartition_by_area(false, true, 0.5);
		assert!((a.pack_area() - 30.0).abs() < 1e-2);
		assert!((b.pack_area() - 30.0).abs() < 1e-2);
	}
}
