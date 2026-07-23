//! Shared authored pocket-water stamp enum for Marazion high/low passes.

use crate::terrain::marazion::leaf_kind::MarazionLeafKind;
use marazion_watersheds::{Bog, HydrologyNode, Lake, Stream, StreamsGraph};

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
	pub fn hydrology_nodes(&self) -> Vec<HydrologyNode> {
		match self {
			Self::Empty => Vec::new(),
			Self::Stream(s) => s.hydrology_nodes(),
			Self::StreamsGraph(g) => g.hydrology_nodes(),
			Self::Bog(b) => b.hydrology_nodes(),
			Self::Lake(l) => l.hydrology_nodes(),
		}
	}

	pub fn is_empty(&self) -> bool {
		matches!(self, Self::Empty)
	}
}
