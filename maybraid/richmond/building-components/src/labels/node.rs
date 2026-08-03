//! Label IR node: style + geometry + text + placement.

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::labels::geometry::LabelGeometry;
use crate::labels::style::LabelStyle;
use crate::labels::wireframe::LabelWireframeAssets;
use crate::placed::Placement;
use crate::scene_children::{pose, wireframe_box_with_handles};

/// Authoring IR for a debug volume label (colored wireframe; face text is playground-only).
#[derive(Debug, Clone, PartialEq)]
pub struct LabelNode {
	pub style: LabelStyle,
	pub geometry: LabelGeometry,
	pub text: String,
	pub placement: Placement,
}

impl LabelNode {
	pub fn new(
		style: LabelStyle,
		geometry: LabelGeometry,
		text: impl Into<String>,
		placement: Placement,
	) -> Self {
		Self {
			style,
			geometry,
			text: text.into(),
			placement,
		}
	}

	/// Rectangle label whose placement scale matches geometry extents.
	pub fn rectangle(
		style: LabelStyle,
		text: impl Into<String>,
		center: bevy_math::Vec3,
		extents: bevy_math::Vec3,
		yaw: f32,
	) -> Self {
		let extents = extents.max(bevy_math::Vec3::splat(1e-4));
		Self::new(
			style,
			LabelGeometry::rectangle(extents),
			text,
			Placement::new(center, yaw).with_scale(extents),
		)
	}
}

impl LodScene for LabelNode {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_level(
		&self,
		_lod_ref: &LodRef,
		_level: lod::gen::LodSceneLevel,
	) -> impl Scene + 'static {
		let mesh = LabelWireframeAssets::unit_cube();
		let material = LabelWireframeAssets::material_for(self.style);
		// Unit cube is 1³; placement.scale carries full extents.
		wireframe_box_with_handles(mesh, material, pose(self.placement))
	}
}
