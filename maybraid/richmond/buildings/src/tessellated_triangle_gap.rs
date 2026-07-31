//! World-space triangle with one closed cutout → solid annular fill as [`PanelComplex`].
//!
//! Gap vertices are authored in world space and projected onto the triangle plane
//! ([`crate::panel_plane`]). One simple hole (v1): bridge to the outer triangle, then
//! ear-clip. Present via [`Self::into_complex`] — no [`BuildingComponents`] on this type.
//! Strip-with-gaps composition is future work.

use bevy_math::{Vec2, Vec3};
use richmond_building_components::panels::PanelStyle;

use crate::panel_complex::{
	PanelComplex, PanelComplexJointPolicy, PanelPointId, DEFAULT_PANEL_THICKNESS,
};
use crate::panel_plane::{panel_plane_frame, PanelPlaneFrame};

/// Outer world triangle + one closed gap polyline → annular [`PanelComplex`].
#[derive(Debug, Clone, PartialEq)]
pub struct TessellatedTriangleGap {
	pub style: PanelStyle,
	pub a: Vec3,
	pub b: Vec3,
	pub c: Vec3,
	/// Closed gap (first connects to last). World positions; projected at build.
	pub gap: Vec<Vec3>,
	complex: PanelComplex,
}

impl TessellatedTriangleGap {
	/// Build eagerly. Invalid outer / gap → `debug_assert` and solid outer or empty.
	pub fn new(
		style: PanelStyle,
		a: Vec3,
		b: Vec3,
		c: Vec3,
		gap: impl IntoIterator<Item = impl Into<Vec3>>,
	) -> Self {
		let gap: Vec<Vec3> = gap.into_iter().map(Into::into).collect();
		let complex = build_complex(style, a, b, c, &gap, PanelComplexJointPolicy::default());
		Self {
			style,
			a,
			b,
			c,
			gap,
			complex,
		}
	}

	pub fn rough_stone(
		a: Vec3,
		b: Vec3,
		c: Vec3,
		gap: impl IntoIterator<Item = impl Into<Vec3>>,
	) -> Self {
		Self::new(PanelStyle::RoughStonework, a, b, c, gap)
	}

	pub fn shepherds_thatch(
		a: Vec3,
		b: Vec3,
		c: Vec3,
		gap: impl IntoIterator<Item = impl Into<Vec3>>,
	) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, a, b, c, gap)
	}

	/// Rebuild with a new gap, keeping style / corners / joint policy.
	pub fn with_gap(self, gap: impl IntoIterator<Item = impl Into<Vec3>>) -> Self {
		let policy = self.complex.joint_policy;
		let mut next = Self::new(self.style, self.a, self.b, self.c, gap);
		next.complex = next.complex.with_joint_policy(policy);
		next
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.complex = self.complex.with_joint_policy(joint_policy);
		self
	}

	pub fn set_joint_policy(&mut self, joint_policy: PanelComplexJointPolicy) -> &mut Self {
		self.complex.set_joint_policy(joint_policy);
		self
	}

	pub fn as_complex(&self) -> &PanelComplex {
		&self.complex
	}

	pub fn into_complex(self) -> PanelComplex {
		self.complex
	}
}

impl AsRef<PanelComplex> for TessellatedTriangleGap {
	fn as_ref(&self) -> &PanelComplex {
		&self.complex
	}
}

impl From<TessellatedTriangleGap> for PanelComplex {
	fn from(value: TessellatedTriangleGap) -> Self {
		value.into_complex()
	}
}

fn build_complex(
	style: PanelStyle,
	a: Vec3,
	b: Vec3,
	c: Vec3,
	gap: &[Vec3],
	joint_policy: PanelComplexJointPolicy,
) -> PanelComplex {
	let mut complex = PanelComplex::new(style).with_joint_policy(joint_policy);
	let Some(frame) = panel_plane_frame(a, b, c) else {
		debug_assert!(false, "TessellatedTriangleGap: degenerate outer triangle");
		return complex;
	};

	let outer = orient_ccw(frame.outer_2d().to_vec());
	let gap_2d: Vec<Vec2> = gap.iter().copied().map(|p| frame.project(p)).collect();

	let fill_tris = match validate_and_triangulate(&outer, &gap_2d) {
		Some(tris) => tris,
		None => {
			debug_assert!(
				false,
				"TessellatedTriangleGap: invalid gap (need >= 3 verts strictly inside outer); solid fallback"
			);
			vec![[outer[0], outer[1], outer[2]]]
		}
	};

	add_tris_to_complex(&mut complex, &frame, &fill_tris);
	complex
}

