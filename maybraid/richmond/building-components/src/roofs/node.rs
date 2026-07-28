//! Roof IR node: style + geometry + placement.

use bevy::prelude::Transform;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::Quat;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::placed::Placement;
use crate::roofs::geometry::RoofGeometry;
use crate::roofs::style::RoofStyle;
use crate::roofs::tessellate::RoofKit;
use crate::roofs::{RoughStonePerchRoof, RoughStoneSpireRoof, WoodPerchDeck};
use crate::scene_children;

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

fn pose(placement: Placement) -> Transform {
	Transform::from_translation(placement.translation)
		.with_rotation(Quat::from_rotation_y(placement.yaw))
		.with_scale(placement.scale)
}

fn with_pose(transform: Transform, child: impl Scene + 'static) -> impl Scene + 'static {
	(
		child,
		bsn! {
			template_value(transform)
		},
	)
}

impl LodScene for RoofNode {
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
