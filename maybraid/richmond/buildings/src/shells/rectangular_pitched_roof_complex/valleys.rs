//! Pitch-plane valleys and rail truncation at concave corners.

use bevy_math::Vec3;

use super::geometry::{LongAxis, Plane, VolumeCandidate, EPS};
use super::topology::ConcaveCorner;

/// Valley segment from eave meeting point up toward the ridge junction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValleySegment {
	pub eave_point: Vec3,
	/// Representative high point on the valley (lower ridge meet when heights differ).
	pub ridge_point: Vec3,
	pub vol_a: usize,
	pub vol_b: usize,
}

/// Build valleys and truncate candidate rails so neighboring pitches meet on them.
pub(super) fn apply_valleys(
	volumes: &mut [VolumeCandidate],
	corners: &[ConcaveCorner],
) -> Vec<ValleySegment> {
	let mut valleys = Vec::new();
	for corner in corners {
		let Some(valley) = build_valley(volumes, corner) else {
			continue;
		};
		truncate_for_valley(volumes, corner, &valley);
		valleys.push(valley);
	}
	valleys
}

fn build_valley(volumes: &[VolumeCandidate], corner: &ConcaveCorner) -> Option<ValleySegment> {
	let a = &volumes[corner.vol_a];
	let b = &volumes[corner.vol_b];
	debug_assert_eq!(a.long_axis, LongAxis::X);
	debug_assert_eq!(b.long_axis, LongAxis::Z);

	let plane_a = a.pitch_plane(corner.side_a)?;
	let plane_b = b.pitch_plane(corner.side_b)?;
	let (origin, dir) = Plane::intersect(plane_a, plane_b)?;

	// Eave meeting point: expand wall corner by each volume's side overhang.
	let (cx, cz) = corner.corner_xz;
	let y_eave = a.eave[corner.side_a].a.y.min(b.eave[corner.side_b].a.y);
	let eave_x = if corner.side_b == 1 {
		cx + b.side_overhang
	} else {
		cx - b.side_overhang
	};
	let eave_z = if corner.side_a == 1 {
		cz + a.side_overhang
	} else {
		cz - a.side_overhang
	};
	let eave_point = Vec3::new(eave_x, y_eave, eave_z);
	let eave_on_valley = closest_point_on_line(origin, dir, eave_point);

	// Unequal ridges meet by snapping both junction ends to the lower ridge
	// height at the plan crossing (taller ridge banks down). Keeps a closed
	// valley without a vertical gap between ridge ends.
	let ridge_point = lowest_ridge_junction(a, b);

	Some(ValleySegment {
		eave_point: eave_on_valley,
		ridge_point,
		vol_a: corner.vol_a,
		vol_b: corner.vol_b,
	})
}

/// Plan crossing of the two ridges, at the lower ridge height.
fn lowest_ridge_junction(a: &VolumeCandidate, b: &VolumeCandidate) -> Vec3 {
	let y_join = a.ridge.a.y.min(b.ridge.a.y);
	Vec3::new(b.ridge.a.x, y_join, a.ridge.a.z)
}

fn closest_point_on_line(origin: Vec3, dir: Vec3, p: Vec3) -> Vec3 {
	let dir = dir.normalize_or_zero();
	origin + dir * (p - origin).dot(dir)
}

/// Outside (convex) plan corner opposite a concave L corner, expanded by overhangs.
fn outside_eave_corner(volumes: &[VolumeCandidate], corner: &ConcaveCorner) -> Option<Vec3> {
	let a = &volumes[corner.vol_a];
	let b = &volumes[corner.vol_b];
	let (end_a, end_b) = (corner.end_a?, corner.end_b?);
	let outer_a = 1 - corner.side_a;
	let outer_b = 1 - corner.side_b;

	// Start from each volume's outer eave at the junction end, then share XZ.
	let ea = a.eave[outer_a].end(end_a);
	let eb = b.eave[outer_b].end(end_b);
	let y = ea.y.min(eb.y);
	Some(Vec3::new(eb.x, y, ea.z))
}

