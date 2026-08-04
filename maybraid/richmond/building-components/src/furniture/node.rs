//! Furniture IR node: style + geometry + placement.

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::furniture::geometry::FurnitureGeometry;
use crate::furniture::style::FurnitureStyle;
use crate::furniture::wireframe::FurnitureWireframeAssets;
use crate::placed::Placement;
use crate::scene_children::{pose, wireframe_box_with_handles};

/// Authoring IR for a furniture / fixture placeholder.
#[derive(Debug, Clone, PartialEq)]
pub struct FurnitureNode {
	pub style: FurnitureStyle,
	pub geometry: FurnitureGeometry,
	pub placement: Placement,
}

impl FurnitureNode {
	pub fn new(style: FurnitureStyle, geometry: FurnitureGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement }
	}

	pub fn placeholder(geometry: FurnitureGeometry, placement: Placement) -> Self {
		Self::new(FurnitureStyle::Placeholder, geometry, placement)
	}

	pub fn bed(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Bed, placement)
	}

	pub fn wardrobe(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Wardrobe, placement)
	}

	pub fn dresser(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Dresser, placement)
	}

	pub fn nightstand(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Nightstand, placement)
	}

	pub fn bedroom_furniture(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::BedroomFurniture, placement)
	}

	pub fn vanity(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Vanity, placement)
	}

	pub fn toilet(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Toilet, placement)
	}
}

impl LodScene for FurnitureNode {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_level(
		&self,
		_lod_ref: &LodRef,
		_level: lod::gen::LodSceneLevel,
	) -> impl Scene + 'static {
		match self.style {
			FurnitureStyle::Placeholder => {
				let mesh = FurnitureWireframeAssets::unit_cube();
				let material = FurnitureWireframeAssets::material_for(self.geometry);
				wireframe_box_with_handles(mesh, material, pose(self.placement))
			}
		}
	}
}
