//! Wet-core watershed depressions (stream corridors and lake bowls).
//!
//! A depression owns the **inner carve** and a candidate [`WaterFill`]. Convert it
//! into a [`crate::complex::WatershedDepressionComplex`] with a shared apron via
//! [`Self::into_complex`]. Authored plans (`Lake` / `Stream` / `Bog`) realize that
//! complex; terrain emit compiles modulations from the graph.


use crate::complex::{WatershedApronShelf, WatershedDepressionComplex, WatershedEdge, WatershedNode};
use crate::fill::WaterFill;
use jersey_terrain_stamps::{JerseyModulation, Region2D};
use procedural_common::Bounds2;

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

	/// Lift this single wet core into a one-part [`WatershedDepressionComplex`].
	///
	/// Placement follows [`WatershedDepressionKind`]:
	/// - [`WatershedDepressionKind::LakeBowl`] / [`WatershedDepressionKind::JointPolyline`]
	///   → sole node (no edges)
	/// - [`WatershedDepressionKind::StreamCorridor`] → sole edge between empty hubs
	///
	/// Multi-part pocket complexes that already own several nodes/edges should
	/// build [`WatershedDepressionComplex`] directly instead of going through here.
	pub fn into_complex(
		self,
		bounds: Bounds2,
		seed: u32,
		apron: WatershedApronShelf,
	) -> WatershedDepressionComplex {
		match self.kind {
			WatershedDepressionKind::LakeBowl | WatershedDepressionKind::JointPolyline => {
				let mut complex = WatershedDepressionComplex::new(bounds, seed);
				complex.push_node(WatershedNode::with_depression(self));
				complex.with_apron(apron)
			}
			WatershedDepressionKind::StreamCorridor => {
				let mut complex = WatershedDepressionComplex::new(bounds, seed);
				let from = complex.push_node(WatershedNode::empty());
				let to = complex.push_node(WatershedNode::empty());
				complex.push_edge(WatershedEdge {
					from,
					to,
					depression: self,
				});
				complex.with_apron(apron)
			}
		}
	}
}
