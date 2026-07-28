//! Wall IR node: style + geometry + placement.

use bevy::prelude::Transform;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::Quat;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::assets::partitions::rough_stonework::{
	ARC_15, ARC_180, ARC_90, HEADER_15, HEADER_90, LINEAR,
};
use crate::assets::AssetPath;
use crate::partitions::geometry::WallGeometry;
use crate::partitions::rough_stonework::{
	RoughStonework15, RoughStonework180, RoughStonework90, RoughStoneworkHeader15,
	RoughStoneworkHeader180, RoughStoneworkHeader90, RoughStoneworkLinear,
	RoughStoneworkLinearHeaderSubsegment, RoughStoneworkLinearSubsegment,
};
use crate::partitions::style::WallStyle;
use crate::partitions::tessellate::WallKit;
use crate::placed::Placement;
use crate::scene_children;

/// Authoring IR for a wall / partition feature.
#[derive(Debug, Clone, PartialEq)]
pub struct WallNode {
	pub style: WallStyle,
	pub geometry: WallGeometry,
	pub placement: Placement,
}

impl WallNode {
	pub fn new(style: WallStyle, geometry: WallGeometry, placement: Placement) -> Self {
		Self {
			style,
			geometry,
			placement,
		}
	}

	pub fn rough_stone(geometry: WallGeometry, placement: Placement) -> Self {
		Self::new(WallStyle::RoughStonework, geometry, placement)
	}
}

fn pose(placement: Placement) -> Transform {
	Transform::from_translation(placement.translation)
		.with_rotation(Quat::from_rotation_y(placement.yaw))
		.with_scale(placement.scale)
}

fn posed_glb(asset: AssetPath, transform: Transform) -> impl Scene + 'static {
	(
		asset.mesh_ref().scene(),
		bsn! {
			template_value(transform)
		},
	)
}

fn with_pose(transform: Transform, child: impl Scene + 'static) -> impl Scene + 'static {
	(
		child,
		bsn! {
			template_value(transform)
		},
	)
}

/// Scene for a single private wall kit piece (used by door frames).
pub(crate) fn wall_kit_scene(kit: WallKit, lod_ref: &LodRef) -> Box<dyn Scene> {
	match kit {
		WallKit::Linear => Box::new(RoughStoneworkLinear.scene_with_lod(lod_ref)),
		WallKit::LinearSubsegment => {
			Box::new(RoughStoneworkLinearSubsegment.scene_with_lod(lod_ref))
		}
		WallKit::LinearHeaderSubsegment => {
			Box::new(RoughStoneworkLinearHeaderSubsegment.scene_with_lod(lod_ref))
		}
		WallKit::Arc180 => Box::new(RoughStonework180.scene_with_lod(lod_ref)),
		WallKit::Arc90 => Box::new(RoughStonework90.scene_with_lod(lod_ref)),
		WallKit::Arc15 => Box::new(RoughStonework15.scene_with_lod(lod_ref)),
		WallKit::HeaderArc180 => Box::new(RoughStoneworkHeader180.scene_with_lod(lod_ref)),
		WallKit::HeaderArc90 => Box::new(RoughStoneworkHeader90.scene_with_lod(lod_ref)),
		WallKit::HeaderArc15 => Box::new(RoughStoneworkHeader15.scene_with_lod(lod_ref)),
	}
}

fn posed_wall_kit(kit: WallKit, transform: Transform, lod_ref: &LodRef) -> Box<dyn Scene> {
	match kit {
		WallKit::Linear => Box::new(posed_glb(LINEAR, transform)),
		WallKit::Arc180 => Box::new(posed_glb(ARC_180, transform)),
		WallKit::Arc90 => Box::new(posed_glb(ARC_90, transform)),
		WallKit::Arc15 => Box::new(posed_glb(ARC_15, transform)),
		WallKit::HeaderArc90 => Box::new(posed_glb(HEADER_90, transform)),
		WallKit::HeaderArc15 => Box::new(posed_glb(HEADER_15, transform)),
		other => Box::new(with_pose(transform, wall_kit_scene(other, lod_ref))),
	}
}

impl LodScene for WallNode {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.placed_kits(self.placement)
			.into_iter()
			.map(|piece| {
				let transform = pose(piece.placement);
				match self.style {
					WallStyle::RoughStonework => posed_wall_kit(piece.geom, transform, lod_ref),
				}
			})
			.collect();
		scene_children(children)
	}
}
