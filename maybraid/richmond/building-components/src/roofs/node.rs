//! Roof IR node: style + geometry + placement — fine-phase [`LodScene`] host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Component, Quat, Transform};
use bevy::scene::prelude::Scene;
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::SceneChunk;

use crate::arc_kit::ArcKit;
use crate::assets::panels::shepherds_thatch::{RECTANGLE_HIGH, RECTANGLE_LOW, RECTANGLE_MID};
use crate::assets::roofs::shepherds_thatch::{
	RIGHT_TRIANGLE_HIGH, RIGHT_TRIANGLE_LOW, RIGHT_TRIANGLE_MID,
};
use crate::empty_scene;
use crate::lod_band::{placement_bounds, warm_mesh_lod_culls};
use crate::partitions::mesh_set::PartitionMeshSet;
use crate::partitions::geometry::LinearLod;
use crate::placed::Placement;
use crate::roofs::geometry::RoofGeometry;
use crate::roofs::lod::{roof_scene_ref_for_level, RoofLodProbe};
use crate::roofs::style::RoofStyle;
use crate::roofs::tessellate::RoofKit;
use crate::scene_children::{pose, scene_children, with_pose};

/// Authoring IR for a roof / cap feature.
#[derive(Debug, Clone, PartialEq, Component, Default)]
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

	fn probe(&self) -> RoofLodProbe {
		RoofLodProbe::from_placement(&self.placement)
	}

	fn content_for_level(&self, level: LodSceneLevel) -> impl Scene + 'static {
		let parent = pose(self.placement);
		let pitch = Transform::from_rotation(Quat::from_rotation_x(self.geometry.pitch_radians()));
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.kit_pieces_for_style(self.style)
			.into_iter()
			.map(|piece| {
				let transform = parent * pitch * pose(piece.placement);
				let child: Box<dyn Scene> = match (self.style, piece.geom) {
					(RoofStyle::ShepherdsThatch, RoofKit::RightTriangle { mirror }) => {
						Box::new(LinearLod::posed_mirrored_tier(
							PartitionMeshSet::new(
								RIGHT_TRIANGLE_HIGH,
								RIGHT_TRIANGLE_MID,
								RIGHT_TRIANGLE_LOW,
							),
							Transform::IDENTITY,
							level,
							mirror,
						))
					}
					(RoofStyle::ShepherdsThatch, RoofKit::Rectangle) => {
						Box::new(roof_scene_ref_for_level(
							RECTANGLE_HIGH.scene_ref(),
							RECTANGLE_MID.scene_ref(),
							RECTANGLE_LOW.scene_ref(),
							level,
						))
					}
					(RoofStyle::ShepherdsThatch, RoofKit::DomeArc(ArcKit::D15))
					| (RoofStyle::ShepherdsThatch, RoofKit::DomeArc(ArcKit::D90))
					| (RoofStyle::ShepherdsThatch, RoofKit::DomeArc(ArcKit::D180)) => {
						Box::new(bevy::scene::SceneFunction(empty_scene))
					}
				};
				Box::new(with_pose(transform, child)) as Box<dyn Scene>
			})
			.collect();
		scene_children(children)
	}
}

impl LodScene for RoofNode {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.probe().level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.probe().status_for_lod_ref(lod_ref)
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		warm_mesh_lod_culls(current)
	}

	fn scene_with_level(
		&self,
		_lod_ref: &LodRef,
		level: LodSceneLevel,
	) -> impl Scene + 'static {
		self.content_for_level(level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		placement_bounds(&self.placement)
	}
}
