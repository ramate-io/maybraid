//! Stair IR node: style + geometry + placement — fine-phase [`LodScene`] host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Component;
use bevy::scene::prelude::Scene;
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::SceneChunk;

use crate::assets::stairs::rough_stonework::TREAD;
use crate::lod_band::placement_bounds;
use crate::parent_confines::{confined_scene, ParentConfines};
use crate::placed::Placement;
use crate::scene_children::{pose, posed_glb, scene_children};
use crate::stairs::geometry::StairGeometry;
use crate::stairs::style::StairStyle;
use crate::stairs::tessellate::StairKit;

/// Authoring IR for a stair feature.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct StairNode {
	pub style: StairStyle,
	pub geometry: StairGeometry,
	pub placement: Placement,
	/// External silhouette vs internal detail gating.
	pub confines: ParentConfines,
}

impl StairNode {
	pub fn new(style: StairStyle, geometry: StairGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement, confines: ParentConfines::External }
	}

	pub fn rough_stone(geometry: StairGeometry, placement: Placement) -> Self {
		Self::new(StairStyle::RoughStonework, geometry, placement)
	}

	pub fn wood(geometry: StairGeometry, placement: Placement) -> Self {
		Self::new(StairStyle::Wood, geometry, placement)
	}

	pub fn with_confines(mut self, confines: ParentConfines) -> Self {
		self.confines = confines;
		self
	}
}

impl LodScene for StairNode {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		LodSceneCulls::None
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.placed_kits(self.placement)
			.into_iter()
			.map(|piece| {
				let transform = pose(piece.placement);
				match piece.geom {
					StairKit::Tread => Box::new(posed_glb(TREAD, transform)) as Box<dyn Scene>,
				}
			})
			.collect();
		confined_scene(self.confines, scene_children(children))
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		placement_bounds(&self.placement)
	}
}
