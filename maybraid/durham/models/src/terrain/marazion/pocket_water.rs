//! Shared authored pocket-water stamp enum for Marazion high/low passes.

use crate::terrain::marazion::leaf_kind::MarazionLeafKind;
use marazion_watersheds::{Bog, HydroNode, Lake, Stream, StreamsGraph};

/// Authored Marazion pocket-water stamp held by a pass leaf (not a compiled complex).
#[derive(Debug, Clone)]
pub enum MarazionPocketWater {
	Empty,
	Stream(Stream),
	StreamsGraph(StreamsGraph),
	Bog(Bog),
	Lake(Lake),
}

impl MarazionPocketWater {
	pub fn kind(&self) -> MarazionLeafKind {
		match self {
			Self::Empty => MarazionLeafKind::Empty,
			Self::Stream(_) => MarazionLeafKind::Stream,
			Self::StreamsGraph(_) => MarazionLeafKind::StreamsGraph,
			Self::Bog(_) => MarazionLeafKind::Bog,
			Self::Lake(_) => MarazionLeafKind::Lake,
		}
	}

	/// Hydrology nodes from this authored stamp (empty when unoccupied).
	pub fn hydro_nodes(&self) -> Vec<HydroNode> {
		match self {
			Self::Empty => Vec::new(),
			Self::Stream(s) => s.hydro_nodes(),
			Self::StreamsGraph(g) => g.hydro_nodes(),
			Self::Bog(b) => b.hydro_nodes(),
			Self::Lake(l) => l.hydro_nodes(),
		}
	}

	pub fn is_empty(&self) -> bool {
		matches!(self, Self::Empty)
	}
}
