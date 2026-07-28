//! Continuous wall / partition geometry.

use bevy_math::Vec2;

/// Kit-local \(Y\) span of header meshes (\([0, \texttt{HEADER_KIT_HEIGHT}]\)).
///
/// Full-height walls use \(Y \in [0, 1]\). With the same \(Y\) scale \(H\), a header
/// occupies \(0.2\,H\) world height; place its baseline at \(0.8\,H\) to meet the
/// storey top.
pub const HEADER_KIT_HEIGHT: f32 = 0.2;

/// Wall path geometry in world/cell space (continuous size and orientation).
#[derive(Debug, Clone, PartialEq)]
pub enum WallGeometry {
	Linear(LinearWall),
	Polyline(PolylineWall),
	Arc(ArcSweep),
	/// Header-height arc (\(Y \in [0, [`HEADER_KIT_HEIGHT`]]\) in kit space) for door/window frames.
	HeaderArc(ArcSweep),
}

impl WallGeometry {
	pub fn linear() -> Self {
		Self::Linear(LinearWall::default())
	}

	pub fn polyline(points: impl Into<Vec<Vec2>>) -> Self {
		Self::Polyline(PolylineWall {
			points: points.into(),
		})
	}

	pub fn arc(sweep_degrees: f32) -> Self {
		Self::Arc(ArcSweep { sweep_degrees })
	}

	pub fn header_arc(sweep_degrees: f32) -> Self {
		Self::HeaderArc(ArcSweep { sweep_degrees })
	}
}

/// Alias kept for migration; prefer [`WallGeometry`].
pub type Wall = WallGeometry;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LinearWall;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PolylineWall {
	pub points: Vec<Vec2>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcSweep {
	pub sweep_degrees: f32,
}

impl Default for ArcSweep {
	fn default() -> Self {
		Self {
			sweep_degrees: 90.0,
		}
	}
}

/// Alias for continuous arc params (was `ArcWall`).
pub type ArcWall = ArcSweep;
