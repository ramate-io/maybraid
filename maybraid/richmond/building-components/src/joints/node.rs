//! Joint IR node: style + geometry + placement.

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::joints::geometry::JointGeometry;
use crate::joints::rough_stonework::JointLod;
use crate::joints::style::JointStyle;
use crate::placed::Placement;
use crate::scene_children::pose;

/// Authoring IR for a joint / crease filler.
#[derive(Debug, Clone, PartialEq)]
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
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_level(
		&self,
		_lod_ref: &LodRef,
		level: lod::gen::LodSceneLevel,
	) -> impl Scene + 'static {
		let _ = self.style;
		let _ = self.geometry;
		JointLod::posed_tier(pose(self.placement), level)
	}
}
