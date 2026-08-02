//! Plan adjacency and concave-corner (L/T) detection for orthogonal massing.

use super::geometry::{LongAxis, VolumeCandidate, EPS};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PlanRect {
	pub min_x: f32,
	pub min_z: f32,
	pub max_x: f32,
	pub max_z: f32,
}

impl PlanRect {
	pub fn from_candidate(c: &VolumeCandidate) -> Self {
		let (min_x, min_z) = c.plan_min();
		let (max_x, max_z) = c.plan_max();
		Self {
			min_x,
			min_z,
			max_x,
			max_z,
		}
	}

	pub fn overlap(self, other: Self) -> Option<Self> {
		let min_x = self.min_x.max(other.min_x);
		let min_z = self.min_z.max(other.min_z);
		let max_x = self.max_x.min(other.max_x);
		let max_z = self.max_z.min(other.max_z);
		if max_x + EPS < min_x || max_z + EPS < min_z {
			return None;
		}
		Some(Self {
			min_x,
			min_z,
			max_x: max_x.max(min_x),
			max_z: max_z.max(min_z),
		})
	}
}

/// A concave plan corner where two perpendicular volumes meet (valley site).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ConcaveCorner {
	/// Long-X volume index.
	pub vol_a: usize,
	/// Long-Z volume index.
	pub vol_b: usize,
	/// Eave/wall side on A: `0` = −Z, `1` = +Z.
	pub side_a: usize,
	/// Eave/wall side on B: `0` = −X, `1` = +X.
	pub side_b: usize,
	/// Junction long-end on A (`0` = min X, `1` = max X), if any.
	pub end_a: Option<usize>,
	/// Junction long-end on B (`0` = min Z, `1` = max Z), if any.
	pub end_b: Option<usize>,
	/// Inner corner in XZ (massing wall plan).
	pub corner_xz: (f32, f32),
}

/// Find orthogonal L/T concave corners and mark junction ends on candidates.
pub(super) fn resolve_junctions(volumes: &mut [VolumeCandidate]) -> Vec<ConcaveCorner> {
	let n = volumes.len();
	let rects: Vec<PlanRect> = volumes.iter().map(PlanRect::from_candidate).collect();
	let mut corners = Vec::new();

	for i in 0..n {
		for j in (i + 1)..n {
			if volumes[i].long_axis == volumes[j].long_axis {
				continue;
			}
			let (ia, ib, ra, rb) = if volumes[i].long_axis == LongAxis::X {
				(i, j, rects[i], rects[j])
			} else {
				(j, i, rects[j], rects[i])
			};
			let Some(overlap) = ra.overlap(rb) else {
				continue;
			};
			for c in classify_xz_pair(ia, ib, ra, rb, overlap) {
				if let Some(end) = c.end_a {
					volumes[c.vol_a].end_free[end] = false;
				}
				if let Some(end) = c.end_b {
					volumes[c.vol_b].end_free[end] = false;
				}
				corners.push(c);
			}
		}
	}
	corners
}

fn classify_xz_pair(
	vol_a: usize,
	vol_b: usize,
	ra: PlanRect,
	rb: PlanRect,
	overlap: PlanRect,
) -> Vec<ConcaveCorner> {
	let a_pos = ra.max_x > overlap.max_x + EPS;
	let a_neg = ra.min_x < overlap.min_x - EPS;
	let b_pos = rb.max_z > overlap.max_z + EPS;
	let b_neg = rb.min_z < overlap.min_z - EPS;

	let mut out = Vec::new();

	// L: each arm extends one way past the overlap.
	if (a_pos ^ a_neg) && (b_pos ^ b_neg) {
		let corner_x = if a_pos { overlap.max_x } else { overlap.min_x };
		let corner_z = if b_pos { overlap.max_z } else { overlap.min_z };
		let side_a = if b_pos { 1 } else { 0 };
		let side_b = if a_pos { 1 } else { 0 };
		let end_a = if a_pos {
			// Free +X → junction toward −X if A's min aligns with overlap.
			if (ra.min_x - overlap.min_x).abs() <= EPS {
				Some(0)
			} else {
				None
			}
		} else if (ra.max_x - overlap.max_x).abs() <= EPS {
			Some(1)
		} else {
			None
		};
		let end_b = if b_pos {
			if (rb.min_z - overlap.min_z).abs() <= EPS {
				Some(0)
			} else {
				None
			}
		} else if (rb.max_z - overlap.max_z).abs() <= EPS {
			Some(1)
		} else {
			None
		};
		out.push(ConcaveCorner {
			vol_a,
			vol_b,
			side_a,
			side_b,
			end_a,
			end_b,
			corner_xz: (corner_x, corner_z),
		});
		return out;
	}

	// T: bar extends both ways along X, stem extends one way along Z.
	if a_pos && a_neg && (b_pos ^ b_neg) {
		let corner_z = if b_pos { overlap.max_z } else { overlap.min_z };
		let side_a = if b_pos { 1 } else { 0 };
		let end_b = if b_pos {
			if (rb.min_z - overlap.min_z).abs() <= EPS {
				Some(0)
			} else {
				None
			}
		} else if (rb.max_z - overlap.max_z).abs() <= EPS {
			Some(1)
		} else {
			None
		};
		for &(corner_x, side_b) in &[(overlap.min_x, 0usize), (overlap.max_x, 1usize)] {
			out.push(ConcaveCorner {
				vol_a,
				vol_b,
				side_a,
				side_b,
				end_a: None,
				end_b,
				corner_xz: (corner_x, corner_z),
			});
		}
		return out;
	}

	// T: bar along Z, stem along X.
	if b_pos && b_neg && (a_pos ^ a_neg) {
		let corner_x = if a_pos { overlap.max_x } else { overlap.min_x };
		let side_b = if a_pos { 1 } else { 0 };
		let end_a = if a_pos {
			if (ra.min_x - overlap.min_x).abs() <= EPS {
				Some(0)
			} else {
				None
			}
		} else if (ra.max_x - overlap.max_x).abs() <= EPS {
			Some(1)
		} else {
			None
		};
		for &(corner_z, side_a) in &[(overlap.min_z, 0usize), (overlap.max_z, 1usize)] {
			out.push(ConcaveCorner {
				vol_a,
				vol_b,
				side_a,
				side_b,
				end_a,
				end_b: None,
				corner_xz: (corner_x, corner_z),
			});
		}
	}

	out
}
