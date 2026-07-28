//! Stair IR node: style + geometry + placement.

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::assets::stairs::rough_stonework::TREAD;
use crate::placed::Placement;
use crate::scene_children::{pose, posed_glb, scene_children, with_pose};
use crate::stairs::geometry::StairGeometry;
use crate::stairs::style::StairStyle;
use crate::stairs::tessellate::StairKit;
use crate::stairs::{RoughStoneSpiralStair, RoughStoneStraightStair, WoodStraightStair};

/// Authoring IR for a stair feature.
#[derive(Debug, Clone, PartialEq)]
pub struct StairNode {
	pub style: StairStyle,
	pub geometry: StairGeometry,
	pub placement: Placement,
}

impl StairNode {
	pub fn new(style: StairStyle, geometry: StairGeometry, placement: Placement) -> Self {
		Self {
			style,
			geometry,
			placement,
		}
	}

	pub fn rough_stone(geometry: StairGeometry, placement: Placement) -> Self {
		Self::new(StairStyle::RoughStonework, geometry, placement)
	}

	pub fn wood(geometry: StairGeometry, placement: Placement) -> Self {
		Self::new(StairStyle::Wood, geometry, placement)
	}
}

impl LodScene for StairNode {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.placed_kits(self.placement)
			.into_iter()
			.map(|piece| {
				let transform = pose(piece.placement);
				match self.style {
					StairStyle::RoughStonework => match piece.geom {
						StairKit::Tread => {
							Box::new(posed_glb(TREAD, transform)) as Box<dyn Scene>
						}
						StairKit::Spiral => Box::new(with_pose(
							transform,
							RoughStoneSpiralStair.scene_with_lod(lod_ref),
						)) as Box<dyn Scene>,
						StairKit::Straight => Box::new(with_pose(
							transform,
							RoughStoneStraightStair.scene_with_lod(lod_ref),
						)) as Box<dyn Scene>,
					},
					StairStyle::Wood => {
						let child: Box<dyn Scene> = match piece.geom {
							StairKit::Tread | StairKit::Spiral => {
								Box::new(RoughStoneSpiralStair.scene_with_lod(lod_ref))
							}
							StairKit::Straight => {
								Box::new(WoodStraightStair.scene_with_lod(lod_ref))
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
