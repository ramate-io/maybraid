//! Pitch-plane valleys and rail truncation at concave corners.

use bevy_math::Vec3;

use super::geometry::{LongAxis, Plane, VolumeCandidate, EPS};
use super::topology::ConcaveCorner;

/// Valley segment from eave meeting point up to the ridge junction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValleySegment {
	pub eave_point: Vec3,
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
	let y_eave = a.eave[corner.side_a].a.y;
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

	// Ridge junction: A's ridge z × B's ridge x at shared ridge height (lerp if needed).
	let ridge_x = b.ridge.mid().x;
	let ridge_z = a.ridge.mid().z;
	let y_a = a.ridge.a.y;
	let y_b = b.ridge.a.y;
	let y_ridge = 0.5 * (y_a + y_b);
	let mut ridge_point = Vec3::new(ridge_x, y_ridge, ridge_z);

	// Snap ridge_point onto the valley line (closest point on the infinite line).
	ridge_point = closest_point_on_line(origin, dir, ridge_point);
	// Prefer the eave_point projection onto the valley for the low end.
	let eave_on_valley = closest_point_on_line(origin, dir, eave_point);

	Some(ValleySegment {
		eave_point: eave_on_valley,
		ridge_point,
		vol_a: corner.vol_a,
		vol_b: corner.vol_b,
	})
}

fn closest_point_on_line(origin: Vec3, dir: Vec3, p: Vec3) -> Vec3 {
	let dir = dir.normalize_or_zero();
	origin + dir * (p - origin).dot(dir)
}

fn truncate_for_valley(
	volumes: &mut [VolumeCandidate],
	corner: &ConcaveCorner,
	valley: &ValleySegment,
) {
	// Truncate long-X volume A toward the junction end (or side attachment for T-bar).
	truncate_long_x(
		&mut volumes[corner.vol_a],
		corner.side_a,
		corner.end_a,
		valley,
	);
	truncate_long_z(
		&mut volumes[corner.vol_b],
		corner.side_b,
		corner.end_b,
		valley,
	);
}

fn truncate_long_x(
	vol: &mut VolumeCandidate,
	side: usize,
	end: Option<usize>,
	valley: &ValleySegment,
) {
	let jx = valley.ridge_point.x;
	if let Some(end) = end {
		set_long_param_x(vol, end, jx, valley.ridge_point.y);
		// Facing eave junction end lands on the valley eave point.
		let ey = vol.eave[side].end(end).y;
		vol.eave[side].set_end(
			end,
			Vec3::new(valley.eave_point.x, ey, valley.eave_point.z),
		);
		let wy = vol.wall[side].end(end).y;
		let wz = vol.wall[side].end(end).z;
		vol.wall[side].set_end(end, Vec3::new(jx, wy, wz));
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
) {
	let jz = valley.ridge_point.z;
	if let Some(end) = end {
		set_long_param_z(vol, end, jz, valley.ridge_point.y);
		let eave_end = vol.eave[side].end(end);
		vol.eave[side].set_end(
			end,
			Vec3::new(eave_end.x, eave_end.y, valley.eave_point.z),
		);
		let wall_end = vol.wall[side].end(end);
		vol.wall[side].set_end(end, Vec3::new(wall_end.x, wall_end.y, jz));
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

fn set_long_param_x(vol: &mut VolumeCandidate, end: usize, x: f32, y_ridge: f32) {
	let mut r = vol.ridge.end(end);
	r.x = x;
	r.y = y_ridge;
	vol.ridge.set_end(end, r);
	for i in 0..2 {
		let mut e = vol.eave[i].end(end);
		e.x = x;
		vol.eave[i].set_end(end, e);
		let mut w = vol.wall[i].end(end);
		w.x = x;
		vol.wall[i].set_end(end, w);
	}
}

fn set_long_param_z(vol: &mut VolumeCandidate, end: usize, z: f32, y_ridge: f32) {
	let mut r = vol.ridge.end(end);
	r.z = z;
	r.y = y_ridge;
	vol.ridge.set_end(end, r);
	for i in 0..2 {
		let mut e = vol.eave[i].end(end);
		e.z = z;
		vol.eave[i].set_end(end, e);
		let mut w = vol.wall[i].end(end);
		w.z = z;
		vol.wall[i].set_end(end, w);
	}
}
