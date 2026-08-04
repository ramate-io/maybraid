//! Axis-aligned volume packing: opening faces, face bands, area-targeted grow.
//!
//! Complements [`crate::bounds`] (max-empty / inflate / grow primitives) with
//! higher-level placement that still operates on plan AABBs.

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
	pub fn along_len(self) -> f32 {
		(self.along1 - self.along0).max(0.0)
	}

	/// Clip along-span to `host`.
	pub fn clip_to_host(self, host: Aabb2d) -> Option<Self> {
		let (a0, a1) = if self.thru_is_x {
			(
				self.along0.max(host.min.y),
				self.along1.min(host.max.y),
			)
		} else {
			(
				self.along0.max(host.min.x),
				self.along1.min(host.max.x),
			)
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
}

/// Long face of `passage` that opens into `host` (closer to host center on the short axis).
pub fn passage_opening_face(host: Aabb2d, passage: Aabb2d) -> Option<PlanOpeningFace> {
	let size = passage.max - passage.min;
	if size.x < MIN_SPAN && size.y < MIN_SPAN {
		return None;
	}
	let c = (host.min + host.max) * 0.5;
	let face = if size.x >= size.y {
		// Long axis X → faces at min.y / max.y.
		let along0 = passage.min.x;
		let along1 = passage.max.x;
		let d_min = (passage.min.y - c.y).abs();
		let d_max = (passage.max.y - c.y).abs();
		if d_max <= d_min {
			PlanOpeningFace {
				thru_is_x: false,
				thru: passage.max.y,
				along0,
				along1,
				inward_positive: c.y >= passage.max.y - EPS,
			}
		} else {
			PlanOpeningFace {
				thru_is_x: false,
				thru: passage.min.y,
				along0,
				along1,
				inward_positive: c.y >= passage.min.y - EPS,
			}
		}
	} else {
		// Long axis Y → faces at min.x / max.x.
		let along0 = passage.min.y;
		let along1 = passage.max.y;
		let d_min = (passage.min.x - c.x).abs();
		let d_max = (passage.max.x - c.x).abs();
		if d_max <= d_min {
			PlanOpeningFace {
				thru_is_x: true,
				thru: passage.max.x,
				along0,
				along1,
				inward_positive: c.x >= passage.max.x - EPS,
			}
		} else {
			PlanOpeningFace {
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

fn overlap_1d(a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
	(a1.min(b1) - a0.max(b0)).max(0.0)
}

/// Length of the shared border between `region` and the opening face (≥0).
///
/// Counts only when the region sits on the inward side of the face and its
/// boundary overlaps the face segment along-axis.
pub fn shared_opening_border_len(region: Aabb2d, face: PlanOpeningFace) -> f32 {
	if face.thru_is_x {
		let on_face = if face.inward_positive {
			(region.min.x - face.thru).abs() <= EPS * 10.0
				&& region.max.x > face.thru + MIN_SPAN
		} else {
			(region.max.x - face.thru).abs() <= EPS * 10.0
				&& region.min.x < face.thru - MIN_SPAN
		};
		if !on_face {
			return 0.0;
		}
		overlap_1d(region.min.y, region.max.y, face.along0, face.along1)
	} else {
		let on_face = if face.inward_positive {
			(region.min.y - face.thru).abs() <= EPS * 10.0
				&& region.max.y > face.thru + MIN_SPAN
		} else {
			(region.max.y - face.thru).abs() <= EPS * 10.0
				&& region.min.y < face.thru - MIN_SPAN
		};
		if !on_face {
			return 0.0;
		}
		overlap_1d(region.min.x, region.max.x, face.along0, face.along1)
	}
}

pub fn contacts_opening_face(region: Aabb2d, face: PlanOpeningFace, min_len: f32) -> bool {
	shared_opening_border_len(region, face) + EPS >= min_len
}

/// Unit interval placement of a segment of length `len` inside `[a0, a1]`.
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

fn extrude_from_face(face: PlanOpeningFace, s0: f32, s1: f32, depth: f32) -> Aabb2d {
	if face.thru_is_x {
		if face.inward_positive {
			Aabb2d {
				min: Vec2::new(face.thru, s0),
				max: Vec2::new(face.thru + depth, s1),
			}
		} else {
			Aabb2d {
				min: Vec2::new(face.thru - depth, s0),
				max: Vec2::new(face.thru, s1),
			}
		}
	} else if face.inward_positive {
		Aabb2d {
			min: Vec2::new(s0, face.thru),
			max: Vec2::new(s1, face.thru + depth),
		}
	} else {
		Aabb2d {
			min: Vec2::new(s0, face.thru - depth),
			max: Vec2::new(s1, face.thru),
		}
	}
}

/// Minimal seed extruded inward from an opening face.
///
/// `contact_len` along the face, `depth` into the host; `along_t` ∈ [0,1] slides
/// the contact segment within the face.
pub fn seed_from_opening_face(
	host: Aabb2d,
	face: PlanOpeningFace,
	contact_len: f32,
	depth: f32,
	along_t: f32,
) -> Option<Aabb2d> {
	let face = face.clip_to_host(host)?;
	let contact_len = contact_len.max(MIN_SPAN);
	let depth = depth.max(MIN_SPAN);
	let (s0, s1) = place_segment(face.along0, face.along1, contact_len, along_t)?;
	let seed = extrude_from_face(face, s0, s1, depth);
	let clamped = Aabb2d {
		min: seed.min.max(host.min),
		max: seed.max.min(host.max),
	};
	if clamped.max.x - clamped.min.x < MIN_SPAN || clamped.max.y - clamped.min.y < MIN_SPAN {
		return None;
	}
	let need = contact_len.min(face.along_len());
	if shared_opening_border_len(clamped, face) + EPS < need {
		return None;
	}
	Some(clamped)
}

/// Seed on the longest free face segment (avoiding excludes already on the face).
pub fn seed_from_free_opening_face(
	host: Aabb2d,
	face: PlanOpeningFace,
	excludes: &[Aabb2d],
	contact_len: f32,
	depth: f32,
	along_t: f32,
) -> Option<Aabb2d> {
	let face = face.clip_to_host(host)?;
	let (seg0, seg1) = longest_free_face_segment(face, excludes, contact_len)?;
	let contact_len = contact_len.min(seg1 - seg0).max(MIN_SPAN);
	let depth = depth.max(MIN_SPAN);
	let (s0, s1) = place_segment(seg0, seg1, contact_len, along_t)?;
	let seed = extrude_from_face(face, s0, s1, depth);
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
	if shared_opening_border_len(clamped, face) + EPS < contact_len {
		return None;
	}
	Some(clamped)
}

/// Counter / furniture band on an opening face (along × depth into host).
pub fn face_band(
	host: Aabb2d,
	face: PlanOpeningFace,
	along_len: f32,
	depth: f32,
	along_t: f32,
) -> Option<Aabb2d> {
	seed_from_opening_face(host, face, along_len, depth, along_t)
}

/// Spec for an optional band on a long opening (counter, bar, …).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptionalFaceBand {
	pub place: bool,
	/// Along length in world units (already resolved).
	pub along: f32,
	pub depth: f32,
	/// 0..1 placement along the opening face.
	pub along_t: f32,
}

/// Along-intervals of `face` not blocked by excludes that sit on the face.
///
/// An exclude blocks the face where it shares a border (same thru line) and
/// overlaps in the along axis — typical for a counter already on that opening.
pub fn free_segments_on_face(face: PlanOpeningFace, excludes: &[Aabb2d]) -> Vec<(f32, f32)> {
	let mut blocked: Vec<(f32, f32)> = Vec::new();
	for e in excludes {
		let (e0, e1, on) = if face.thru_is_x {
			let on = e.min.x <= face.thru + EPS && e.max.x >= face.thru - EPS;
			(e.min.y, e.max.y, on)
		} else {
			let on = e.min.y <= face.thru + EPS && e.max.y >= face.thru - EPS;
			(e.min.x, e.max.x, on)
		};
		if on {
			let b0 = e0.max(face.along0);
			let b1 = e1.min(face.along1);
			if b1 - b0 > EPS {
				blocked.push((b0, b1));
			}
		}
	}
	blocked.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
	let mut free = Vec::new();
	let mut cursor = face.along0;
	for (b0, b1) in blocked {
		if b0 > cursor + EPS {
			free.push((cursor, b0.min(face.along1)));
		}
		cursor = cursor.max(b1);
	}
	if face.along1 > cursor + EPS {
		free.push((cursor, face.along1));
	}
	free
}

/// Best free segment on `face` with length ≥ `min_len` (longest wins).
pub fn longest_free_face_segment(
	face: PlanOpeningFace,
	excludes: &[Aabb2d],
	min_len: f32,
) -> Option<(f32, f32)> {
	free_segments_on_face(face, excludes)
		.into_iter()
		.filter(|(a, b)| b - a + EPS >= min_len)
		.max_by(|a, b| (a.1 - a.0).partial_cmp(&(b.1 - b.0)).unwrap_or(std::cmp::Ordering::Equal))
}

/// Resolve bands for a list of opening faces. Skips `place: false` and failed geometry.
pub fn pack_optional_face_bands(
	host: Aabb2d,
	faces: &[PlanOpeningFace],
	specs: &[OptionalFaceBand],
) -> Vec<Aabb2d> {
	let n = faces.len().min(specs.len());
	let mut out = Vec::new();
	for i in 0..n {
		if !specs[i].place {
			continue;
		}
		if let Some(band) = face_band(host, faces[i], specs[i].along, specs[i].depth, specs[i].along_t)
		{
			out.push(band);
		}
	}
	out
}

/// Axis-aligned bipartition of `host` by area fraction along one axis.
///
/// `first_from_min`: the first piece is the low side of the cut axis.
pub fn bipartition_aabb2_by_area(
	host: Aabb2d,
	cut_x: bool,
	first_from_min: bool,
	first_frac: f32,
) -> (Aabb2d, Aabb2d) {
	let frac = first_frac.clamp(0.05, 0.95);
	if cut_x {
		let span = host.max.x - host.min.x;
		let cut = if first_from_min {
			host.min.x + span * frac
		} else {
			host.max.x - span * frac
		};
		let (a, b) = if first_from_min {
			(
				Aabb2d {
					min: host.min,
					max: Vec2::new(cut, host.max.y),
				},
				Aabb2d {
					min: Vec2::new(cut, host.min.y),
					max: host.max,
				},
			)
		} else {
			(
				Aabb2d {
					min: Vec2::new(cut, host.min.y),
					max: host.max,
				},
				Aabb2d {
					min: host.min,
					max: Vec2::new(cut, host.max.y),
				},
			)
		};
		(a, b)
	} else {
		let span = host.max.y - host.min.y;
		let cut = if first_from_min {
			host.min.y + span * frac
		} else {
			host.max.y - span * frac
		};
		let (a, b) = if first_from_min {
			(
				Aabb2d {
					min: host.min,
					max: Vec2::new(host.max.x, cut),
				},
				Aabb2d {
					min: Vec2::new(host.min.x, cut),
					max: host.max,
				},
			)
		} else {
			(
				Aabb2d {
					min: Vec2::new(host.min.x, cut),
					max: host.max,
				},
				Aabb2d {
					min: host.min,
					max: Vec2::new(host.max.x, cut),
				},
			)
		};
		(a, b)
	}
}

fn lerp_aabb2(a: Aabb2d, b: Aabb2d, t: f32) -> Aabb2d {
	let t = t.clamp(0.0, 1.0);
	Aabb2d {
		min: a.min + (b.min - a.min) * t,
		max: a.max + (b.max - a.max) * t,
	}
}

/// Grow `seed` inside `host` avoiding `excludes`, stopping near `target_area`.
///
/// Full grow first; if oversized, binary-search the lerp back toward `seed`.
pub fn grow_aabb2_toward_area(
	host: Aabb2d,
	seed: Aabb2d,
	excludes: &[Aabb2d],
	target_area: f32,
) -> Aabb2d {
	let full = grow_aabb2(host, seed, excludes);
	let seed_area = aabb2_area(seed);
	let full_area = aabb2_area(full);
	let target = target_area.max(seed_area);
	if full_area <= target + EPS {
		return full;
	}
	if seed_area >= target - EPS {
		return seed;
	}
	let mut lo = 0.0_f32;
	let mut hi = 1.0_f32;
	let mut best = seed;
	for _ in 0..24 {
		let mid = 0.5 * (lo + hi);
		let cand = lerp_aabb2(seed, full, mid);
		if aabb2_area(cand) < target {
			lo = mid;
			best = cand;
		} else {
			hi = mid;
			best = cand;
		}
	}
	best
}

/// Grow two seeds toward area targets, then alternate full grow for scraps.
pub fn grow_aabb2_pair_toward_areas(
	host: Aabb2d,
	a: Aabb2d,
	b: Aabb2d,
	hard_a: &[Aabb2d],
	hard_b: &[Aabb2d],
	target_a: f32,
	target_b: f32,
	rounds: usize,
) -> (Aabb2d, Aabb2d) {
	let mut ex_a: Vec<Aabb2d> = hard_a.to_vec();
	ex_a.push(b);
	let mut a = grow_aabb2_toward_area(host, a, &ex_a, target_a);

	let mut ex_b: Vec<Aabb2d> = hard_b.to_vec();
	ex_b.push(a);
	let mut b = grow_aabb2_toward_area(host, b, &ex_b, target_b);

	for _ in 0..rounds.max(1) {
		ex_a = hard_a.to_vec();
		ex_a.push(b);
		let a_next = grow_aabb2(host, a, &ex_a);
		ex_b = hard_b.to_vec();
		ex_b.push(a_next);
		let b_next = grow_aabb2(host, b, &ex_b);
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

/// True when `band` does not intersect any exclude (open overlap).
pub fn band_clear_of(band: Aabb2d, excludes: &[Aabb2d]) -> bool {
	!excludes.iter().any(|e| intersects_aabb2(band, *e))
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
		let face = passage_opening_face(host, passage).unwrap();
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
		let face = passage_opening_face(host, passage).unwrap();
		let seed = seed_from_opening_face(host, face, 1.0, 1.0, 0.5).unwrap();
		assert!(contacts_opening_face(seed, face, 1.0));
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
		let face = passage_opening_face(host, passage).unwrap();
		let band = face_band(host, face, 2.0, 0.8, 0.0).unwrap();
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
		let grown = grow_aabb2_toward_area(host, seed, &[], 12.0);
		let area = aabb2_area(grown);
		assert!(area >= 11.0 && area <= 14.0, "area {area}");
	}

	#[test]
	fn bipartition_splits_half() {
		let host = Aabb2d {
			min: Vec2::ZERO,
			max: Vec2::new(10.0, 6.0),
		};
		let (a, b) = bipartition_aabb2_by_area(host, false, true, 0.5);
		assert!((aabb2_area(a) - 30.0).abs() < 1e-2);
		assert!((aabb2_area(b) - 30.0).abs() < 1e-2);
	}
}
