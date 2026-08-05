//! Roof IR node: style + geometry + placement.

use bevy::prelude::{Quat, Transform};
use bevy::scene::prelude::Scene;
use lod::gen::{LodScene, LodSceneCulls};
use lod::lod_ref::LodRef;

use crate::arc_kit::ArcKit;
use crate::assets::panels::shepherds_thatch::{RECTANGLE_HIGH, RECTANGLE_LOW, RECTANGLE_MID};
use crate::lod_band::warm_mesh_lod_culls;
use crate::placed::Placement;
use crate::roofs::geometry::RoofGeometry;
use crate::roofs::lod::{leaf_scene_ref_lod, RoofLodProbe};
use crate::roofs::style::RoofStyle;
use crate::roofs::tessellate::RoofKit;
use crate::roofs::{
	ShepherdsThatchDome15, ShepherdsThatchDome180, ShepherdsThatchDome90,
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
		Self { style, geometry, placement }
	}

	pub fn shepherds_thatch(geometry: RoofGeometry, placement: Placement) -> Self {
		Self::new(RoofStyle::ShepherdsThatch, geometry, placement)
	}
}

impl LodScene for RoofNode {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef) -> LodSceneCulls {
		warm_mesh_lod_culls(RoofLodProbe::from_placement(&self.placement).level_for(
			lod_ref.current_transform,
		))
	}

	fn scene_with_level(
		&self,
		lod_ref: &LodRef,
		_level: lod::gen::LodSceneLevel,
	) -> impl Scene + 'static {
		let parent = pose(self.placement);
		let pitch = Transform::from_rotation(Quat::from_rotation_x(self.geometry.pitch_radians()));
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.kit_pieces_for_style(self.style)
			.into_iter()
			.map(|piece| {
				// parent_pose * pitch_x * kit_pose
				let transform = parent * pitch * pose(piece.placement);
				let child: Box<dyn Scene> = match (self.style, piece.geom) {
					(RoofStyle::ShepherdsThatch, RoofKit::RightTriangle { mirror }) => Box::new(
						ShepherdsThatchRightTriangle::scene_with_lod_mirrored(lod_ref, mirror),
					),
					(RoofStyle::ShepherdsThatch, RoofKit::Rectangle) => {
						Box::new(leaf_scene_ref_lod(
							RECTANGLE_HIGH.scene_ref(),
							RECTANGLE_MID.scene_ref(),
							RECTANGLE_LOW.scene_ref(),
							lod_ref,
						))
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
