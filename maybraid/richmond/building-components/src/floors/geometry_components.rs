//! Normalized floor geometry components.

use bevy_math::Vec3;

use crate::floors::geometry::{
	ArcFloorFill, CircleInscribedSquareFloor, Floor, RectangleFloor, StructFloorFill,
};
use crate::partitions::geometry_components::{decompose_arc_sweep, ArcKit};
use crate::placed::{IntoGeometryComponents, Placed};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloorComponent {
	Rectangle,
	ArcFill(ArcKit),
	StructFill,
	CircleInscribedSquare,
}

impl IntoGeometryComponents for Floor {
	type Component = FloorComponent;

	fn into_geometry_components(&self) -> Vec<Placed<FloorComponent>> {
		match self {
			Self::Rectangle(g) => g.into_geometry_components(),
			Self::ArcFill(g) => g.into_geometry_components(),
			Self::StructFill(g) => g.into_geometry_components(),
			Self::CircleInscribedSquare(g) => g.into_geometry_components(),
		}
	}
}

impl IntoGeometryComponents for RectangleFloor {
	type Component = FloorComponent;

	fn into_geometry_components(&self) -> Vec<Placed<FloorComponent>> {
		vec![Placed::at_origin(FloorComponent::Rectangle)]
	}
}

impl IntoGeometryComponents for StructFloorFill {
	type Component = FloorComponent;

	fn into_geometry_components(&self) -> Vec<Placed<FloorComponent>> {
		vec![Placed::at_origin(FloorComponent::StructFill)]
	}
}

impl IntoGeometryComponents for CircleInscribedSquareFloor {
	type Component = FloorComponent;

	fn into_geometry_components(&self) -> Vec<Placed<FloorComponent>> {
		vec![Placed::at_origin(FloorComponent::CircleInscribedSquare)]
	}
}

impl IntoGeometryComponents for ArcFloorFill {
	type Component = FloorComponent;

	fn into_geometry_components(&self) -> Vec<Placed<FloorComponent>> {
		decompose_arc_sweep(self.sweep_degrees)
			.into_iter()
			.map(|(kit, yaw)| Placed::new(FloorComponent::ArcFill(kit), Vec3::ZERO, yaw))
			.collect()
	}
}
