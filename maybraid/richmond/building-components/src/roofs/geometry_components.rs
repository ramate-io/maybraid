//! Normalized roof geometry components.

use crate::placed::{IntoGeometryComponents, Placed};
use crate::roofs::geometry::{PerchDeck, PerchRoof, Roof, SpireRoof};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoofComponent {
	Spire,
	Perch,
	Deck,
}

impl IntoGeometryComponents for Roof {
	type Component = RoofComponent;

	fn into_geometry_components(&self) -> Vec<Placed<RoofComponent>> {
		match self {
			Self::Spire(g) => g.into_geometry_components(),
			Self::Perch(g) => g.into_geometry_components(),
			Self::Deck(g) => g.into_geometry_components(),
		}
	}
}

impl IntoGeometryComponents for SpireRoof {
	type Component = RoofComponent;

	fn into_geometry_components(&self) -> Vec<Placed<RoofComponent>> {
		vec![Placed::at_origin(RoofComponent::Spire)]
	}
}

impl IntoGeometryComponents for PerchRoof {
	type Component = RoofComponent;

	fn into_geometry_components(&self) -> Vec<Placed<RoofComponent>> {
		vec![Placed::at_origin(RoofComponent::Perch)]
	}
}

impl IntoGeometryComponents for PerchDeck {
	type Component = RoofComponent;

	fn into_geometry_components(&self) -> Vec<Placed<RoofComponent>> {
		vec![Placed::at_origin(RoofComponent::Deck)]
	}
}
