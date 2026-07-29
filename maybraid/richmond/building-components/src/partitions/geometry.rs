//! Continuous partition geometry (primitive kit IR — no portals).

use bevy_math::{Vec2, Vec3};

/// Kit-local \(Y\) span of header meshes (\([0, \texttt{HEADER_KIT_HEIGHT}]\)).
///
/// Full-height partitions use \(Y \in [0, 1]\). With the same \(Y\) scale \(H\), a header
/// occupies \(0.2\,H\) world height; place its baseline at \(0.8\,H\) to meet the
/// storey top.
pub const HEADER_KIT_HEIGHT: f32 = 0.2;

/// Default polyline joint omission threshold (radians).
///
/// Plan or slope kinks below this are left to abutting linear kits alone.
pub const DEFAULT_MIN_JOINT_ANGLE: f32 = 0.1;

/// Partition path geometry in world/cell space (continuous size and orientation).
#[derive(Debug, Clone, PartialEq)]
pub enum PartitionGeometry {
	Linear(LinearPartition),
	Polyline(PolylinePartition),
	Arc(ArcSweep),
	/// Header-height arc (\(Y \in [0, [`HEADER_KIT_HEIGHT`]]\) in kit space) for door/window frames.
	HeaderArc(ArcSweep),
}

impl PartitionGeometry {
	pub fn linear() -> Self {
		Self::Linear(LinearPartition::default())
	}

	pub fn polyline(points: impl Into<Vec<Vec3>>) -> Self {
		Self::Polyline(PolylinePartition::new(points))
	}

	pub fn arc(sweep_degrees: f32) -> Self {
		Self::Arc(ArcSweep { sweep_degrees })
	}

	pub fn header_arc(sweep_degrees: f32) -> Self {
		Self::HeaderArc(ArcSweep { sweep_degrees })
	}
}

/// Alias for continuous partition geometry.
pub type Partition = PartitionGeometry;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LinearPartition;

#[derive(Debug, Clone, PartialEq)]
pub struct PolylinePartition {
	pub points: Vec<Vec3>,
	/// Omit joint kits when both plan and slope kink angles are below this (radians).
	pub min_joint_angle: f32,
}

impl Default for PolylinePartition {
	fn default() -> Self {
		Self {
			points: Vec::new(),
			min_joint_angle: DEFAULT_MIN_JOINT_ANGLE,
		}
	}
}

impl PolylinePartition {
	pub fn new(points: impl Into<Vec<Vec3>>) -> Self {
		Self {
			points: points.into(),
			min_joint_angle: DEFAULT_MIN_JOINT_ANGLE,
		}
	}

	pub fn with_min_joint_angle(mut self, min_joint_angle: f32) -> Self {
		self.min_joint_angle = min_joint_angle.max(0.0);
		self
	}
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

/// Convert legacy 2D polyline points (XZ) into 3D with \(Y = 0\).
pub fn polyline_from_xz(points: impl IntoIterator<Item = Vec2>) -> PolylinePartition {
	PolylinePartition::new(
		points
			.into_iter()
			.map(|p| Vec3::new(p.x, 0.0, p.y))
			.collect::<Vec<_>>(),
	)
}
