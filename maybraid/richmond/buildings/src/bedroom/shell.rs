//! AABB face → standing [`Rectangle`] / short return walls (paneling kits).

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};
use lod::gen::LodSceneLevel;

use crate::constraints::FaceKind;
use crate::paneling::{PanelPoint, Rectangle};

/// One solid rectangular wall face (or short return).
#[derive(Debug, Clone, PartialEq)]
pub struct ShellWall(pub Rectangle);

impl BuildingComponents for ShellWall {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.0.panel_nodes_for_level(level)
	}
}

/// Full-face wall on `face` when the AABB has positive extent, else [`None`].
pub fn face_rectangle(aabb: &Aabb3d, face: FaceKind, thickness: f32) -> Option<Rectangle> {
	let (a0, a1, b0, b1) = face_quad(aabb, face, 0.0, 1.0)?;
	Some(Rectangle::rough_stone(
		PanelPoint::new(a0, thickness),
		PanelPoint::new(a1, thickness),
		PanelPoint::new(b0, thickness),
		PanelPoint::new(b1, thickness),
	))
}

/// Short return wall on the −side of `face` (door leave), matching legacy 0.35×half span.
pub fn opening_return_rectangle(aabb: &Aabb3d, face: FaceKind, thickness: f32) -> Option<Rectangle> {
	// Legacy: centered stub covering 0.35 of half-span → u in [0, 0.35] of full face.
	face_span_rectangle(aabb, face, 0.0, 0.35, thickness)
}

/// Sub-span of a face in normalized \(u \in [0,1]\) along the face length.
pub fn face_span_rectangle(
	aabb: &Aabb3d,
	face: FaceKind,
	u0: f32,
	u1: f32,
	thickness: f32,
) -> Option<Rectangle> {
	let (a0, a1, b0, b1) = face_quad(aabb, face, u0, u1)?;
	Some(Rectangle::rough_stone(
		PanelPoint::new(a0, thickness),
		PanelPoint::new(a1, thickness),
		PanelPoint::new(b0, thickness),
		PanelPoint::new(b1, thickness),
	))
}

/// Bottom `(a0,a1)` and top `(b0,b1)` corners for face span `[u0,u1]`.
fn face_quad(aabb: &Aabb3d, face: FaceKind, u0: f32, u1: f32) -> Option<(Vec3, Vec3, Vec3, Vec3)> {
	let u0 = u0.clamp(0.0, 1.0);
	let u1 = u1.clamp(0.0, 1.0);
	if u1 - u0 < 1e-4 {
		return None;
	}
	let min = aabb.min;
	let max = aabb.max;
	let y0 = min.y;
	let y1 = max.y;
	if (y1 - y0).abs() < 1e-4 {
		return None;
	}
	let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
	match face {
		FaceKind::Front => {
			// −Z, +X along face
			let z = min.z;
			let a0 = Vec3::new(lerp(min.x, max.x, u0), y0, z);
			let a1 = Vec3::new(lerp(min.x, max.x, u1), y0, z);
			let b0 = Vec3::new(lerp(min.x, max.x, u0), y1, z);
			let b1 = Vec3::new(lerp(min.x, max.x, u1), y1, z);
			Some((a0, a1, b0, b1))
		}
		FaceKind::Back => {
			// +Z, +X along face
			let z = max.z;
			let a0 = Vec3::new(lerp(min.x, max.x, u0), y0, z);
			let a1 = Vec3::new(lerp(min.x, max.x, u1), y0, z);
			let b0 = Vec3::new(lerp(min.x, max.x, u0), y1, z);
			let b1 = Vec3::new(lerp(min.x, max.x, u1), y1, z);
			Some((a0, a1, b0, b1))
		}
		FaceKind::Left => {
			// −X, +Z along face
			let x = min.x;
			let a0 = Vec3::new(x, y0, lerp(min.z, max.z, u0));
			let a1 = Vec3::new(x, y0, lerp(min.z, max.z, u1));
			let b0 = Vec3::new(x, y1, lerp(min.z, max.z, u0));
			let b1 = Vec3::new(x, y1, lerp(min.z, max.z, u1));
			Some((a0, a1, b0, b1))
		}
		FaceKind::Right => {
			// +X, +Z along face
			let x = max.x;
			let a0 = Vec3::new(x, y0, lerp(min.z, max.z, u0));
			let a1 = Vec3::new(x, y0, lerp(min.z, max.z, u1));
			let b0 = Vec3::new(x, y1, lerp(min.z, max.z, u0));
			let b1 = Vec3::new(x, y1, lerp(min.z, max.z, u1));
			Some((a0, a1, b0, b1))
		}
		FaceKind::Top | FaceKind::Bottom => None,
	}
}
