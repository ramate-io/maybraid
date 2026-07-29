//! Partition IR node: style + geometry + placement.
//!
//! Composite geometries (polyline / arc) expand to kits under **one** LOD host so a short
//! polyline flips as a single parent. Leaf style types still expose per-mesh hosts for
//! playground previews.

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::{LodScene, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;

use crate::assets::partitions::rough_stonework::{
	ARC_15_HIGH, ARC_15_LOW, ARC_15_MID, ARC_180_HIGH, ARC_180_LOW, ARC_180_MID, ARC_90_HIGH,
	ARC_90_LOW, ARC_90_MID, HEADER_15_HIGH, HEADER_15_LOW, HEADER_15_MID, HEADER_90_HIGH,
	HEADER_90_LOW, HEADER_90_MID, LINEAR, LINEAR_HIGH, LINEAR_LOW, LINEAR_MID,
};
use crate::parent_confines::{confined_scene, ParentConfines};
use crate::partitions::geometry::{
	JointLod, LinearLod, PartitionGeometry, PartitionTile,
};
use crate::partitions::lod::{
	lod_level_for_placement, lod_status_for_placement, posed_partition_parent_lod,
	PartitionLodProbe, PartitionMeshSet,
};
use crate::partitions::rough_stonework::{
	RoughStonework15, RoughStonework180, RoughStonework90, RoughStoneworkHeader15,
	RoughStoneworkHeader180, RoughStoneworkHeader90, RoughStoneworkJoint, RoughStoneworkLinear,
	RoughStoneworkLinearHeaderSubsegment, RoughStoneworkLinearSubsegment,
};
use crate::partitions::style::PartitionStyle;
use crate::placed::Placement;
use crate::scene_children::{pose, scene_children, with_pose};

/// Authoring IR for a partition feature (primitive — no portals).
#[derive(Debug, Clone, PartialEq)]
pub struct PartitionNode {
	pub style: PartitionStyle,
	pub geometry: PartitionGeometry,
	pub placement: Placement,
	/// External silhouette vs internal detail gating.
	pub confines: ParentConfines,
}

impl PartitionNode {
	pub fn new(style: PartitionStyle, geometry: PartitionGeometry, placement: Placement) -> Self {
		Self {
			style,
			geometry,
			placement,
			confines: ParentConfines::External,
		}
	}

	pub fn rough_stone(geometry: PartitionGeometry, placement: Placement) -> Self {
		Self::new(PartitionStyle::RoughStonework, geometry, placement)
	}

	pub fn with_confines(mut self, confines: ParentConfines) -> Self {
		self.confines = confines;
		self
	}

	/// Status as if a rough-stone linear partition sat at `center` with `extent` scale.
	pub fn representative_lod_status(
		center: Vec3,
		extent: Vec3,
		lod_ref: &LodRef,
	) -> LodSceneStatus {
		let node = Self::rough_stone(
			PartitionGeometry::linear(),
			Placement::new(center, 0.0).with_scale(extent.max(Vec3::splat(1e-4))),
		);
		node.scene_lod_status(lod_ref)
	}

	fn kit_scenes_for_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> Vec<Box<dyn Scene>> {
		self.geometry
			.placed_tiles(self.placement)
			.into_iter()
			.filter_map(|piece| {
				let transform = pose(piece.placement);
				match self.style {
					PartitionStyle::RoughStonework => match piece.geom {
						PartitionTile::Joint => {
							if !JointLod::included_at(level) {
								return None;
							}
							Some(Box::new(JointLod::posed_tier(transform, level)) as Box<dyn Scene>)
						}
						PartitionTile::Linear
						| PartitionTile::Arc180
						| PartitionTile::Arc90
						| PartitionTile::Arc15
						| PartitionTile::HeaderArc90
						| PartitionTile::HeaderArc15 => Some(Box::new(LinearLod::posed_tier(
							tile_mesh_set(piece.geom),
							transform,
							level,
						))
							as Box<dyn Scene>),
						other => Some(Box::new(with_pose(
							transform,
							partition_tile_scene(other, lod_ref),
						)) as Box<dyn Scene>),
					},
				}
			})
			.collect()
	}
}

pub(crate) fn tile_mesh_set(tile: PartitionTile) -> PartitionMeshSet {
	match tile {
		PartitionTile::Linear => PartitionMeshSet::new(LINEAR_HIGH, LINEAR_MID, LINEAR_LOW),
		PartitionTile::Arc180 => PartitionMeshSet::new(ARC_180_HIGH, ARC_180_MID, ARC_180_LOW),
		PartitionTile::Arc90 => PartitionMeshSet::new(ARC_90_HIGH, ARC_90_MID, ARC_90_LOW),
		PartitionTile::Arc15 => PartitionMeshSet::new(ARC_15_HIGH, ARC_15_MID, ARC_15_LOW),
		PartitionTile::HeaderArc90 => {
			PartitionMeshSet::new(HEADER_90_HIGH, HEADER_90_MID, HEADER_90_LOW)
		}
		PartitionTile::HeaderArc15 => {
			PartitionMeshSet::new(HEADER_15_HIGH, HEADER_15_MID, HEADER_15_LOW)
		}
		PartitionTile::LinearSubsegment
		| PartitionTile::LinearHeaderSubsegment
		| PartitionTile::HeaderArc180
		| PartitionTile::Joint => PartitionMeshSet::uniform(LINEAR),
	}
}

pub(crate) fn partition_tile_scene(tile: PartitionTile, lod_ref: &LodRef) -> Box<dyn Scene> {
	match tile {
		PartitionTile::Linear => Box::new(RoughStoneworkLinear.scene_with_lod(lod_ref)),
		PartitionTile::LinearSubsegment => {
			Box::new(RoughStoneworkLinearSubsegment.scene_with_lod(lod_ref))
		}
		PartitionTile::LinearHeaderSubsegment => {
			Box::new(RoughStoneworkLinearHeaderSubsegment.scene_with_lod(lod_ref))
		}
		PartitionTile::Arc180 => Box::new(RoughStonework180.scene_with_lod(lod_ref)),
		PartitionTile::Arc90 => Box::new(RoughStonework90.scene_with_lod(lod_ref)),
		PartitionTile::Arc15 => Box::new(RoughStonework15.scene_with_lod(lod_ref)),
		PartitionTile::HeaderArc180 => Box::new(RoughStoneworkHeader180.scene_with_lod(lod_ref)),
		PartitionTile::HeaderArc90 => Box::new(RoughStoneworkHeader90.scene_with_lod(lod_ref)),
		PartitionTile::HeaderArc15 => Box::new(RoughStoneworkHeader15.scene_with_lod(lod_ref)),
		PartitionTile::Joint => Box::new(RoughStoneworkJoint.scene_with_lod(lod_ref)),
	}
}

impl LodScene for PartitionNode {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		lod_level_for_placement(&self.placement, lod_ref)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		lod_status_for_placement(&self.placement, lod_ref)
	}

	fn scene_with_level(
		&self,
		lod_ref: &LodRef,
		level: LodSceneLevel,
	) -> impl Scene + 'static {
		confined_scene(
			self.confines,
			scene_children(self.kit_scenes_for_level(lod_ref, level)),
		)
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		// One host for the whole node (polyline / arc / linear) — kits are content, not nested hosts.
		let level = self.scene_lod_level(lod_ref);
		let probe = PartitionLodProbe::from_placement(&self.placement);
		let high = self.scene_with_level(lod_ref, LodSceneLevel::High);
		let mid = self.scene_with_level(lod_ref, LodSceneLevel::Medium);
		let low = self.scene_with_level(lod_ref, LodSceneLevel::Low);
		posed_partition_parent_lod(level, probe, high, mid, low)
	}
}

