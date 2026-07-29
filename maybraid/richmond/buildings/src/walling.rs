//! Portal-sensitive wall helpers that emit [`PartitionNode`] collections.
//!
//! Lower-order kit IR lives in `richmond_building_components::partitions` (`Partition*`).
//! These higher-order types plan door/window openings along a path.

pub mod arc;
pub mod linear;
pub mod noisy_polyline;
pub mod polyline;
pub mod portal;

pub use arc::{ArcWall, ArcWallParams};
pub use linear::{LinearWall, LinearWallParams, DEFAULT_PORTAL_WIDTH as LINEAR_DEFAULT_PORTAL_WIDTH};
pub use noisy_polyline::{NoisyPolylineWall, NoisyPolylineWallParams};
pub use polyline::{
	PolylineWall, PolylineWallParams, DEFAULT_PORTAL_WIDTH as POLYLINE_DEFAULT_PORTAL_WIDTH,
};
pub use portal::{
	AssignedPortal, MustAssignPortal, Portal, WallRegion, ArcRegion, SLICE_Y_FRAC,
};

use richmond_building_components::partitions::PartitionNode;

/// Umbrella for portal-sensitive arc / linear / polyline walls.
#[derive(Debug, Clone, PartialEq)]
pub enum Walling {
	Arc(ArcWall),
	Linear(LinearWall),
	Polyline(PolylineWall),
	NoisyPolyline(NoisyPolylineWall),
}

impl Walling {
	pub fn partitions(&self) -> &[PartitionNode] {
		match self {
			Self::Arc(w) => &w.partitions,
			Self::Linear(w) => &w.partitions,
			Self::Polyline(w) => &w.partitions,
			Self::NoisyPolyline(w) => w.partitions(),
		}
	}

	pub fn portals(&self) -> &[AssignedPortal] {
		match self {
			Self::Arc(w) => &w.portals,
			Self::Linear(w) => &w.portals,
			Self::Polyline(w) => &w.portals,
			Self::NoisyPolyline(w) => w.portals(),
		}
	}
}

impl From<ArcWall> for Walling {
	fn from(w: ArcWall) -> Self {
		Self::Arc(w)
	}
}

impl From<LinearWall> for Walling {
	fn from(w: LinearWall) -> Self {
		Self::Linear(w)
	}
}

impl From<PolylineWall> for Walling {
	fn from(w: PolylineWall) -> Self {
		Self::Polyline(w)
	}
}

impl From<NoisyPolylineWall> for Walling {
	fn from(w: NoisyPolylineWall) -> Self {
		Self::NoisyPolyline(w)
	}
}
