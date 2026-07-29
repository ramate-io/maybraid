//! Continuous roof geometry primitives.

/// Continuous roof / cap forms. Tessellation into kit pieces is private.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoofGeometry {
	/// Unit right triangles tiled into a rectangle, then pitched about local X.
	RectangularHalfGable(RectangularHalfGable),
	/// Like [`Self::RectangularHalfGable`], but the closing bottom triangle at one
	/// end is scaled to fit a crossing pitch.
	RectangularIntersectingHalfGable(RectangularIntersectingHalfGable),
	/// A single pitched unit right triangle.
	HalfTriangularHip(HalfTriangularHip),
	/// Pitched triangle plus further triangles so the roofline is an edge.
	HalfTrapezoidalHip(HalfTrapezoidalHip),
	/// Dome sweep filled with 180° / 90° / 15° arc kits (empty leaves for now).
	Dome(DomeRoof),
}

impl RoofGeometry {
	pub fn rectangular_half_gable(length_units: u32, pitch_degrees: f32) -> Self {
		Self::RectangularHalfGable(RectangularHalfGable {
			length_units,
			pitch_degrees,
		})
	}

	pub fn rectangular_intersecting_half_gable(
		length_units: u32,
		pitch_degrees: f32,
		end_triangle_scale: f32,
	) -> Self {
		Self::RectangularIntersectingHalfGable(RectangularIntersectingHalfGable {
			length_units,
			pitch_degrees,
			end_triangle_scale,
		})
	}

	pub fn half_triangular_hip(pitch_degrees: f32) -> Self {
		Self::HalfTriangularHip(HalfTriangularHip { pitch_degrees })
	}

	pub fn half_trapezoidal_hip(pitch_degrees: f32, edge_units: u32) -> Self {
		Self::HalfTrapezoidalHip(HalfTrapezoidalHip {
			pitch_degrees,
			edge_units,
		})
	}

	pub fn dome(sweep_degrees: f32) -> Self {
		Self::Dome(DomeRoof { sweep_degrees })
	}

	/// Pitch about local +X in degrees. Domes are unpitched (`0`).
	pub fn pitch_degrees(&self) -> f32 {
		match self {
			Self::RectangularHalfGable(g) => g.pitch_degrees,
			Self::RectangularIntersectingHalfGable(g) => g.pitch_degrees,
			Self::HalfTriangularHip(g) => g.pitch_degrees,
			Self::HalfTrapezoidalHip(g) => g.pitch_degrees,
			Self::Dome(_) => 0.0,
		}
	}
}

/// Alias kept for migration; prefer [`RoofGeometry`].
pub type Roof = RoofGeometry;

/// Rectangular half-gable: `length_units` unit squares along Z, then pitched.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangularHalfGable {
	/// Number of unit squares along the ridge (Z). Clamped to at least 1.
	pub length_units: u32,
	/// Pitch about local +X (degrees).
	pub pitch_degrees: f32,
}

/// Intersecting rectangular half-gable with a scalable end triangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangularIntersectingHalfGable {
	/// Number of unit squares along the ridge (Z). Clamped to at least 1.
	pub length_units: u32,
	/// Pitch about local +X (degrees).
	pub pitch_degrees: f32,
	/// Non-uniform scale applied to the closing bottom triangle at the far end.
	pub end_triangle_scale: f32,
}

/// Half triangular hip: one pitched unit right triangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HalfTriangularHip {
	/// Pitch about local +X (degrees).
	pub pitch_degrees: f32,
}

/// Half trapezoidal hip: base triangle plus `edge_units` further triangles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HalfTrapezoidalHip {
	/// Pitch about local +X (degrees).
	pub pitch_degrees: f32,
	/// Extra triangles beyond the base that form the roofline edge. Clamped to ≥ 1.
	pub edge_units: u32,
}

impl Default for HalfTrapezoidalHip {
	fn default() -> Self {
		Self {
			pitch_degrees: 0.0,
			edge_units: 1,
		}
	}
}

/// Dome roof filled via the shared 180° / 90° / 15° arc kit standard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomeRoof {
	pub sweep_degrees: f32,
}

impl Default for DomeRoof {
	fn default() -> Self {
		Self {
			sweep_degrees: 360.0,
		}
	}
}
