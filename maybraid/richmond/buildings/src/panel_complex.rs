//! Triangle mesh of panel points with crease joints on shared edges.
//!
//! Authors insert [`PanelPoint`]s (position + thickness), then list
//! [`PanelTriangle`]s by id. Interior edges (two incident triangles) are
//! found in one pass via canonical undirected keys; a [`JointNode`] is
//! emitted when the dihedral kink meets [`PanelComplexJointPolicy`].

use std::collections::HashMap;

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::{JointNode, JointPost};
use richmond_building_components::panels::{PanelNode, PanelStyle, DEFAULT_MIN_JOINT_ANGLE};
use richmond_building_components::BuildingComponents;

use crate::tessellated_triangle_panel::TessellatedTrianglePanel;

/// Default world thickness matching unscaled panel kits (\(Y \in [-0.2, 0.2]\)).
pub const DEFAULT_PANEL_THICKNESS: f32 = 0.4;

/// Stable point handle inside a [`PanelComplex`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PanelPointId(pub u32);

impl PanelPointId {
	pub const fn new(id: u32) -> Self {
		Self(id)
	}

	pub const fn get(self) -> u32 {
		self.0
	}
}

/// World position + panel thickness at a mesh vertex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelPoint {
	pub position: Vec3,
	pub thickness: f32,
}

impl PanelPoint {
	pub fn new(position: Vec3, thickness: f32) -> Self {
		Self { position, thickness: thickness.max(1e-4) }
	}

	/// Point with [`DEFAULT_PANEL_THICKNESS`].
	pub fn at(position: Vec3) -> Self {
		Self::new(position, DEFAULT_PANEL_THICKNESS)
	}
}

impl From<Vec3> for PanelPoint {
	fn from(position: Vec3) -> Self {
		Self::at(position)
	}
}

/// Ordered triangle of point ids (winding defines the normal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelTriangle {
	pub a: PanelPointId,
	pub b: PanelPointId,
	pub c: PanelPointId,
}

impl PanelTriangle {
	pub fn new(a: PanelPointId, b: PanelPointId, c: PanelPointId) -> Self {
		Self { a, b, c }
	}

	pub fn vertices(self) -> [PanelPointId; 3] {
		[self.a, self.b, self.c]
	}

	/// Three undirected edges as canonical `(min, max)` keys.
	pub fn undirected_edges(self) -> Option<[(PanelPointId, PanelPointId); 3]> {
		let e0 = canonical_edge(self.a, self.b)?;
		let e1 = canonical_edge(self.b, self.c)?;
		let e2 = canonical_edge(self.c, self.a)?;
		Some([e0, e1, e2])
	}

	pub fn contains(self, id: PanelPointId) -> bool {
		self.a == id || self.b == id || self.c == id
	}

	/// Vertex opposite an undirected edge, if that edge is on this triangle.
	pub fn opposite(self, u: PanelPointId, v: PanelPointId) -> Option<PanelPointId> {
		let key = canonical_edge(u, v)?;
		for &(a, b, opp) in &[
			(self.a, self.b, self.c),
			(self.b, self.c, self.a),
			(self.c, self.a, self.b),
		] {
			if canonical_edge(a, b) == Some(key) {
				return Some(opp);
			}
		}
		None
	}
}

/// When to spawn a crease joint from the dihedral kink on a shared edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelComplexJointPolicy {
	/// Spawn a joint when the dihedral kink (radians) is ≥ this threshold.
	pub min_dihedral_rad: f32,
}

impl Default for PanelComplexJointPolicy {
	fn default() -> Self {
		Self { min_dihedral_rad: DEFAULT_MIN_JOINT_ANGLE }
	}
}

impl PanelComplexJointPolicy {
	pub fn always() -> Self {
		Self { min_dihedral_rad: 0.0 }
	}

	pub fn never() -> Self {
		Self { min_dihedral_rad: f32::INFINITY }
	}

	pub fn min_dihedral_rad(min_dihedral_rad: f32) -> Self {
		Self { min_dihedral_rad: min_dihedral_rad.max(0.0) }
	}
}

