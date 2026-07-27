//! Normalized door geometry components (header + 15° kit).

use bevy_math::Vec3;

use crate::doors::geometry::{Door, DoorFrame15, DoorLeaf};
use crate::partitions::geometry_components::WallComponent;
use crate::placed::{IntoGeometryComponents, Placed};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DoorComponent {
	Leaf,
	FramePiece(WallComponent),
}

impl IntoGeometryComponents for Door {
	type Component = DoorComponent;

	fn into_geometry_components(&self) -> Vec<Placed<DoorComponent>> {
		match self {
			Self::Frame15(g) => g.into_geometry_components(),
			Self::Leaf(g) => g.into_geometry_components(),
		}
	}
}

impl IntoGeometryComponents for DoorLeaf {
	type Component = DoorComponent;

	fn into_geometry_components(&self) -> Vec<Placed<DoorComponent>> {
		vec![Placed::at_origin(DoorComponent::Leaf)]
	}
}

impl IntoGeometryComponents for DoorFrame15 {
	type Component = DoorComponent;

	fn into_geometry_components(&self) -> Vec<Placed<DoorComponent>> {
		vec![
			Placed::new(
				DoorComponent::FramePiece(WallComponent::LinearHeaderSubsegment),
				Vec3::new(0.0, 0.85, 0.0),
				0.0,
			),
			Placed::new(
				DoorComponent::FramePiece(WallComponent::HeaderArc15),
				Vec3::new(-0.9, 0.85, 0.0),
				0.0,
			),
			Placed::new(
				DoorComponent::FramePiece(WallComponent::HeaderArc15),
				Vec3::new(0.9, 0.85, 0.0),
				std::f32::consts::PI,
			),
			Placed::new(
				DoorComponent::FramePiece(WallComponent::Arc15),
				Vec3::new(-1.0, 0.0, 0.0),
				0.0,
			),
			Placed::new(
				DoorComponent::FramePiece(WallComponent::Arc15),
				Vec3::new(1.0, 0.0, 0.0),
				std::f32::consts::PI,
			),
		]
	}
}
