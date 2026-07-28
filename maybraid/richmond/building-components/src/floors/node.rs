//! Floor IR node: style + geometry + placement.

use bevy::prelude::Transform;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::Quat;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::assets::floors::rough_stonework::{CIRCLE_INSCRIBED_SQUARE, RECTANGLE};
use crate::assets::AssetPath;
use crate::floors::geometry::FloorGeometry;
use crate::floors::style::FloorStyle;
use crate::floors::tessellate::FloorKit;
use crate::floors::{
	RoughStoneFloorArcFill, RoughStoneFloorStructFill, WoodFloorArcFill, WoodFloorRectangle,
	WoodFloorStructFill,
};
use crate::placed::Placement;
use crate::scene_children;

/// Authoring IR for a floor slab feature.
#[derive(Debug, Clone, PartialEq)]
pub struct FloorNode {
	pub style: FloorStyle,
	pub geometry: FloorGeometry,
	pub placement: Placement,
}

impl FloorNode {
	pub fn new(style: FloorStyle, geometry: FloorGeometry, placement: Placement) -> Self {
		Self {
			style,
			geometry,
			placement,
		}
	}

	pub fn rough_stone(geometry: FloorGeometry, placement: Placement) -> Self {
		Self::new(FloorStyle::RoughStonework, geometry, placement)
	}

	pub fn wood(geometry: FloorGeometry, placement: Placement) -> Self {
		Self::new(FloorStyle::Wood, geometry, placement)
	}
}

fn pose(placement: Placement) -> Transform {
	Transform::from_translation(placement.translation)
		.with_rotation(Quat::from_rotation_y(placement.yaw))
		.with_scale(placement.scale)
}

fn posed_glb(asset: AssetPath, transform: Transform) -> impl Scene + 'static {
	(
		asset.mesh_ref().scene(),
		bsn! {
			template_value(transform)
		},
	)
}

fn with_pose(transform: Transform, child: impl Scene + 'static) -> impl Scene + 'static {
	(
		child,
		bsn! {
			template_value(transform)
		},
	)
}

impl LodScene for FloorNode {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.placed_kits(self.placement)
			.into_iter()
			.map(|piece| {
				let transform = pose(piece.placement);
				match self.style {
					FloorStyle::RoughStonework => match piece.geom {
						FloorKit::Rectangle => {
							Box::new(posed_glb(RECTANGLE, transform)) as Box<dyn Scene>
						}
						FloorKit::CircleInscribedSquare => {
							Box::new(posed_glb(CIRCLE_INSCRIBED_SQUARE, transform)) as Box<dyn Scene>
						}
						FloorKit::ArcFill(_) => Box::new(with_pose(
							transform,
							RoughStoneFloorArcFill.scene_with_lod(lod_ref),
						)) as Box<dyn Scene>,
						FloorKit::StructFill => Box::new(with_pose(
							transform,
							RoughStoneFloorStructFill.scene_with_lod(lod_ref),
						)) as Box<dyn Scene>,
					},
					FloorStyle::Wood => {
						let child: Box<dyn Scene> = match piece.geom {
							FloorKit::Rectangle | FloorKit::CircleInscribedSquare => {
								Box::new(WoodFloorRectangle.scene_with_lod(lod_ref))
							}
							FloorKit::ArcFill(_) => {
								Box::new(WoodFloorArcFill.scene_with_lod(lod_ref))
							}
							FloorKit::StructFill => {
								Box::new(WoodFloorStructFill.scene_with_lod(lod_ref))
							}
						};
						Box::new(with_pose(transform, child)) as Box<dyn Scene>
					}
				}
			})
			.collect();
		scene_children(children)
	}
}
