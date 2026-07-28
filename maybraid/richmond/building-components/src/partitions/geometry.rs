//! Continuous wall / partition geometry.

use bevy_math::Vec2;

/// Wall path geometry in world/cell space (continuous size and orientation).
#[derive(Debug, Clone, PartialEq)]
pub enum Wall {
	Linear(LinearWall),
	Polyline(PolylineWall),
	Arc(ArcWall),
	/// Header-height arc (\(Y \in [0, 0.3]\) in kit space) for door/window frames.
	HeaderArc(ArcWall),
}

impl Wall {
	pub fn linear() -> Self {
		Self::Linear(LinearWall::default())
	}

	pub fn polyline(points: impl Into<Vec<Vec2>>) -> Self {
		Self::Polyline(PolylineWall {
			points: points.into(),
		})
	}

	pub fn arc(sweep_degrees: f32) -> Self {
		Self::Arc(ArcWall { sweep_degrees })
	}

	pub fn header_arc(sweep_degrees: f32) -> Self {
		Self::HeaderArc(ArcWall { sweep_degrees })
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LinearWall;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PolylineWall {
	pub points: Vec<Vec2>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcWall {
	pub sweep_degrees: f32,
}

impl Default for ArcWall {
	fn default() -> Self {
		Self {
			sweep_degrees: 90.0,
		}
	}
}