/// Interior edge shared by exactly two triangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SharedEdge {
	pub a: PanelPointId,
	pub b: PanelPointId,
	pub tri0: usize,
	pub tri1: usize,
}

impl SharedEdge {
	pub fn endpoints(self) -> (PanelPointId, PanelPointId) {
		(self.a, self.b)
	}
}

/// Issues found by [`PanelComplex::validate`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PanelComplexValidation {
	pub unknown_point_refs: Vec<(usize, PanelPointId)>,
	pub degenerate_triangles: Vec<usize>,
	pub non_manifold_edges: Vec<(PanelPointId, PanelPointId)>,
}

impl PanelComplexValidation {
	pub fn is_ok(&self) -> bool {
		self.unknown_point_refs.is_empty()
			&& self.degenerate_triangles.is_empty()
			&& self.non_manifold_edges.is_empty()
	}
}

/// One-pass shared-edge discovery: canonical `(min,max)` keys, \(O(T)\) time.
///
/// Edges with one incidence are boundary (omitted). Edges with two incidences
/// become [`SharedEdge`]. Edges with three or more are recorded in
/// `non_manifold` and omitted from the shared list.
pub fn shared_edges(
	triangles: &[PanelTriangle],
) -> (Vec<SharedEdge>, Vec<(PanelPointId, PanelPointId)>) {
	let mut incidences: HashMap<(PanelPointId, PanelPointId), Vec<usize>> = HashMap::new();
	for (tri_idx, tri) in triangles.iter().enumerate() {
		let Some(edges) = tri.undirected_edges() else {
			continue;
		};
		for key in edges {
			incidences.entry(key).or_default().push(tri_idx);
		}
	}

	let mut shared = Vec::new();
	let mut non_manifold = Vec::new();
	for ((a, b), tris) in incidences {
		match tris.as_slice() {
			[] | [_] => {}
			[tri0, tri1] => shared.push(SharedEdge {
				a,
				b,
				tri0: *tri0,
				tri1: *tri1,
			}),
			_ => non_manifold.push((a, b)),
		}
	}
	(shared, non_manifold)
}

fn canonical_edge(u: PanelPointId, v: PanelPointId) -> Option<(PanelPointId, PanelPointId)> {
	if u == v {
		None
	} else if u < v {
		Some((u, v))
	} else {
		Some((v, u))
	}
}

/// Authored triangle complex → tessellated panels + optional crease joints.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelComplex {
	pub style: PanelStyle,
	/// Dense slot table; removed points become `None` (ids are not reused).
	points: Vec<Option<PanelPoint>>,
	pub triangles: Vec<PanelTriangle>,
	pub joint_policy: PanelComplexJointPolicy,
}

impl PanelComplex {
	pub fn new(style: PanelStyle) -> Self {
		Self {
			style,
			points: Vec::new(),
			triangles: Vec::new(),
			joint_policy: PanelComplexJointPolicy::default(),
		}
	}

	pub fn rough_stone() -> Self {
		Self::new(PanelStyle::RoughStonework)
	}

	pub fn shepherds_thatch() -> Self {
		Self::new(PanelStyle::ShepherdsThatch)
	}

	pub fn with_style(mut self, style: PanelStyle) -> Self {
		self.style = style;
		self
	}

	pub fn set_style(&mut self, style: PanelStyle) -> &mut Self {
		self.style = style;
		self
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.joint_policy = joint_policy;
		self
	}

	pub fn set_joint_policy(&mut self, joint_policy: PanelComplexJointPolicy) -> &mut Self {
		self.joint_policy = joint_policy;
		self
	}

	/// Insert a point with [`DEFAULT_PANEL_THICKNESS`].
	pub fn insert_point(&mut self, position: Vec3) -> PanelPointId {
		self.insert_point_thick(position, DEFAULT_PANEL_THICKNESS)
	}

	/// Insert a point with explicit thickness.
	pub fn insert_point_thick(&mut self, position: Vec3, thickness: f32) -> PanelPointId {
		let id = PanelPointId(self.points.len() as u32);
		self.points.push(Some(PanelPoint::new(position, thickness)));
		id
	}