fn validate_and_triangulate(outer: &[Vec2], gap: &[Vec2]) -> Option<Vec<[Vec2; 3]>> {
	if gap.len() < 3 {
		return None;
	}
	// Strictly inside outer triangle.
	for &p in gap {
		if !point_in_triangle_strict(p, outer[0], outer[1], outer[2]) {
			return None;
		}
	}
	// Gap edges must not cross outer edges (endpoints inside ⇒ only proper crossings matter).
	let outer_edges = [(outer[0], outer[1]), (outer[1], outer[2]), (outer[2], outer[0])];
	for i in 0..gap.len() {
		let g0 = gap[i];
		let g1 = gap[(i + 1) % gap.len()];
		for &(o0, o1) in &outer_edges {
			if segments_properly_intersect(g0, g1, o0, o1) {
				return None;
			}
		}
	}
	// Non-zero hole area.
	if signed_area(gap).abs() < 1e-10 {
		return None;
	}

	let hole = orient_cw(gap.to_vec());
	let merged = bridge_outer_hole(outer, &hole)?;
	ear_clip(&merged)
}

fn add_tris_to_complex(complex: &mut PanelComplex, frame: &PanelPlaneFrame, tris: &[[Vec2; 3]]) {
	// Dedup points by rounded key so shared ears share PanelPointIds.
	let mut key_to_id: Vec<(u64, PanelPointId)> = Vec::new();
	let mut id_of = |complex: &mut PanelComplex, p: Vec2| -> PanelPointId {
		let key = pack_vec2_key(p);
		if let Some((_, id)) = key_to_id.iter().find(|(k, _)| *k == key) {
			return *id;
		}
		let id = complex.insert_point_thick(frame.unproject(p), DEFAULT_PANEL_THICKNESS);
		key_to_id.push((key, id));
		id
	};
	for tri in tris {
		let ia = id_of(complex, tri[0]);
		let ib = id_of(complex, tri[1]);
		let ic = id_of(complex, tri[2]);
		complex.add_triangle(ia, ib, ic);
	}
}

fn pack_vec2_key(p: Vec2) -> u64 {
	let xi = (p.x * 1e5).round() as i32 as u32;
	let yi = (p.y * 1e5).round() as i32 as u32;
	((xi as u64) << 32) | yi as u64
}

fn signed_area(poly: &[Vec2]) -> f32 {
	let n = poly.len();
	if n < 3 {
		return 0.0;
	}
	let mut a = 0.0;
	for i in 0..n {
		let p = poly[i];
		let q = poly[(i + 1) % n];
		a += p.x * q.y - q.x * p.y;
	}
	a * 0.5
}

fn orient_ccw(mut poly: Vec<Vec2>) -> Vec<Vec2> {
	if signed_area(&poly) < 0.0 {
		poly.reverse();
	}
	poly
}

fn orient_cw(mut poly: Vec<Vec2>) -> Vec<Vec2> {
	if signed_area(&poly) > 0.0 {
		poly.reverse();
	}
	poly
}

