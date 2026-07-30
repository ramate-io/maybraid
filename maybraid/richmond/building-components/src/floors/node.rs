//! Floor IR node: style + geometry + placement.

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::assets::floors::rough_stonework::RECTANGLE;
use crate::assets::panels::rough_stonework::INSCRIBED_SQUARE;
use crate::floors::geometry::FloorGeometry;
use crate::floors::style::FloorStyle;
use crate::floors::tessellate::FloorKit;
use crate::floors::{
	RoughStoneFloorArcFill, RoughStoneFloorRightTriangle, RoughStoneFloorStructFill,
	WoodFloorArcFill, WoodFloorRectangle, WoodFloorStructFill,
};
use crate::panels::to_centered_rect_placement;
use crate::parent_confines::{confined_scene, ParentConfines};
use crate::placed::Placement;
use crate::scene_children::{pose, posed_glb, scene_children, with_pose};

/// Authoring IR for a floor slab feature.
#[derive(Debug, Clone, PartialEq)]
pub struct FloorNode {
	pub style: FloorStyle,
	pub geometry: FloorGeometry,
	pub placement: Placement,
	/// External silhouette vs internal detail gating.
	pub confines: ParentConfines,
}

impl FloorNode {
	pub fn new(style: FloorStyle, geometry: FloorGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement, confines: ParentConfines::External }
	}

	pub fn rough_stone(geometry: FloorGeometry, placement: Placement) -> Self {
		Self::new(FloorStyle::RoughStonework, geometry, placement)
	}

	pub fn wood(geometry: FloorGeometry, placement: Placement) -> Self {
		Self::new(FloorStyle::Wood, geometry, placement)
	}

	pub fn with_confines(mut self, confines: ParentConfines) -> Self {
		self.confines = confines;
		self
	}
}

impl LodScene for FloorNode {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_level(
		&self,
		lod_ref: &LodRef,
		_level: lod::gen::LodSceneLevel,
	) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.placed_kits_for_style(self.style, self.placement)
			.into_iter()
			.filter_map(|piece| {
				let transform = match piece.geom {
					FloorKit::Rectangle => pose(to_centered_rect_placement(piece.placement)),
					_ => pose(piece.placement),
				};
				match self.style {
					FloorStyle::RoughStonework => match piece.geom {
						FloorKit::Rectangle => {
							Some(Box::new(posed_glb(RECTANGLE, transform)) as Box<dyn Scene>)
						}
						FloorKit::RightTriangle { mirror } => Some(Box::new(with_pose(
							transform,
							RoughStoneFloorRightTriangle::scene_with_lod_mirrored(lod_ref, mirror),
						)) as Box<dyn Scene>),
						FloorKit::CircleInscribedSquare => {
							Some(Box::new(posed_glb(INSCRIBED_SQUARE, transform)) as Box<dyn Scene>)
						}
						FloorKit::ArcFill(_) => Some(Box::new(with_pose(
							transform,
							RoughStoneFloorArcFill.scene_with_lod(lod_ref),
						)) as Box<dyn Scene>),
						FloorKit::StructFill => Some(Box::new(with_pose(
							transform,
							RoughStoneFloorStructFill.scene_with_lod(lod_ref),
						)) as Box<dyn Scene>),
					},
					FloorStyle::Wood => {
						let child: Box<dyn Scene> = match piece.geom {
							FloorKit::Rectangle
							| FloorKit::CircleInscribedSquare
							| FloorKit::RightTriangle { .. } => {
								Box::new(WoodFloorRectangle.scene_with_lod(lod_ref))
							}
							FloorKit::ArcFill(_) => {
								Box::new(WoodFloorArcFill.scene_with_lod(lod_ref))
							}
							FloorKit::StructFill => {
								Box::new(WoodFloorStructFill.scene_with_lod(lod_ref))
							}
						};
						Some(Box::new(with_pose(transform, child)) as Box<dyn Scene>)
					}
				}
			})
			.collect();
		confined_scene(self.confines, scene_children(children))
	}
}
