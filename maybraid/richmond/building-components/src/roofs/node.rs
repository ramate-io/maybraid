//! Roof IR node: style + geometry + placement.

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::placed::Placement;
use crate::roofs::geometry::RoofGeometry;
use crate::roofs::style::RoofStyle;
use crate::roofs::tessellate::RoofKit;
use crate::roofs::{RoughStonePerchRoof, RoughStoneSpireRoof, WoodPerchDeck};
use crate::scene_children::{pose, scene_children, with_pose};

/// Authoring IR for a roof / cap feature.
#[derive(Debug, Clone, PartialEq)]
pub struct RoofNode {
	pub style: RoofStyle,
	pub geometry: RoofGeometry,
	pub placement: Placement,
}

impl RoofNode {
	pub fn new(style: RoofStyle, geometry: RoofGeometry, placement: Placement) -> Self {
		Self {
			style,
			geometry,
			placement,
		}
	}

	pub fn rough_stone(geometry: RoofGeometry, placement: Placement) -> Self {
		Self::new(RoofStyle::RoughStonework, geometry, placement)
	}

	pub fn wood(geometry: RoofGeometry, placement: Placement) -> Self {
		Self::new(RoofStyle::Wood, geometry, placement)
	}
}

impl LodScene for RoofNode {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.placed_kits(self.placement)
			.into_iter()
			.map(|piece| {
				let transform = pose(piece.placement);
				// Style currently selects among existing leaf placeholders only.
				let child: Box<dyn Scene> = match (self.style, piece.geom) {
					(_, RoofKit::Spire) => Box::new(RoughStoneSpireRoof.scene_with_lod(lod_ref)),
					(_, RoofKit::Perch) => Box::new(RoughStonePerchRoof.scene_with_lod(lod_ref)),
					(_, RoofKit::Deck) => Box::new(WoodPerchDeck.scene_with_lod(lod_ref)),
				};
				Box::new(with_pose(transform, child)) as Box<dyn Scene>
			})
			.collect();
		scene_children(children)
	}
}