	/// Owned insert: returns `(self, id)`.
	pub fn with_point(mut self, position: Vec3) -> (Self, PanelPointId) {
		let id = self.insert_point(position);
		(self, id)
	}

	/// Owned insert with thickness.
	pub fn with_point_thick(mut self, position: Vec3, thickness: f32) -> (Self, PanelPointId) {
		let id = self.insert_point_thick(position, thickness);
		(self, id)
	}

	/// Insert many default-thickness points; returns their ids in order.
	pub fn insert_points<I>(&mut self, positions: I) -> Vec<PanelPointId>
	where
		I: IntoIterator<Item = Vec3>,
	{
		positions.into_iter().map(|p| self.insert_point(p)).collect()
	}

	/// Extend with [`PanelPoint`] values; returns their ids in order.
	pub fn extend_points<I>(&mut self, points: I) -> Vec<PanelPointId>
	where
		I: IntoIterator<Item = PanelPoint>,
	{
		points
			.into_iter()
			.map(|p| self.insert_point_thick(p.position, p.thickness))
			.collect()
	}

	pub fn point(&self, id: PanelPointId) -> Option<&PanelPoint> {
		self.points.get(id.0 as usize).and_then(|p| p.as_ref())
	}

	pub fn point_mut(&mut self, id: PanelPointId) -> Option<&mut PanelPoint> {
		self.points.get_mut(id.0 as usize).and_then(|p| p.as_mut())
	}