fn cross2(o: Vec2, a: Vec2, b: Vec2) -> f32 {
	(a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

fn point_in_triangle_strict(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
	let area = cross2(a, b, c);
	if area.abs() < 1e-12 {
		return false;
	}
	let ab = cross2(a, b, p);
	let bc = cross2(b, c, p);
	let ca = cross2(c, a, p);
	if area > 0.0 {
		ab > 1e-8 && bc > 1e-8 && ca > 1e-8
	} else {
		ab < -1e-8 && bc < -1e-8 && ca < -1e-8
	}
}

fn point_in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
	let area = cross2(a, b, c);
	if area.abs() < 1e-12 {
		return false;
	}
	let ab = cross2(a, b, p);
	let bc = cross2(b, c, p);
	let ca = cross2(c, a, p);
	if area > 0.0 {
		ab >= -1e-8 && bc >= -1e-8 && ca >= -1e-8
	} else {
		ab <= 1e-8 && bc <= 1e-8 && ca <= 1e-8
	}
}

fn on_segment(a: Vec2, b: Vec2, p: Vec2) -> bool {
	p.x >= a.x.min(b.x) - 1e-8
		&& p.x <= a.x.max(b.x) + 1e-8
		&& p.y >= a.y.min(b.y) - 1e-8
		&& p.y <= a.y.max(b.y) + 1e-8
		&& cross2(a, b, p).abs() < 1e-6
}

fn segments_properly_intersect(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
	let d1 = cross2(a, b, c);
	let d2 = cross2(a, b, d);
	let d3 = cross2(c, d, a);
	let d4 = cross2(c, d, b);
	if ((d1 > 1e-8 && d2 < -1e-8) || (d1 < -1e-8 && d2 > 1e-8))
		&& ((d3 > 1e-8 && d4 < -1e-8) || (d3 < -1e-8 && d4 > 1e-8))
	{
		return true;
	}
	false
}

fn segment_intersects_open(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
	// Proper intersection, or overlap interior (not mere shared endpoint).
	if segments_properly_intersect(a, b, c, d) {
		return true;
	}
	// Collinear overlap excluding pure endpoint touches is rare for bridges; skip.
	let _ = (on_segment(a, b, c), on_segment(a, b, d));
	false
}

/// Bridge outer (CCW) to hole (CW); returns a single CCW simple polygon.
fn bridge_outer_hole(outer: &[Vec2], hole: &[Vec2]) -> Option<Vec<Vec2>> {
	if outer.len() < 3 || hole.is_empty() {
		return None;
	}
	// Hole vertex with minimum X, tie-break Y (panel Z).
	let (hj, &h) = hole
		.iter()
		.enumerate()
		.min_by(|(_, p), (_, q)| {
			p.x
				.partial_cmp(&q.x)
				.unwrap_or(std::cmp::Ordering::Equal)
				.then(p.y.partial_cmp(&q.y).unwrap_or(std::cmp::Ordering::Equal))
		})?;

	let mut oi = None;
	for (i, &o) in outer.iter().enumerate() {
		if bridge_visible(o, h, outer, hole) {
			oi = Some(i);
			break;
		}
	}
	// Prefer any outer vertex; fall back to trying all for visibility.
	let oi = oi.or_else(|| {
		(0..outer.len()).find(|&i| bridge_visible(outer[i], h, outer, hole))
	})?;

	let mut poly = Vec::with_capacity(outer.len() + hole.len() + 2);
	for k in 0..outer.len() {
		poly.push(outer[(oi + k) % outer.len()]);
	}
	poly.push(outer[oi]); // close bridge start
	for k in 0..hole.len() {
		poly.push(hole[(hj + k) % hole.len()]);
	}
	poly.push(hole[hj]); // close bridge end

	if signed_area(&poly) < 0.0 {
		poly.reverse();
	}
	Some(poly)
}

fn bridge_visible(o: Vec2, h: Vec2, outer: &[Vec2], hole: &[Vec2]) -> bool {
	if (o - h).length_squared() < 1e-12 {
		return false;
	}
	// Midpoint must lie in the annulus: inside outer, outside hole.
	let mid = (o + h) * 0.5;
	if !point_in_triangle(mid, outer[0], outer[1], outer[2]) {
		return false;
	}
	if point_in_polygon(mid, hole) {
		return false;
	}
	// Must not properly cross hole edges (except at h) or outer edges (except at o).
	for i in 0..hole.len() {
		let a = hole[i];
		let b = hole[(i + 1) % hole.len()];
		if endpoints_touch(o, h, a, b) {
			continue;
		}
		if segment_intersects_open(o, h, a, b) {
			return false;
		}
	}
	for i in 0..outer.len() {
		let a = outer[i];
		let b = outer[(i + 1) % outer.len()];
		if endpoints_touch(o, h, a, b) {
			continue;
		}
		if segment_intersects_open(o, h, a, b) {
			return false;
		}
	}
	true
}

fn endpoints_touch(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
	near(a, c) || near(a, d) || near(b, c) || near(b, d)
}

fn near(a: Vec2, b: Vec2) -> bool {
	(a - b).length_squared() < 1e-12
}

fn point_in_polygon(p: Vec2, poly: &[Vec2]) -> bool {
	// Ray cast + boundary treat as inside for hole rejection of midpoint.
	let n = poly.len();
	if n < 3 {
		return false;
	}
	let mut inside = false;
	let mut j = n - 1;
	for i in 0..n {
		let pi = poly[i];
		let pj = poly[j];
		if on_segment(pj, pi, p) {
			return true;
		}
		let intersect = ((pi.y > p.y) != (pj.y > p.y))
			&& (p.x < (pj.x - pi.x) * (p.y - pi.y) / (pj.y - pi.y + 1e-30) + pi.x);
		if intersect {
			inside = !inside;
		}
		j = i;
	}
	inside
}

fn ear_clip(poly: &[Vec2]) -> Option<Vec<[Vec2; 3]>> {
	let mut verts = poly.to_vec();
	// Remove consecutive duplicates from bridge closure.
	verts.dedup_by(|a, b| near(*a, *b));
	if verts.len() >= 2 && near(verts[0], *verts.last().unwrap()) {
		verts.pop();
	}
	if verts.len() < 3 {
		return None;
	}
	if signed_area(&verts) < 0.0 {
		verts.reverse();
	}

	let mut tris = Vec::new();
	let mut guard = 0;
	while verts.len() > 3 {
		guard += 1;
		if guard > poly.len() * poly.len() + 16 {
			return None;
		}
		let n = verts.len();
		let mut clipped = false;
		for i in 0..n {
			let prev = verts[(i + n - 1) % n];
			let curr = verts[i];
			let next = verts[(i + 1) % n];
			if !is_convex_ear(prev, curr, next) {
				continue;
			}
			if ear_contains_other(&verts, i, prev, curr, next) {
				continue;
			}
			tris.push([prev, curr, next]);
			verts.remove(i);
			clipped = true;
			break;
		}
		if !clipped {
			return None;
		}
	}
	tris.push([verts[0], verts[1], verts[2]]);
	Some(tris)
}

fn is_convex_ear(prev: Vec2, curr: Vec2, next: Vec2) -> bool {
	cross2(prev, curr, next) > 1e-8
}

fn ear_contains_other(verts: &[Vec2], ear_i: usize, a: Vec2, b: Vec2, c: Vec2) -> bool {
	let n = verts.len();
	for (j, &p) in verts.iter().enumerate() {
		if j == ear_i || j == (ear_i + n - 1) % n || j == (ear_i + 1) % n {
			continue;
		}
		// Strict interior only — vertices on an ear edge must not block clipping.
		if point_in_triangle_strict(p, a, b, c) {
			return true;
		}
	}
	false
}

#[cfg(test)]
mod tests {
	use super::*;
	use richmond_building_components::BuildingComponents;
	use lod::gen::LodSceneLevel;

	fn ground_triangle() -> (Vec3, Vec3, Vec3) {
		(
			Vec3::ZERO,
			Vec3::new(3.0, 0.0, 0.0),
			Vec3::new(0.0, 0.0, 2.0),
		)
	}

	fn rect_gap() -> [Vec3; 4] {
		[
			Vec3::new(0.8, 0.0, 0.5),
			Vec3::new(1.4, 0.0, 0.5),
			Vec3::new(1.4, 0.0, 0.9),
			Vec3::new(0.8, 0.0, 0.9),
		]
	}

	#[test]
	fn valid_hole_leaves_centroid_empty() {
		let (a, b, c) = ground_triangle();
		let gap = rect_gap();
		let g = TessellatedTriangleGap::rough_stone(a, b, c, gap);
		assert!(g.as_complex().triangles().len() >= 3);
		let frame = panel_plane_frame(a, b, c).unwrap();
		let centroid = frame.project(
			gap.iter().copied().fold(Vec3::ZERO, |s, p| s + p) / gap.len() as f32,
		);
		for tri in g.as_complex().triangles() {
			let pa = frame.project(g.as_complex().point(tri.a).unwrap().position);
			let pb = frame.project(g.as_complex().point(tri.b).unwrap().position);
			let pc = frame.project(g.as_complex().point(tri.c).unwrap().position);
			assert!(
				!point_in_triangle_strict(centroid, pa, pb, pc),
				"hole centroid should not lie in fill triangle"
			);
		}
	}

	#[test]
	#[cfg(not(debug_assertions))]
	fn short_gap_falls_back_to_solid_outer() {
		let (a, b, c) = ground_triangle();
		let g = TessellatedTriangleGap::rough_stone(a, b, c, [Vec3::new(1.0, 0.0, 0.5)]);
		assert_eq!(g.as_complex().triangles().len(), 1);
	}

	#[test]
	#[cfg(debug_assertions)]
	#[should_panic(expected = "invalid gap")]
	fn short_gap_debug_asserts() {
		let (a, b, c) = ground_triangle();
		let _ = TessellatedTriangleGap::rough_stone(a, b, c, [Vec3::new(1.0, 0.0, 0.5)]);
	}

	#[test]
	#[cfg(not(debug_assertions))]
	fn degenerate_outer_yields_empty() {
		let g = TessellatedTriangleGap::rough_stone(
			Vec3::ZERO,
			Vec3::new(1.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			rect_gap(),
		);
		assert!(g.as_complex().triangles().is_empty());
	}

	#[test]
	#[cfg(debug_assertions)]
	#[should_panic(expected = "degenerate outer triangle")]
	fn degenerate_outer_debug_asserts() {
		let _ = TessellatedTriangleGap::rough_stone(
			Vec3::ZERO,
			Vec3::new(1.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			rect_gap(),
		);
	}

	#[test]
	fn fill_has_shared_edges_and_never_policy_suppresses_joints() {
		let (a, b, c) = ground_triangle();
		let g = TessellatedTriangleGap::rough_stone(a, b, c, rect_gap());
		assert!(
			!g.as_complex().shared_edges().is_empty(),
			"annulus fill should share edges"
		);
		let never = TessellatedTriangleGap::rough_stone(a, b, c, rect_gap())
			.with_joint_policy(PanelComplexJointPolicy::never())
			.into_complex();
		assert!(never.joint_nodes().is_empty());
		assert!(
			never
				.panel_nodes_for_level(LodSceneLevel::High)
				.flatten()
				.len()
				>= 3
		);
	}
}
