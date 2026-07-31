//! Panel IR node: style + geometry + placement.

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::floors::RoughStoneFloorRightTriangle;
use crate::panels::geometry::{PanelGeometry, Rectangle, RightTriangle};
use crate::panels::rough_stonework::RoughStonePanelRectangle;
use crate::panels::style::PanelStyle;
use crate::placed::Placement;
use crate::roofs::ShepherdsThatchRightTriangle;
use crate::scene_children::{pose, scene_children, with_pose};

/// Authoring IR for a shared panel feature (rectangle / triangle tessellation).
#[derive(Debug, Clone, PartialEq)]
pub struct PanelNode {
	pub style: PanelStyle,
	pub geometry: PanelGeometry,
	pub placement: Placement,
}

impl PanelNode {
	pub fn new(style: PanelStyle, geometry: PanelGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement }
	}

	pub fn rough_stone(geometry: PanelGeometry, placement: Placement) -> Self {
		Self::new(PanelStyle::RoughStonework, geometry, placement)
	}

	pub fn shepherds_thatch(geometry: PanelGeometry, placement: Placement) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, geometry, placement)
	}
}

impl LodScene for PanelNode {
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
			.flatten(self.style.kit_caps())
			.into_iter()
			.filter_map(|piece| {
				// Transform multiply (not euler-add compose_child) so parent pitch/roll
				// compose correctly with in-plane kit yaw.
				let transform = pose(self.placement) * pose(piece.placement);
				match self.style {
					PanelStyle::RoughStonework => match piece.geom {
						PanelGeometry::Rectangle(Rectangle) => {
							Some(Box::new(with_pose(
								transform,
								RoughStonePanelRectangle::scene_with_lod(lod_ref),
							)) as Box<dyn Scene>)
						}
						PanelGeometry::RightTriangle(RightTriangle { mirror }) => {
							Some(Box::new(with_pose(
								transform,
								RoughStoneFloorRightTriangle::scene_with_lod_mirrored(
									lod_ref, mirror,
								),
							)) as Box<dyn Scene>)
						}
						_ => None,
					},
					PanelStyle::ShepherdsThatch => match piece.geom {
						PanelGeometry::Rectangle(Rectangle) => {
							// No thatch rectangle kit; flatten uses dual-triangle policy.
							Some(Box::new(::bevy::scene::SceneFunction(crate::empty_scene))
								as Box<dyn Scene>)
						}
						PanelGeometry::RightTriangle(RightTriangle { mirror }) => {
							Some(Box::new(with_pose(
								transform,
								ShepherdsThatchRightTriangle::scene_with_lod_mirrored(
									lod_ref, mirror,
								),
							)) as Box<dyn Scene>)
						}
						_ => None,
					},
				}
			})
			.collect();
		scene_children(children)
	}
}