	/// Present points as `(id, point)` in id order.
	pub fn points(&self) -> impl Iterator<Item = (PanelPointId, &PanelPoint)> + '_ {
		self.points.iter().enumerate().filter_map(|(i, p)| {
			p.as_ref().map(|pt| (PanelPointId(i as u32), pt))
		})
	}

	pub fn triangles(&self) -> &[PanelTriangle] {
		&self.triangles
	}

	pub fn set_point(&mut self, id: PanelPointId, point: PanelPoint) -> &mut Self {
		if let Some(slot) = self.points.get_mut(id.0 as usize) {
			*slot = Some(PanelPoint::new(point.position, point.thickness));
		} else {
			debug_assert!(false, "set_point: unknown id {id:?}");
		}
		self
	}

	pub fn set_position(&mut self, id: PanelPointId, position: Vec3) -> &mut Self {
		if let Some(p) = self.point_mut(id) {
			p.position = position;
		} else {
			debug_assert!(false, "set_position: unknown id {id:?}");
		}
		self
	}

	pub fn set_thickness(&mut self, id: PanelPointId, thickness: f32) -> &mut Self {
		if let Some(p) = self.point_mut(id) {
			p.thickness = thickness.max(1e-4);
		} else {
			debug_assert!(false, "set_thickness: unknown id {id:?}");
		}
		self
	}

	/// Remove a point and any triangles that reference it. Id slots are not reused.
	pub fn remove_point(&mut self, id: PanelPointId) -> &mut Self {
		if let Some(slot) = self.points.get_mut(id.0 as usize) {
			*slot = None;
			self.triangles.retain(|t| !t.contains(id));
		} else {
			debug_assert!(false, "remove_point: unknown id {id:?}");
		}
		self
	}

	pub fn clear_triangles(&mut self) -> &mut Self {
		self.triangles.clear();
		self
	}

	pub fn clear(&mut self) -> &mut Self {
		self.points.clear();
		self.triangles.clear();
		self
	}

	/// Append a triangle. Unknown ids or degeneracy → debug assert and skip.
	pub fn add_triangle(
		&mut self,
		a: PanelPointId,
		b: PanelPointId,
		c: PanelPointId,
	) -> &mut Self {
		if self.point(a).is_none() || self.point(b).is_none() || self.point(c).is_none() {
			debug_assert!(false, "add_triangle: unknown point id");
			return self;
		}
		if a == b || b == c || c == a {
			debug_assert!(false, "add_triangle: degenerate vertex repeat");
			return self;
		}
		self.triangles.push(PanelTriangle::new(a, b, c));
		self
	}

	/// Alias of [`Self::add_triangle`].
	pub fn triangle(
		&mut self,
		a: PanelPointId,
		b: PanelPointId,
		c: PanelPointId,
	) -> &mut Self {
		self.add_triangle(a, b, c)
	}

	pub fn add_triangles<I>(&mut self, tris: I) -> &mut Self
	where
		I: IntoIterator<Item = (PanelPointId, PanelPointId, PanelPointId)>,
	{
		for (a, b, c) in tris {
			self.add_triangle(a, b, c);
		}
		self
	}

	/// Manifold shared edges only (non-manifold omitted).
	pub fn shared_edges(&self) -> Vec<SharedEdge> {
		shared_edges(&self.triangles).0
	}

	pub fn validate(&self) -> PanelComplexValidation {
		let mut report = PanelComplexValidation::default();
		for (i, tri) in self.triangles.iter().enumerate() {
			for id in tri.vertices() {
				if self.point(id).is_none() {
					report.unknown_point_refs.push((i, id));
				}
			}
			if tri.undirected_edges().is_none() {
				report.degenerate_triangles.push(i);
				continue;
			}
			if let (Some(pa), Some(pb), Some(pc)) =
				(self.point(tri.a), self.point(tri.b), self.point(tri.c))
			{
				if triangle_normal(pa.position, pb.position, pc.position).is_none() {
					report.degenerate_triangles.push(i);
				}
			}
		}
		report.non_manifold_edges = shared_edges(&self.triangles).1;
		report
	}

	pub fn triangle_panel(&self, tri: PanelTriangle) -> Option<TessellatedTrianglePanel> {
		let a = self.point(tri.a)?.position;
		let b = self.point(tri.b)?.position;
		let c = self.point(tri.c)?.position;
		Some(TessellatedTrianglePanel::new(self.style, a, b, c))
	}

	pub fn triangle_panels(&self) -> Vec<TessellatedTrianglePanel> {
		self.triangles
			.iter()
			.filter_map(|t| self.triangle_panel(*t))
			.collect()
	}

	/// Average thickness of the two endpoints of a shared edge.
	pub fn edge_thickness(&self, a: PanelPointId, b: PanelPointId) -> Option<f32> {
		let ta = self.point(a)?.thickness;
		let tb = self.point(b)?.thickness;
		Some(0.5 * (ta + tb))
	}

	fn triangle_normal_at(&self, tri_idx: usize) -> Option<Vec3> {
		let tri = self.triangles.get(tri_idx)?;
		let a = self.point(tri.a)?.position;
		let b = self.point(tri.b)?.position;
		let c = self.point(tri.c)?.position;
		triangle_normal(a, b, c)
	}

	/// Dihedral kink (radians) between the two faces of a shared edge.
	pub fn dihedral_kink(&self, edge: SharedEdge) -> Option<f32> {
		let n0 = self.triangle_normal_at(edge.tri0)?;
		let n1 = self.triangle_normal_at(edge.tri1)?;
		Some(n0.dot(n1).clamp(-1.0, 1.0).acos())
	}

	/// Crease [`JointNode`]s for shared edges that meet the joint policy.
	///
	/// Non-manifold edges are skipped (see [`Self::validate`]). Presentation
	/// stays infallible.
	pub fn joint_nodes(&self) -> Vec<JointNode> {
		let mut out = Vec::new();
		for edge in self.shared_edges() {
			let Some(kink) = self.dihedral_kink(edge) else {
				continue;
			};
			if kink < self.joint_policy.min_dihedral_rad {
				continue;
			}
			let Some(thickness) = self.edge_thickness(edge.a, edge.b) else {
				continue;
			};
			let Some(n0) = self.triangle_normal_at(edge.tri0) else {
				continue;
			};
			let Some(n1) = self.triangle_normal_at(edge.tri1) else {
				continue;
			};
			let Some(pa) = self.point(edge.a) else {
				continue;
			};
			let Some(pb) = self.point(edge.b) else {
				continue;
			};
			let radial_hint = n0 + n1;
			let Some(placement) = JointPost::placed_along_crease(
				pa.position,
				pb.position,
				thickness,
				radial_hint,
			) else {
				continue;
			};
			out.push(JointNode::rough_stone_post(placement));
		}
		out
	}
}