fn truncate_for_valley(
	volumes: &mut [VolumeCandidate],
	corner: &ConcaveCorner,
	valley: &ValleySegment,
) {
	let ridge_join = valley.ridge_point;

	truncate_long_x(
		&mut volumes[corner.vol_a],
		corner.side_a,
		corner.end_a,
		valley,
		ridge_join,
	);
	truncate_long_z(
		&mut volumes[corner.vol_b],
		corner.side_b,
		corner.end_b,
		valley,
		ridge_join,
	);

	// Close the outside hip: meet outer eaves at the convex corner.
	if let Some(outer) = outside_eave_corner(volumes, corner) {
		if let (Some(end_a), Some(end_b)) = (corner.end_a, corner.end_b) {
			let outer_a = 1 - corner.side_a;
			let outer_b = 1 - corner.side_b;
			let ya = volumes[corner.vol_a].eave[outer_a].end(end_a).y;
			let yb = volumes[corner.vol_b].eave[outer_b].end(end_b).y;
			volumes[corner.vol_a].eave[outer_a].set_end(end_a, Vec3::new(outer.x, ya, outer.z));
			volumes[corner.vol_b].eave[outer_b].set_end(end_b, Vec3::new(outer.x, yb, outer.z));
			let wa = volumes[corner.vol_a].wall[outer_a].end(end_a);
			let wb = volumes[corner.vol_b].wall[outer_b].end(end_b);
			// Walls stay on the massing corner (no side overhang).
			let (cx, cz) = corner_massing_outside(volumes, corner);
			volumes[corner.vol_a].wall[outer_a].set_end(end_a, Vec3::new(cx, wa.y, cz));
			volumes[corner.vol_b].wall[outer_b].set_end(end_b, Vec3::new(cx, wb.y, cz));
		}
	}
}

fn corner_massing_outside(volumes: &[VolumeCandidate], corner: &ConcaveCorner) -> (f32, f32) {
	let a = &volumes[corner.vol_a];
	let b = &volumes[corner.vol_b];
	let (amin_x, amin_z) = a.plan_min();
	let (amax_x, _) = a.plan_max();
	let (bmin_x, bmin_z) = b.plan_min();
	let (_, bmax_z) = b.plan_max();
	// Outside corner shares the non-facing extremes of each arm.
	let x = if corner.side_b == 1 {
		amin_x.min(bmin_x)
	} else {
		amax_x.max(b.plan_max().0)
	};
	let z = if corner.side_a == 1 {
		amin_z.min(bmin_z)
	} else {
		a.plan_max().1.max(bmax_z)
	};
	(x, z)
}

fn truncate_long_x(
	vol: &mut VolumeCandidate,
	side: usize,
	end: Option<usize>,
	valley: &ValleySegment,
	ridge_end: Vec3,
) {
	if let Some(end) = end {
		// Junction ridge end (may bank down to the lower of the two ridges).
		vol.ridge.set_end(end, ridge_end);

		// Facing eave / wall land on the valley. Outer rails are handled separately
		// so the convex corner can form a hip.
		let ey = vol.eave[side].end(end).y;
		vol.eave[side].set_end(
			end,
			Vec3::new(valley.eave_point.x, ey, valley.eave_point.z),
		);
		let wy = vol.wall[side].end(end).y;
		let wz = vol.wall[side].end(end).z;
		vol.wall[side].set_end(end, Vec3::new(ridge_end.x, wy, wz));
	} else {
		// T-bar: snap facing-eave endpoints near the stem onto the valley eave.
		let tol = 0.75 * vol.short_span + vol.side_overhang + EPS;
		for end_i in 0..2 {
			let e = vol.eave[side].end(end_i);
			if (e.x - valley.eave_point.x).abs() < tol {
				vol.eave[side].set_end(
					end_i,
					Vec3::new(valley.eave_point.x, e.y, valley.eave_point.z),
				);
			}
		}
	}
}

fn truncate_long_z(
	vol: &mut VolumeCandidate,
	side: usize,
	end: Option<usize>,
	valley: &ValleySegment,
	ridge_end: Vec3,
) {
	if let Some(end) = end {
		vol.ridge.set_end(end, ridge_end);

		let eave_end = vol.eave[side].end(end);
		vol.eave[side].set_end(
			end,
			Vec3::new(valley.eave_point.x, eave_end.y, valley.eave_point.z),
		);
		let wall_end = vol.wall[side].end(end);
		vol.wall[side].set_end(end, Vec3::new(wall_end.x, wall_end.y, ridge_end.z));
	} else {
		for end_i in 0..2 {
			let e = vol.eave[side].end(end_i);
			if (e.z - valley.eave_point.z).abs()
				< 0.75 * vol.short_span + vol.side_overhang + EPS
			{
				vol.eave[side].set_end(
					end_i,
					Vec3::new(valley.eave_point.x, e.y, valley.eave_point.z),
				);
			}
		}
	}
}
