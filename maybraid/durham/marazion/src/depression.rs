//! Wet-core watershed depressions (stream corridors and lake bowls).
//!
//! A depression records the **wet-core footprint** for graph bookkeeping /
//! `wet_union`. Vertical recipe and fill live on
//! [`crate::hydro::HydroPrimitive`]s attached to the complex.

use jersey_terrain_stamps::Region2D;

/// Wet-core recipe for one hydrology part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatershedDepressionKind {
	/// Polyline channel / thalweg corridor (stream edge).
	StreamCorridor,
	/// Elliptical radial bowl (lake node).
	LakeBowl,
	/// Optional polyline joint at a graph node (unused in v1 facades).
	JointPolyline,
}

/// One wet core (no outer apron; elevation lives on hydro primitives).
#[derive(Debug, Clone)]
pub struct WatershedDepression {
	pub kind: WatershedDepressionKind,
	/// Footprint used when unioning wet cores for debug / overlays.
	pub wet_core: Region2D,
}

impl WatershedDepression {
	pub fn new(kind: WatershedDepressionKind, wet_core: Region2D) -> Self {
		Self { kind, wet_core }
	}
}
