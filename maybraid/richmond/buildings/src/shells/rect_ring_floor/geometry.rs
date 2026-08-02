//! Outer / inner wall runs and frame slab bands for [`RectRingFloor`].

use bevy_math::{Vec2, Vec3};

use crate::shells::ortho::{PlanRect, WallEdge, EPS};

use super::RectRingFloorParams;

/// Axis-aligned plan rectangle (full extents) for frame slab bands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PlanAabb {
	pub min_x: f32,
	pub max_x: f32,
	pub min_z: f32,
	pub max_z: f32,
}

impl PlanAabb {
	pub fn new(min_x: f32, max_x: f32, min_z: f32, max_z: f32) -> Self {
		Self {
			min_x: min_x.min(max_x),
			max_x: min_x.max(max_x),
			min_z: min_z.min(max_z),
			max_z: min_z.max(max_z),
		}
	}

	pub fn to_plan_rect(self, y: f32) -> PlanRect {
		PlanRect::new(
			Vec3::new(
				0.5 * (self.min_x + self.max_x),
				y,
				0.5 * (self.min_z + self.max_z),
			),
			(self.max_x - self.min_x).max(EPS),
			(self.max_z - self.min_z).max(EPS),
		)
	}
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RectRingGeom {
	pub y0: f32,
	pub height: f32,
	pub slab_rects: Vec<PlanAabb>,
	pub edges: Vec<WallEdge>,
}

impl RectRingFloorParams {
	pub(super) fn resolve_geometry(&self) -> RectRingGeom {
		let cx = self.center_xz.x;
		let cz = self.center_xz.z;
		let y0 = self.center_xz.y;
		let height = self.storey_height.max(EPS);

		let ox0 = cx - self.outer.x * 0.5;
		let ox1 = cx + self.outer.x * 0.5;
		let oz0 = cz - self.outer.y * 0.5;
		let oz1 = cz + self.outer.y * 0.5;
		let ix0 = cx - self.inner.x * 0.5;
		let ix1 = cx + self.inner.x * 0.5;
		let iz0 = cz - self.inner.y * 0.5;
		let iz1 = cz + self.inner.y * 0.5;

		let mut edges = Vec::new();
		// Outer loop: CCW S→E→N→W. Gallery-facing normals point into the ring.
		push_cardinal_runs(
			&mut edges,
			y0,
			height,
			[
				// South: SW→SE, gallery +Z
				(Vec3::new(ox0, y0, oz0), Vec3::new(ox1, y0, oz0), Vec2::new(0.0, 1.0)),
				// East: SE→NE, gallery −X
				(Vec3::new(ox1, y0, oz0), Vec3::new(ox1, y0, oz1), Vec2::new(-1.0, 0.0)),
				// North: NE→NW, gallery −Z
				(Vec3::new(ox1, y0, oz1), Vec3::new(ox0, y0, oz1), Vec2::new(0.0, -1.0)),
				// West: NW→SW, gallery +X
				(Vec3::new(ox0, y0, oz1), Vec3::new(ox0, y0, oz0), Vec2::new(1.0, 0.0)),
			],
			&self.outer_omits,
		);
		// Inner loop: same CCW side starts for omit measurement; gallery normals
		// face out of the courtyard (into the ring corridor). Walk each side
		// opposite the outer so the strip faces the gallery (CW relative to the
		// courtyard polygon).
		push_cardinal_runs(
			&mut edges,
			y0,
			height,
			[
				// South: SE→SW (CW), gallery −Z
				(Vec3::new(ix1, y0, iz0), Vec3::new(ix0, y0, iz0), Vec2::new(0.0, -1.0)),
				// East: NE→SE (CW), gallery +X
				(Vec3::new(ix1, y0, iz1), Vec3::new(ix1, y0, iz0), Vec2::new(1.0, 0.0)),
				// North: NW→NE (CW), gallery +Z
				(Vec3::new(ix0, y0, iz1), Vec3::new(ix1, y0, iz1), Vec2::new(0.0, 1.0)),
				// West: SW→NW (CW), gallery −X
				(Vec3::new(ix0, y0, iz0), Vec3::new(ix0, y0, iz1), Vec2::new(-1.0, 0.0)),
			],
			&map_omits_for_cw_sides(&self.inner_omits, [self.inner.x, self.inner.y, self.inner.x, self.inner.y]),
		);

		// Frame bands: N/S take full outer width; E/W take the inner depth only.
		let mut slab_rects = Vec::new();
		if oz1 - iz1 > EPS {
			slab_rects.push(PlanAabb::new(ox0, ox1, iz1, oz1)); // North
		}
		if ix0 - ox0 > EPS {
			slab_rects.push(PlanAabb::new(ox0, ix0, iz0, iz1)); // West
		}
		if iz0 - oz0 > EPS {
			slab_rects.push(PlanAabb::new(ox0, ox1, oz0, iz0)); // South
		}
		if ox1 - ix1 > EPS {
			slab_rects.push(PlanAabb::new(ix1, ox1, iz0, iz1)); // East
		}

		RectRingGeom {
			y0,
			height,
			slab_rects,
			edges,
		}
	}
}

/// Remap CCW-authored omit intervals onto CW side walks (same S/E/N/W indexing).
fn map_omits_for_cw_sides(
	omits: &[Vec<(f32, f32)>; 4],
	lengths: [f32; 4],
) -> [Vec<(f32, f32)>; 4] {
	std::array::from_fn(|i| reverse_omits(lengths[i], &omits[i]))
}

fn reverse_omits(side_len: f32, omits: &[(f32, f32)]) -> Vec<(f32, f32)> {
	let len = side_len.max(EPS);
	omits
		.iter()
		.map(|&(a, b)| {
			let lo = a.min(b).clamp(0.0, len);
			let hi = a.max(b).clamp(0.0, len);
			(len - hi, len - lo)
		})
		.collect()
}

fn push_cardinal_runs(
	edges: &mut Vec<WallEdge>,
	_y0: f32,
	height: f32,
	sides: [(Vec3, Vec3, Vec2); 4],
	omits: &[Vec<(f32, f32)>; 4],
) {
	for (i, (start, end, outward)) in sides.into_iter().enumerate() {
		let len = start.distance(end);
		if len < EPS {
			continue;
		}
		let dir = (end - start) / len;
		for (a, b) in solid_runs(len, &omits[i]) {
			let s = start + dir * a;
			let e = start + dir * b;
			if s.distance(e) < EPS {
				continue;
			}
			edges.push(WallEdge::new(s, e, height, outward));
		}
	}
}

/// Solid intervals along `[0, length]` after subtracting merged omit intervals.
pub(super) fn solid_runs(length: f32, omits: &[(f32, f32)]) -> Vec<(f32, f32)> {
	let length = length.max(0.0);
	if length < EPS {
		return Vec::new();
	}
	let mut intervals: Vec<(f32, f32)> = omits
		.iter()
		.filter_map(|&(a, b)| {
			let lo = a.min(b).clamp(0.0, length);
			let hi = a.max(b).clamp(0.0, length);
			(hi - lo > EPS).then_some((lo, hi))
		})
		.collect();
	intervals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
	let mut merged = Vec::<(f32, f32)>::new();
	for (lo, hi) in intervals {
		if let Some(last) = merged.last_mut() {
			if lo <= last.1 + EPS {
				last.1 = last.1.max(hi);
				continue;
			}
		}
		merged.push((lo, hi));
	}

	let mut solids = Vec::new();
	let mut cursor = 0.0f32;
	for (lo, hi) in merged {
		if lo - cursor > EPS {
			solids.push((cursor, lo));
		}
		cursor = cursor.max(hi);
	}
	if length - cursor > EPS {
		solids.push((cursor, length));
	}
	solids
}
