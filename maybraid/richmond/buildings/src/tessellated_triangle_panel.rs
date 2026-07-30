//! World-space triangle filled via shared 2D panel tessellation.
//!
//! Higher-order (buildings) feature — not a kit primitive. Pipeline:
//! 1. Orthonormal frame for the plane of \(A,B,C\)
//! 2. Project corners into panel \(X,Z\)
//! 3. Encode plane as parent [`Placement`] (yaw / **pitch** / roll via YXZ)
//! 4. [`PanelNode`] fills in panel space; kit yaw is composed under that parent

mod corner_markers;

pub use corner_markers::TessellatedTrianglePanelDebugPlugin;

use bevy::scene::prelude::Scene;
use bevy_math::{EulerRot, Mat3, Quat, Vec2, Vec3};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::panels::{
	PanelGeometry, PanelNode, PanelStyle, TessellatedTriangle,
};
use richmond_building_components::scene_children::scene_children;
use richmond_building_components::Placement;

use corner_markers::corner_marker_scenes;

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

	/// Panel-space triangle + parent [`Placement`] (plane yaw / pitch / roll).
	pub fn panel_plane(&self) -> Option<(TessellatedTriangle, Placement)> {
		project_triangle_to_panel_plane(self.a, self.b, self.c)
	}

	fn panel_node(&self) -> Option<PanelNode> {
		let (tri, placement) = self.panel_plane()?;
		Some(PanelNode::new(
			self.style,
			PanelGeometry::tessellated_triangle(tri),
			placement,
		))
	}
}

/// Build an orthonormal panel frame and 2D corners for world triangle \(ABC\).
///
/// - Origin at \(A\)
/// - Panel \(+X\) along \(B - A\)
/// - Panel \(+Y\) = unit normal \((B-A) \times (C-A)\)
/// - Panel \(+Z\) completes the right-handed basis
///
/// The returned [`Placement`] carries that frame as YXZ euler (yaw / **pitch** / roll)
/// so pitch sits on the panel parent.
///
/// Returns [`None`] when the triangle is degenerate.
pub fn project_triangle_to_panel_plane(
	a: Vec3,
	b: Vec3,
	c: Vec3,
) -> Option<(TessellatedTriangle, Placement)> {
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
	let (yaw, pitch, roll) = rotation.to_euler(EulerRot::YXZ);
	let placement = Placement {
		translation: a,
		yaw,
		pitch,
		roll,
		scale: Vec3::ONE,
	};
	Some((TessellatedTriangle::new(Vec2::ZERO, b2, c2), placement))
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
		match self.panel_node() {
			Some(node) => {
				let mut children: Vec<Box<dyn Scene>> =
					vec![Box::new(node.scene_with_level(lod_ref, level))];
				children.extend(corner_marker_scenes(self.a, self.b, self.c));
				Box::new(scene_children(children)) as Box<dyn Scene>
			}
			None => {
				let children = corner_marker_scenes(self.a, self.b, self.c);
				Box::new(scene_children(children)) as Box<dyn Scene>
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Transform;
	use richmond_building_components::scene_children::pose;

	fn assert_vec3_close(got: Vec3, want: Vec3) {
		assert!((got - want).length() < 1e-4, "got {got:?} want {want:?}");
	}

	fn plane_transform(placement: Placement) -> Transform {
		pose(placement)
	}

	#[test]
	fn ground_triangle_projects_flat() {
		let a = Vec3::ZERO;
		let b = Vec3::new(3.0, 0.0, 0.0);
		let c = Vec3::new(0.0, 0.0, 2.0);
		let (tri, placement) = project_triangle_to_panel_plane(a, b, c).expect("plane");
		assert!((tri.a - Vec2::ZERO).length() < 1e-4);
		assert!((tri.b - Vec2::new(3.0, 0.0)).length() < 1e-4);
		assert!(placement.pitch.abs() < 1e-3);

		let plane = plane_transform(placement);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.a.x, 0.0, tri.a.y)), a);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.b.x, 0.0, tri.b.y)), b);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.c.x, 0.0, tri.c.y)), c);
	}

	#[test]
	fn tilted_triangle_has_pitch_on_parent_and_round_trips() {
		let a = Vec3::new(1.0, 2.0, 3.0);
		let b = Vec3::new(4.0, 2.5, 3.0);
		let c = Vec3::new(1.5, 5.0, 6.0);
		let (tri, placement) = project_triangle_to_panel_plane(a, b, c).expect("plane");
		assert!(
			placement.pitch.abs() > 1e-3 || placement.roll.abs() > 1e-3,
			"expected non-flat parent euler, got yaw={} pitch={} roll={}",
			placement.yaw,
			placement.pitch,
			placement.roll
		);
		let plane = plane_transform(placement);
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
	fn vertical_wall_triangle_has_nonzero_pitch() {
		let a = Vec3::ZERO;
		let b = Vec3::new(2.0, 0.0, 0.0);
		let c = Vec3::new(0.0, 3.0, 0.0);
		let (tri, placement) = project_triangle_to_panel_plane(a, b, c).expect("plane");
		assert!(
			placement.pitch.abs() > 1.0,
			"expected large pitch for vertical face, got {}",
			placement.pitch
		);
		let plane = plane_transform(placement);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.b.x, 0.0, tri.b.y)), b);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.c.x, 0.0, tri.c.y)), c);
		let n = plane.rotation * Vec3::Y;
		assert!(n.z.abs() > 0.9, "expected wall normal along ±Z, got {n:?}");
	}
}
