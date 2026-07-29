//! Continuous and tile partition geometry (primitive kit IR — no portals).

mod arc;
mod header_arc;
mod joint;
mod linear;
mod polyline;

pub use arc::ArcSweep;
pub use joint::{
	JointLod, JointPartition, JOINT_BASE_RADIUS, JOINT_HIGH_FACTOR, JOINT_KIT_HALF,
	JOINT_MEDIUM_FACTOR, JOINT_RADIUS_PER_SLOPE_RAD,
};
pub use linear::{
	LinearLod, LinearPartition, DEFAULT_THICK, LINEAR_HIGH_FACTOR, LINEAR_LOW_FACTOR,
	LINEAR_MEDIUM_FACTOR,
};
pub use polyline::{polyline_from_xz, PolylinePartition, DEFAULT_MIN_JOINT_ANGLE};

use bevy_math::Vec3;

use crate::arc_kit::ArcKit;
use crate::placed::{Placed, Placement};

/// Kit-local \(Y\) span of header meshes (\([0, \texttt{HEADER_KIT_HEIGHT}]\)).
pub const HEADER_KIT_HEIGHT: f32 = 0.2;

/// Partition path geometry in world/cell space (continuous size and orientation).
#[derive(Debug, Clone, PartialEq)]
pub enum PartitionGeometry {
	Linear(LinearPartition),
	/// Circular joint tile (\(X,Z \in [-0.5, 0.5]\), \(Y \in [0, 1]\)).
	Joint(JointPartition),
	/// Short-run polyline (single LOD parent). Prefer splitting long paths upstream.
	Polyline(PolylinePartition),
	Arc(ArcSweep),
	/// Header-height arc (\(Y \in [0, [`HEADER_KIT_HEIGHT`]]\) in kit space).
	HeaderArc(ArcSweep),
}

/// Alias for continuous partition geometry.
pub type Partition = PartitionGeometry;

impl PartitionGeometry {
	pub fn linear() -> Self {
		Self::Linear(LinearPartition::default())
	}

	pub fn joint() -> Self {
		Self::Joint(JointPartition::default())
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

	/// Expand into posed leaf tiles under this geometry (identity parent).
	pub fn tiles(&self) -> Vec<Placed<PartitionTile>> {
		match self {
			Self::Linear(_) => vec![Placed::at_origin(PartitionTile::Linear)],
			Self::Joint(_) => vec![Placed::at_origin(PartitionTile::Joint)],
			Self::Polyline(g) => g.tiles(),
			Self::Arc(g) => g.tiles(false),
			Self::HeaderArc(g) => g.tiles(true),
		}
	}

	/// Expand tiles then compose under `parent` placement.
	pub fn placed_tiles(&self, parent: Placement) -> Vec<Placed<PartitionTile>> {
		self.tiles()
			.into_iter()
			.map(|child| Placed {
				geom: child.geom,
				placement: parent.compose_child(child.placement),
			})
			.collect()
	}
}

/// Discrete kit piece after tessellation (not a continuous authoring form).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartitionTile {
	Linear,
	LinearSubsegment,
	LinearHeaderSubsegment,
	Arc180,
	Arc90,
	Arc15,
	HeaderArc180,
	HeaderArc90,
	HeaderArc15,
	Joint,
}

impl From<ArcKit> for PartitionTile {
	fn from(kit: ArcKit) -> Self {
		match kit {
			ArcKit::D180 => Self::Arc180,
			ArcKit::D90 => Self::Arc90,
			ArcKit::D15 => Self::Arc15,
		}
	}
}

pub(crate) fn header_tile(kit: ArcKit) -> PartitionTile {
	match kit {
		ArcKit::D180 => PartitionTile::HeaderArc180,
		ArcKit::D90 => PartitionTile::HeaderArc90,
		ArcKit::D15 => PartitionTile::HeaderArc15,
	}
}

/// Backward-compatible name used by door frame tessellation.
pub(crate) type PartitionKit = PartitionTile;
