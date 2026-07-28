//! Private roof kit tessellation (not part of the public IR).

use crate::placed::{Placement, Placed};
use crate::roofs::geometry::RoofGeometry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RoofKit {
	Spire,
	Perch,
	Deck,
}

impl RoofGeometry {
	pub(crate) fn kit_pieces(&self) -> Vec<Placed<RoofKit>> {
		match self {
			Self::Spire(_) => vec![Placed::at_origin(RoofKit::Spire)],
			Self::Perch(_) => vec![Placed::at_origin(RoofKit::Perch)],
			Self::Deck(_) => vec![Placed::at_origin(RoofKit::Deck)],
		}
	}

	pub(crate) fn placed_kits(&self, parent: Placement) -> Vec<Placed<RoofKit>> {
		self.kit_pieces()
			.into_iter()
			.map(|child| Placed {
				geom: child.geom,
				placement: parent.compose_child(child.placement),
			})
			.collect()
	}
}
