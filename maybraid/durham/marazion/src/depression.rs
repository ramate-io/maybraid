//! Wet-core watershed depressions (stream corridors and lake bowls).
//!
//! A depression owns the **inner carve** and a candidate [`WaterFill`]; the
//! outer apron / rim shelf is applied once by [`crate::complex::WatershedDepressionComplex`].

use crate::fill::WaterFill;
use jersey_terrain_stamps::{JerseyModulation, Region2D};

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

/// One wet core + local vertical recipe (no outer apron).
#[derive(Debug, Clone)]
pub struct WatershedDepression {
	pub kind: WatershedDepressionKind,
	/// Footprint used when unioning wet cores for a complex apron.
	pub wet_core: Region2D,
	/// Inner carve / bed modulations (channel, thalweg, bowl, …).
	pub carve_modulations: Vec<JerseyModulation>,
	/// Candidate water volume for this core.
	pub fill: Option<WaterFill>,
}

impl WatershedDepression {
	pub fn new(
		kind: WatershedDepressionKind,
		wet_core: Region2D,
		carve_modulations: Vec<JerseyModulation>,
		fill: Option<WaterFill>,
	) -> Self {
		Self {
			kind,
			wet_core,
			carve_modulations,
			fill,
		}
	}

	pub fn is_empty(&self) -> bool {
		self.carve_modulations.is_empty() && self.fill.is_none()
	}
}