fn triangle_normal(a: Vec3, b: Vec3, c: Vec3) -> Option<Vec3> {
	let n = (b - a).cross(c - a);
	let len = n.length();
	if len < 1e-12 {
		None
	} else {
		Some(n / len)
	}
}

impl BuildingComponents for PanelComplex {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Vec<PanelNode> {
		let mut out = Vec::new();
		for panel in self.triangle_panels() {
			out.extend(panel.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, _level: LodSceneLevel) -> Vec<JointNode> {
		self.joint_nodes()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::{EulerRot, Quat};
	use richmond_building_components::BuildingComponents;
	use richmond_building_components::joints::JOINT_KIT_XZ;

	fn folded_quad() -> PanelComplex {
		let mut c = PanelComplex::rough_stone();
		let a0 = c.insert_point_thick(Vec3::ZERO, 0.25);
		let a1 = c.insert_point_thick(Vec3::new(1.0, 0.0, 0.0), 0.25);
		let b0 = c.insert_point_thick(Vec3::new(0.0, 1.0, 0.0), 0.25);
		let b1 = c.insert_point_thick(Vec3::new(0.0, 0.0, 1.0), 0.25);
		c.add_triangle(a0, a1, b1).add_triangle(a0, b1, b0);
		c
	}

	#[test]
	fn builder_quad_emits_two_panels_and_one_shared_edge() {
		let c = folded_quad();
		assert_eq!(c.points().count(), 4);
		assert_eq!(c.triangles().len(), 2);
		assert_eq!(c.panel_nodes_for_level(LodSceneLevel::High).len(), 2);
		let shared = c.shared_edges();
		assert_eq!(shared.len(), 1);
		let (u, v) = shared[0].endpoints();
		// Diagonal a0–b1 = ids 0 and 3.
		assert_eq!(
			canonical_edge(u, v),
			canonical_edge(PanelPointId(0), PanelPointId(3))
		);
	}

	#[test]
	fn folded_joint_aligns_y_and_uses_endpoint_avg_thickness() {
		let c = folded_quad();
		let kink = c.dihedral_kink(c.shared_edges()[0]).expect("kink");
		assert!(
			(kink - std::f32::consts::FRAC_PI_2).abs() < 1e-3,
			"expected ~90° fold, got {kink}"
		);
		let joints = c.joint_nodes();
		assert_eq!(joints.len(), 1);
		let p = &joints[0].placement;
		assert!((p.translation - Vec3::ZERO).length() < 1e-4);
		let diag = Vec3::new(0.0, 0.0, 1.0);
		let rot = Quat::from_euler(EulerRot::YXZ, p.yaw, p.pitch, p.roll);
		let y_axis = rot * Vec3::Y;
		assert!(
			(y_axis - diag).length() < 1e-3 || (y_axis + diag).length() < 1e-3,
			"kit +Y should align with diagonal, got {y_axis:?}"
		);
		assert!((p.scale.y - 1.0).abs() < 1e-4);
		let want_xz = 0.25 / JOINT_KIT_XZ;
		assert!((p.scale.x - want_xz).abs() < 1e-4);
		assert!((p.scale.z - want_xz).abs() < 1e-4);
	}

	#[test]
	fn edge_thickness_averages_endpoints() {
		let mut c = PanelComplex::rough_stone();
		let a = c.insert_point_thick(Vec3::ZERO, 0.2);
		let b = c.insert_point_thick(Vec3::new(1.0, 0.0, 0.0), 0.6);
		let d = c.insert_point(Vec3::new(0.0, 1.0, 0.0));
		let e = c.insert_point(Vec3::new(0.0, 0.0, 1.0));
		c.add_triangle(a, b, e).add_triangle(a, e, d);
		// Shared diagonal a–e: avg(0.2, default 0.4) = 0.3.
		let shared = c.shared_edges();
		assert_eq!(shared.len(), 1);
		assert!((c.edge_thickness(shared[0].a, shared[0].b).unwrap() - 0.3).abs() < 1e-5);
	}

	#[test]
	fn boundary_only_disjoint_triangles_have_no_shared_edges() {
		let mut c = PanelComplex::rough_stone();
		let a = c.insert_point(Vec3::ZERO);
		let b = c.insert_point(Vec3::new(1.0, 0.0, 0.0));
		let d = c.insert_point(Vec3::new(0.0, 0.0, 1.0));
		let e = c.insert_point(Vec3::new(3.0, 0.0, 0.0));
		let f = c.insert_point(Vec3::new(4.0, 0.0, 0.0));
		let g = c.insert_point(Vec3::new(3.0, 0.0, 1.0));
		c.add_triangle(a, b, d).add_triangle(e, f, g);
		assert!(c.shared_edges().is_empty());
		assert!(c.joint_nodes().is_empty());
	}

	#[test]
	fn non_manifold_edge_omitted_from_shared_and_flagged() {
		let mut c = PanelComplex::rough_stone();
		let a = c.insert_point(Vec3::ZERO);
		let b = c.insert_point(Vec3::new(1.0, 0.0, 0.0));
		let p0 = c.insert_point(Vec3::new(0.0, 1.0, 0.0));
		let p1 = c.insert_point(Vec3::new(0.0, 0.0, 1.0));
		let p2 = c.insert_point(Vec3::new(0.0, -1.0, 0.0));
		c.add_triangle(a, b, p0)
			.add_triangle(a, b, p1)
			.add_triangle(a, b, p2);
		let (shared, non_manifold) = shared_edges(c.triangles());
		assert!(shared.is_empty());
		assert_eq!(non_manifold.len(), 1);
		assert_eq!(non_manifold[0], (a, b));
		let report = c.validate();
		assert!(!report.is_ok());
		assert_eq!(report.non_manifold_edges, vec![(a, b)]);
		assert!(c.joint_nodes().is_empty());
	}

	#[test]
	fn subtle_kink_respects_joint_policy() {
		let mut c = PanelComplex::rough_stone();
		let a0 = c.insert_point(Vec3::new(0.5, 0.0, 0.0));
		let a1 = c.insert_point(Vec3::new(2.5, 0.0, 0.0));
		let b0 = c.insert_point(Vec3::new(0.0, 0.3, 3.0));
		let b1 = c.insert_point(Vec3::new(3.0, 0.0, 3.0));
		c.add_triangle(a0, a1, b1).add_triangle(a0, b1, b0);
		let kink = c.dihedral_kink(c.shared_edges()[0]).expect("kink");
		assert!(kink > 0.1 && kink < 0.2, "expected mild kink, got {kink}");
		assert_eq!(c.joint_nodes().len(), 1);
		c.set_joint_policy(PanelComplexJointPolicy::never());
		assert!(c.joint_nodes().is_empty());
		c.set_joint_policy(PanelComplexJointPolicy::min_dihedral_rad(0.2));
		assert!(c.joint_nodes().is_empty());
	}

	#[test]
	fn remove_point_drops_incident_triangles() {
		let mut c = folded_quad();
		let b1 = PanelPointId(3);
		c.remove_point(b1);
		assert!(c.point(b1).is_none());
		assert!(c.triangles().is_empty());
		assert_eq!(c.points().count(), 3);
	}

	#[test]
	fn owned_with_point_builder() {
		let (c, a) = PanelComplex::rough_stone().with_point(Vec3::ZERO);
		let (c, b) = c.with_point(Vec3::new(1.0, 0.0, 0.0));
		let (mut c, d) = c.with_point(Vec3::new(0.0, 0.0, 1.0));
		c.triangle(a, b, d);
		assert_eq!(c.triangles().len(), 1);
		assert_eq!(c.panel_nodes_for_level(LodSceneLevel::High).len(), 1);
	}
}
