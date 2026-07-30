//! Continuous floor fill geometry.

use crate::panels::{Quad, QuadPolyline};

#[derive(Debug, Clone, PartialEq)]
pub enum FloorGeometry {
	Rectangle(RectangleFloor),
	ArcFill(ArcFloorFill),
	StructFill(StructFloorFill),
	/// Southern circle−square cap; four yaws fill a circular floor ring.
	CircleInscribedSquare(CircleInscribedSquareFloor),
	/// Shared quadrilateral panel (body + up to four edge triangles).
	Quad(Quad),
	/// Short-run polyline of quads + joints.
	QuadPolyline(QuadPolyline),
}

impl FloorGeometry {
	pub fn rectangle() -> Self {
		Self::Rectangle(RectangleFloor)
	}

	pub fn arc_fill(sweep_degrees: f32) -> Self {
		Self::ArcFill(ArcFloorFill { sweep_degrees })
	}

	pub fn struct_fill() -> Self {
		Self::StructFill(StructFloorFill)
	}

	pub fn circle_inscribed_square() -> Self {
		Self::CircleInscribedSquare(CircleInscribedSquareFloor)
	}

	pub fn quad(quad: Quad) -> Self {
		Self::Quad(quad)
	}

	pub fn quad_polyline(polyline: QuadPolyline) -> Self {
		Self::QuadPolyline(polyline)
	}
}

/// Alias kept for migration; prefer [`FloorGeometry`].
pub type Floor = FloorGeometry;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RectangleFloor;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcFloorFill {
	pub sweep_degrees: f32,
}

impl Default for ArcFloorFill {
	fn default() -> Self {
		Self { sweep_degrees: 360.0 }
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StructFloorFill;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CircleInscribedSquareFloor;
