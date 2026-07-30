//! World-space triangle filled via shared 2D panel tessellation.
//!
//! Higher-order (buildings) feature — not a kit primitive. Pipeline:
//! 1. Orthonormal frame for the plane of \(A,B,C\)
//! 2. Project corners into panel \(X,Z\)
//! 3. [`PanelNode`] + [`TessellatedTriangle`] for 2D fill
//! 4. Root pose maps panel space back onto the world plane

use bevy::prelude::Transform;
use bevy::scene::prelude::Scene;
use bevy_math::{Mat3, Quat, Vec2, Vec3};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::panels::{
	PanelGeometry, PanelNode, PanelStyle, TessellatedTriangle,
};
use richmond_building_components::scene_children::posed_scene;
use richmond_building_components::Placement;

/// Three world-space corners filled with posed panel right-triangle kits.
#[derive(Debug, Clone, PartialEq)]
pub struct TessellatedTrianglePanel {
	pub style: PanelStyle,
	pub a: Vec3,
	pub b: Vec3,
	pub c: Vec3,
}

impl TessellatedTrianglePanel {
	pub fn new(style: PanelStyle, a: Vec3, b: Vec3, c: Vec3) -> Self {
		Self { style, a, b, c }
	}

	pub fn rough_stone(a: Vec3, b: Vec3, c: Vec3) -> Self {
		Self::new(PanelStyle::RoughStonework, a, b, c)
	}

	pub fn shepherds_thatch(a: Vec3, b: Vec3, c: Vec3) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, a, b, c)
	}

	/// Panel-space triangle + transform that maps panel \((X,0,Z)\) onto the world plane.
	pub fn panel_plane(&self) -> Option<(TessellatedTriangle, Transform)> {
		project_triangle_to_panel_plane(self.a, self.b, self.c)
	}

	fn panel_node_and_plane(&self) -> Option<(PanelNode, Transform)> {
		let (tri, plane) = self.panel_plane()?;
		let node = PanelNode::new(
			self.style,
			PanelGeometry::tessellated_triangle(tri),
			Placement::IDENTITY,
		);
		Some((node, plane))
	}
}

/// Build an orthonormal panel frame and 2D corners for world triangle \(ABC\).
///
/// - Origin at \(A\)
/// - Panel \(+X\) along \(B - A\)
/// - Panel \(+Y\) = unit normal \((B-A) \times (C-A)\)
/// - Panel \(+Z\) completes the right-handed basis
///
/// Returns [`None`] when the triangle is degenerate.
pub fn project_triangle_to_panel_plane(
	a: Vec3,
	b: Vec3,
	c: Vec3,
) -> Option<(TessellatedTriangle, Transform)> {
	let ab = b - a;
	let ac = c - a;
	let ab_len = ab.length();
	if ab_len < 1e-8 {
		return None;
	}
	let e0 = ab / ab_len;
	let n = ab.cross(ac);
	let n_len = n.length();
	if n_len < 1e-12 {
		return None;
	}
	let normal = n / n_len;
	let e1 = e0.cross(normal);

	let b2 = Vec2::new(ab_len, 0.0);
	let c2 = Vec2::new(ac.dot(e0), ac.dot(e1));
	if (b2.x * c2.y - b2.y * c2.x).abs() < 1e-12 {
		return None;
	}

	let rotation = Quat::from_mat3(&Mat3::from_cols(e0, normal, e1));
	let plane = Transform::from_translation(a).with_rotation(rotation);
	Some((TessellatedTriangle::new(Vec2::ZERO, b2, c2), plane))
}

impl LodScene for TessellatedTrianglePanel {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_level(
		&self,
		lod_ref: &LodRef,
		level: lod::gen::LodSceneLevel,
	) -> impl Scene + 'static {
		match self.panel_node_and_plane() {
			Some((node, plane)) => {
				// `PanelNode` roots with `scene_children` (identity Transform). Parent the
				// plane pose so it is not overwritten by that identity merge.
				Box::new(posed_scene(plane, node.scene_with_level(lod_ref, level)))
					as Box<dyn Scene>
			}
			None => Box::new(::bevy::scene::SceneFunction(empty_scene)) as Box<dyn Scene>,
		}
	}
}

fn empty_scene(_: &mut bevy::scene::ResolveContext, _: &mut bevy::scene::ResolvedScene) {}

#[cfg(test)]
mod tests {
	use super::*;

	fn assert_vec3_close(got: Vec3, want: Vec3) {
		assert!((got - want).length() < 1e-4, "got {got:?} want {want:?}");
	}

	#[test]
	fn ground_triangle_projects_flat() {
		let a = Vec3::ZERO;
		let b = Vec3::new(3.0, 0.0, 0.0);
		let c = Vec3::new(0.0, 0.0, 2.0);
		let (tri, plane) = project_triangle_to_panel_plane(a, b, c).expect("plane");
		assert!((tri.a - Vec2::ZERO).length() < 1e-4);
		assert!((tri.b - Vec2::new(3.0, 0.0)).length() < 1e-4);
		assert!((tri.c.x).abs() < 1e-4);
		assert!((tri.c.y - (-2.0)).abs() < 1e-4 || (tri.c.y - 2.0).abs() < 1e-4);

		assert_vec3_close(plane.transform_point(Vec3::new(tri.a.x, 0.0, tri.a.y)), a);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.b.x, 0.0, tri.b.y)), b);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.c.x, 0.0, tri.c.y)), c);
	}

	#[test]
	fn tilted_triangle_round_trips() {
		let a = Vec3::new(1.0, 2.0, 3.0);
		let b = Vec3::new(4.0, 2.5, 3.0);
		let c = Vec3::new(1.5, 5.0, 6.0);
		let (tri, plane) = project_triangle_to_panel_plane(a, b, c).expect("plane");
		assert_vec3_close(plane.transform_point(Vec3::new(tri.a.x, 0.0, tri.a.y)), a);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.b.x, 0.0, tri.b.y)), b);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.c.x, 0.0, tri.c.y)), c);
	}

	#[test]
	fn degenerate_is_none() {
		assert!(project_triangle_to_panel_plane(
			Vec3::ZERO,
			Vec3::new(1.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
		)
		.is_none());
	}

	#[test]
	fn vertical_wall_triangle_has_nonzero_pitch_basis() {
		let a = Vec3::ZERO;
		let b = Vec3::new(2.0, 0.0, 0.0);
		let c = Vec3::new(0.0, 3.0, 0.0);
		let (tri, plane) = project_triangle_to_panel_plane(a, b, c).expect("plane");
		assert_vec3_close(plane.transform_point(Vec3::new(tri.b.x, 0.0, tri.b.y)), b);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.c.x, 0.0, tri.c.y)), c);
		// Normal should align with ±Z for triangle in XY.
		let n = plane.rotation * Vec3::Y;
		assert!(n.z.abs() > 0.9, "expected wall normal along ±Z, got {n:?}");
	}
}
