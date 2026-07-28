//! Wall IR node: style + geometry + placement.

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::{LodScene, LodSceneStatus};
use lod::lod_ref::LodRef;

use crate::assets::partitions::rough_stonework::{
	ARC_15, ARC_180_HIGH, ARC_180_LOW, ARC_180_MID, ARC_90_HIGH, ARC_90_LOW, ARC_90_MID,
	HEADER_15_HIGH, HEADER_15_LOW, HEADER_15_MID, HEADER_90_HIGH, HEADER_90_LOW, HEADER_90_MID,
	LINEAR, LINEAR_HIGH, LINEAR_LOW, LINEAR_MID,
};
use crate::partitions::geometry::WallGeometry;
use crate::partitions::lod::{
	band_for_placement, lod_status_for_placement, posed_partition_mesh_lod, PartitionMeshSet,
};
use crate::partitions::rough_stonework::{
	RoughStonework15, RoughStonework180, RoughStonework90, RoughStoneworkHeader15,
	RoughStoneworkHeader180, RoughStoneworkHeader90, RoughStoneworkLinear,
	RoughStoneworkLinearHeaderSubsegment, RoughStoneworkLinearSubsegment,
};
use crate::partitions::style::WallStyle;
use crate::partitions::tessellate::WallKit;
use crate::placed::Placement;
use crate::scene_children::{pose, scene_children, with_pose};

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

	/// Status as if a rough-stone linear partition sat at `center` with `extent` scale.
	///
	/// Parents use this instead of OR-ing every child wall's [`LodScene::scene_lod_status`].
	pub fn representative_lod_status(
		center: Vec3,
		extent: Vec3,
		lod_ref: &LodRef,
	) -> LodSceneStatus {
		let node = Self::rough_stone(
			WallGeometry::linear(),
			Placement::new(center, 0.0).with_scale(extent.max(Vec3::splat(1e-4))),
		);
		node.scene_lod_status(lod_ref)
	}
}

pub(crate) fn kit_mesh_set(kit: WallKit) -> PartitionMeshSet {
	match kit {
		WallKit::Linear => PartitionMeshSet::new(LINEAR_HIGH, LINEAR_MID, LINEAR_LOW),
		WallKit::Arc180 => PartitionMeshSet::new(ARC_180_HIGH, ARC_180_MID, ARC_180_LOW),
		WallKit::Arc90 => PartitionMeshSet::new(ARC_90_HIGH, ARC_90_MID, ARC_90_LOW),
		WallKit::Arc15 => PartitionMeshSet::uniform(ARC_15),
		WallKit::HeaderArc90 => PartitionMeshSet::new(HEADER_90_HIGH, HEADER_90_MID, HEADER_90_LOW),
		WallKit::HeaderArc15 => PartitionMeshSet::new(HEADER_15_HIGH, HEADER_15_MID, HEADER_15_LOW),
		WallKit::LinearSubsegment
		| WallKit::LinearHeaderSubsegment
		| WallKit::HeaderArc180 => PartitionMeshSet::uniform(LINEAR),
	}
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

impl LodScene for WallNode {
	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		lod_status_for_placement(&self.placement, lod_ref)
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let tier = band_for_placement(&self.placement, lod_ref.current_transform).mesh_tier();
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.placed_kits(self.placement)
			.into_iter()
			.map(|piece| {
				let transform = pose(piece.placement);
				match self.style {
					WallStyle::RoughStonework => match piece.geom {
						WallKit::Linear
						| WallKit::Arc180
						| WallKit::Arc90
						| WallKit::Arc15
						| WallKit::HeaderArc90
						| WallKit::HeaderArc15 => Box::new(posed_partition_mesh_lod(
							kit_mesh_set(piece.geom),
							transform,
							tier,
						)) as Box<dyn Scene>,
						other => Box::new(with_pose(transform, wall_kit_scene(other, lod_ref)))
							as Box<dyn Scene>,
					},
				}
			})
			.collect();
		scene_children(children)
	}
}

/// Shared leaf LodScene for kits with a [`PartitionMeshSet`].
macro_rules! impl_partition_mesh_lod_scene {
	($ty:ty, $meshes:expr) => {
		impl ::lod::gen::LodScene for $ty {
			fn scene_lod_status(
				&self,
				lod_ref: &::lod::lod_ref::LodRef,
			) -> ::lod::gen::LodSceneStatus {
				$crate::partitions::lod::leaf_partition_lod_status(lod_ref)
			}

			fn scene_with_lod(
				&self,
				lod_ref: &::lod::lod_ref::LodRef,
			) -> impl ::bevy::scene::Scene + 'static {
				$crate::partitions::lod::leaf_partition_mesh_lod($meshes, lod_ref)
			}
		}
	};
}

pub(crate) use impl_partition_mesh_lod_scene;
