//! Roof IR node: style + geometry + placement.

use bevy::prelude::{Quat, Transform};
use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::arc_kit::ArcKit;
use crate::placed::Placement;
use crate::roofs::geometry::RoofGeometry;
use crate::roofs::style::RoofStyle;
use crate::roofs::tessellate::RoofKit;
use crate::roofs::{
	ShepherdsThatchDome15, ShepherdsThatchDome90, ShepherdsThatchDome180,
	ShepherdsThatchRightTriangle,
};
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

	pub fn shepherds_thatch(geometry: RoofGeometry, placement: Placement) -> Self {
		Self::new(RoofStyle::ShepherdsThatch, geometry, placement)
	}
}

impl LodScene for RoofNode {
	fn scene_lod_status(
		&self,
		_lod_ref: &LodRef,
	) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_level(
		&self,
		lod_ref: &LodRef,
		_level: lod::gen::LodSceneLevel,
	) -> impl Scene + 'static {
		let parent = pose(self.placement);
		let pitch = Transform::from_rotation(Quat::from_rotation_x(f32::to_radians(
			self.geometry.pitch_degrees(),
		)));
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.kit_pieces()
			.into_iter()
			.map(|piece| {
				// parent_pose * pitch_x * kit_pose
				let transform = parent * pitch * pose(piece.placement);
				let child: Box<dyn Scene> = match (self.style, piece.geom) {
					(RoofStyle::ShepherdsThatch, RoofKit::RightTriangle) => {
						Box::new(ShepherdsThatchRightTriangle.scene_with_lod(lod_ref))
					}
					(RoofStyle::ShepherdsThatch, RoofKit::DomeArc(ArcKit::D15)) => {
						Box::new(ShepherdsThatchDome15.scene_with_lod(lod_ref))
					}
					(RoofStyle::ShepherdsThatch, RoofKit::DomeArc(ArcKit::D90)) => {
						Box::new(ShepherdsThatchDome90.scene_with_lod(lod_ref))
					}
					(RoofStyle::ShepherdsThatch, RoofKit::DomeArc(ArcKit::D180)) => {
						Box::new(ShepherdsThatchDome180.scene_with_lod(lod_ref))
					}
				};
				Box::new(with_pose(transform, child)) as Box<dyn Scene>
			})
			.collect();
		scene_children(children)
	}
}