macro_rules! impl_partition_mesh_lod_scene {
	($ty:ty, $meshes:expr) => {
		impl ::lod::gen::LodScene for $ty {
			fn scene_lod_level(
				&self,
				lod_ref: &::lod::lod_ref::LodRef,
			) -> ::lod::gen::LodSceneLevel {
				$crate::partitions::probe::leaf_partition_lod_level(lod_ref)
			}

			fn scene_lod_status(
				&self,
				lod_ref: &::lod::lod_ref::LodRef,
			) -> ::lod::gen::LodSceneStatus {
				$crate::partitions::probe::leaf_partition_lod_status(lod_ref)
			}

			fn scene_with_level(
				&self,
				_lod_ref: &::lod::lod_ref::LodRef,
				level: ::lod::gen::LodSceneLevel,
			) -> impl ::bevy::scene::Scene + 'static {
				$crate::partitions::geometry::LinearLod::posed_tier(
					$meshes,
					::bevy::prelude::Transform::IDENTITY,
					level,
				)
			}

			fn scene_with_lod(
				&self,
				lod_ref: &::lod::lod_ref::LodRef,
			) -> impl ::bevy::scene::Scene + 'static {
				$crate::partitions::geometry::LinearLod::leaf_host($meshes, lod_ref)
			}
		}
	};
}

pub(crate) use impl_partition_mesh_lod_scene;
