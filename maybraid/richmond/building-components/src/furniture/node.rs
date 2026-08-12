//! Furniture IR node: style + geometry + placement — fine-phase [`LodScene`] host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Component;
use bevy::scene::prelude::Scene;
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::SceneChunk;

use crate::furniture::geometry::FurnitureGeometry;
use crate::furniture::style::FurnitureStyle;
use crate::furniture::wireframe::FurnitureWireframeAssets;
use crate::lod_band::placement_bounds;
use crate::placed::Placement;
use crate::scene_children::{pose, wireframe_box_with_handles};

/// Authoring IR for a furniture / fixture placeholder.
#[derive(Debug, Clone, PartialEq, Component, Default)]
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

	pub fn toilet(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Toilet, placement)
	}
}

impl LodScene for FurnitureNode {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		LodSceneCulls::None
	}

	fn scene_with_level(
		&self,
		_lod_ref: &LodRef,
		_level: LodSceneLevel,
	) -> impl Scene + 'static {
		match self.style {
			FurnitureStyle::Placeholder => {
				let mesh = FurnitureWireframeAssets::unit_cube();
				let material = FurnitureWireframeAssets::material_for(self.geometry);
				wireframe_box_with_handles(mesh, material, pose(self.placement))
			}
		}
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		placement_bounds(&self.placement)
	}
}
