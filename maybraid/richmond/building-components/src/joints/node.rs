//! Joint IR node: style + geometry + placement — fine-phase [`LodScene`] host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Component;
use bevy::scene::prelude::Scene;
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::SceneChunk;

use crate::joints::geometry::JointGeometry;
use crate::joints::rough_stonework::JointLod;
use crate::joints::style::JointStyle;
use crate::lod_band::{placement_bounds, warm_mesh_lod_culls};
use crate::placed::Placement;
use crate::scene_children::pose;

/// Authoring IR for a joint / crease filler.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct JointNode {
	pub style: JointStyle,
	pub geometry: JointGeometry,
	pub placement: Placement,
}

impl JointNode {
	pub fn new(style: JointStyle, geometry: JointGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement }
	}

	pub fn rough_stone(geometry: JointGeometry, placement: Placement) -> Self {
		Self::new(JointStyle::RoughStonework, geometry, placement)
	}

	pub fn rough_stone_post(placement: Placement) -> Self {
		Self::rough_stone(JointGeometry::post(), placement)
	}
}

impl LodScene for JointNode {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		JointLod::level_for_placement(&self.placement, lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = JointLod::level_for_placement(&self.placement, lod_ref.previous_transform);
		let curr = JointLod::level_for_placement(&self.placement, lod_ref.current_transform);
		if prev == curr {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr)
		}
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		warm_mesh_lod_culls(current)
	}

	fn scene_with_level(
		&self,
		_lod_ref: &LodRef,
		level: LodSceneLevel,
	) -> impl Scene + 'static {
		let _ = self.style;
		let _ = self.geometry;
		JointLod::posed_tier(pose(self.placement), level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		placement_bounds(&self.placement)
	}
}
