//! Normalized stair geometry components.

use crate::placed::{IntoGeometryComponents, Placed};
use crate::stairs::geometry::{SpiralStair, Stair, StraightStair};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StairComponent {
	Spiral,
	Straight,
}

impl IntoGeometryComponents for Stair {
	type Component = StairComponent;

	fn into_geometry_components(&self) -> Vec<Placed<StairComponent>> {
		match self {
			Self::Spiral(g) => g.into_geometry_components(),
			Self::Straight(g) => g.into_geometry_components(),
		}
	}
}

impl IntoGeometryComponents for SpiralStair {
	type Component = StairComponent;

	fn into_geometry_components(&self) -> Vec<Placed<StairComponent>> {
		vec![Placed::at_origin(StairComponent::Spiral)]
	}
}

impl IntoGeometryComponents for StraightStair {
	type Component = StairComponent;

	fn into_geometry_components(&self) -> Vec<Placed<StairComponent>> {
		vec![Placed::at_origin(StairComponent::Straight)]
	}
}
