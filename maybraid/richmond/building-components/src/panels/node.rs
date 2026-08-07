//! Panel IR node: style + geometry + placement.

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use scene_ref::MirrorAxis;

use crate::assets::AssetPath;
use crate::panels::geometry::{PanelGeometry, Rectangle, RightTriangle};
use crate::panels::lod::{
	leaf_panel_scene_ref_lod, PanelLodProbe, PANEL_ULTRA_LOW_RECTANGLE,
	PANEL_ULTRA_LOW_RIGHT_TRIANGLE,
};
use crate::panels::style::PanelStyle;
use crate::placed::Placement;
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

fn lod_quad_scene(
	high: AssetPath,
	mid: AssetPath,
	low: AssetPath,
	ultra_low: AssetPath,
	lod_ref: &LodRef,
	placement: &Placement,
	mirror: Option<MirrorAxis>,
) -> impl Scene + 'static {
	leaf_panel_scene_ref_lod(
		high.scene_ref().with_mirror(mirror),
		mid.scene_ref().with_mirror(mirror),
		low.scene_ref().with_mirror(mirror),
		ultra_low.scene_ref().with_mirror(mirror),
		lod_ref,
		PanelLodProbe::from_placement(placement),
	)
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
				// Probe uses composed placement so each kit bands on its own footprint.
				let world_placement = self.placement.compose_child(piece.placement);
				match piece.geom {
					PanelGeometry::Rectangle(Rectangle) => {
						let (high, mid, low) = self.style.rectangle_lod()?;
						Some(Box::new(with_pose(
							transform,
							lod_quad_scene(
								high,
								mid,
								low,
								PANEL_ULTRA_LOW_RECTANGLE,
								lod_ref,
								&world_placement,
								None,
							),
						)) as Box<dyn Scene>)
					}
					PanelGeometry::RightTriangle(RightTriangle { mirror }) => {
						let (high, mid, low) = self.style.right_triangle_lod()?;
						Some(Box::new(with_pose(
							transform,
							lod_quad_scene(
								high,
								mid,
								low,
								PANEL_ULTRA_LOW_RIGHT_TRIANGLE,
								lod_ref,
								&world_placement,
								mirror,
							),
						)) as Box<dyn Scene>)
					}
					_ => None,
				}
			})
			.collect();
		scene_children(children)
	}
}
