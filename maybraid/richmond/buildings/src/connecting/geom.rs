//! Shared plan-path helpers for connecting halls and stairwells.

use bevy_math::{Vec2, Vec3};

use crate::openings::MappedOpening;
use crate::paneling::tube::TubeCrossSectionNode;

pub(crate) const EPS: f32 = 1e-5;

pub(crate) fn opening_to_tube_node(end: MappedOpening) -> Option<TubeCrossSectionNode> {
	let (bl, br, tl, tr) = end.endpoint_corners();
	let orient = normalize_xz(end.orientation)?;
	let right = Vec3::new(-orient.y, 0.0, orient.x);

	let bottom_middle = (bl + br) * 0.5;
	let top_middle = (tl + tr) * 0.5;
	// Vertical span for mid-station lerp; pitched offset is carried by `top_middle`.
	let height = (top_middle.y - bottom_middle.y).abs().max(EPS);

	let bottom_left_width = signed_width(bl, bottom_middle, right);
	let bottom_right_width = signed_width(br, bottom_middle, right);
	let top_left_width = signed_width(tl, top_middle, right);
	let top_right_width = signed_width(tr, top_middle, right);

	Some(
		TubeCrossSectionNode::new(
			bottom_middle,
			bottom_left_width,
			bottom_right_width,
			height,
			top_left_width,
			top_right_width,
		)
		.with_top_middle(top_middle),
	)
}

fn signed_width(corner: Vec3, middle: Vec3, right: Vec3) -> f32 {
	let d = corner - middle;
	let along = d.dot(right);
	// Widths are positive extents along ±right from middle.
	along.abs().max(0.0)
}

pub(crate) fn lerp_tube_nodes(
	a: TubeCrossSectionNode,
	b: TubeCrossSectionNode,
	w_a: f32,
	w_b: f32,
	bottom_middle: Vec3,
) -> TubeCrossSectionNode {
	let mut mid = TubeCrossSectionNode::new(
		bottom_middle,
		w_a * a.bottom_left_width + w_b * b.bottom_left_width,
		w_a * a.bottom_right_width + w_b * b.bottom_right_width,
		w_a * a.height + w_b * b.height,
		w_a * a.top_left_width + w_b * b.top_left_width,
		w_a * a.top_right_width + w_b * b.top_right_width,
	);
	match (a.top_middle, b.top_middle) {
		(Some(ta), Some(tb)) => {
			mid = mid.with_top_middle(ta * w_a + tb * w_b);
		}
		(Some(ta), None) => {
			let tb = b.bottom_middle + Vec3::Y * b.height;
			mid = mid.with_top_middle(ta * w_a + tb * w_b);
		}
		(None, Some(tb)) => {
			let ta = a.bottom_middle + Vec3::Y * a.height;
			mid = mid.with_top_middle(ta * w_a + tb * w_b);
		}
		(None, None) => {}
	}
	mid
}

pub(crate) fn normalize_xz(v: Vec2) -> Option<Vec2> {
	let len = v.length();
	if len < EPS {
		None
	} else {
		Some(v / len)
	}
}

/// Intersect rays `p_a + t d_a` and `p_b + s d_b` in XZ. Returns `(t, s, point)`.
///
/// Collinear anti-parallel openings (facing each other on one line) use the
/// plan midpoint — a zero-kink special case of the one-kink connector.
pub(crate) fn ray_intersect_xz(
	p_a: Vec2,
	d_a: Vec2,
	p_b: Vec2,
	d_b: Vec2,
) -> Option<(f32, f32, Vec2)> {
	let delta = p_b - p_a;
	// det([d_a, -d_b]) = d_b.x*d_a.y - d_a.x*d_b.y
	let det = d_a.y * d_b.x - d_a.x * d_b.y;
	if det.abs() < EPS {
		// Parallel: only succeed when collinear and facing each other.
		let cross = d_a.x * delta.y - d_a.y * delta.x;
		if cross.abs() > EPS {
			return None;
		}
		if d_a.dot(d_b) > -EPS {
			return None;
		}
		let to_b = delta.dot(d_a);
		let to_a = (-delta).dot(d_b);
		if to_b < -EPS || to_a < -EPS {
			return None;
		}
		let point = (p_a + p_b) * 0.5;
		let t = (point - p_a).dot(d_a);
		let s = (point - p_b).dot(d_b);
		return Some((t.max(0.0), s.max(0.0), point));
	}
	let t = (delta.y * d_b.x - delta.x * d_b.y) / det;
	let s = (delta.y * d_a.x - delta.x * d_a.y) / det;
	let point = p_a + d_a * t;
	Some((t, s, point))
}

/// Plan kink between two oriented openings, or the midpoint when rays miss.
pub(crate) fn plan_kink(p_a: Vec2, d_a: Option<Vec2>, p_b: Vec2, d_b: Option<Vec2>) -> Vec2 {
	match (d_a, d_b) {
		(Some(d_a), Some(d_b)) => match ray_intersect_xz(p_a, d_a, p_b, d_b) {
			Some((t, s, m)) if t >= -EPS && s >= -EPS => m,
			_ => (p_a + p_b) * 0.5,
		},
		_ => (p_a + p_b) * 0.5,
	}
}
