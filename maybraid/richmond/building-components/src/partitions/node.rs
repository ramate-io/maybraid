//! Partition IR node: style + geometry + placement.

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::{LodScene, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;

use crate::assets::partitions::rough_stonework::{
	ARC_15_HIGH, ARC_180_HIGH, ARC_180_LOW, ARC_180_MID, ARC_90_HIGH, ARC_90_LOW, ARC_90_MID,
	ARC_15_LOW, ARC_15_MID, HEADER_15_HIGH, HEADER_15_LOW, HEADER_15_MID, HEADER_90_HIGH,
	HEADER_90_LOW, HEADER_90_MID, JOINT_HIGH, JOINT_MID, LINEAR, LINEAR_HIGH, LINEAR_LOW,
	LINEAR_MID,
};
use crate::parent_confines::{confined_scene, ParentConfines};
use crate::partitions::geometry::PartitionGeometry;
use crate::partitions::lod::{
	lod_level_for_placement, lod_status_for_placement, posed_joint_mesh_lod, posed_joint_mesh_tier,
	posed_partition_mesh_lod, posed_partition_mesh_tier, PartitionLodProbe, PartitionMeshSet,
};
use crate::partitions::rough_stonework::{
	RoughStonework15, RoughStonework180, RoughStonework90, RoughStoneworkHeader15,
	RoughStoneworkHeader180, RoughStoneworkHeader90, RoughStoneworkJoint, RoughStoneworkLinear,
	RoughStoneworkLinearHeaderSubsegment, RoughStoneworkLinearSubsegment,
};
use crate::partitions::style::PartitionStyle;
use crate::partitions::tessellate::PartitionKit;
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

	fn kit_scenes_for_level(
		&self,
		lod_ref: &LodRef,
		level: LodSceneLevel,
	) -> Vec<Box<dyn Scene>> {
		self.geometry
			.placed_kits(self.placement)
			.into_iter()
			.map(|piece| {
				let transform = pose(piece.placement);
				match self.style {
					PartitionStyle::RoughStonework => match piece.geom {
						PartitionKit::Linear
						| PartitionKit::Arc180
						| PartitionKit::Arc90
						| PartitionKit::Arc15
						| PartitionKit::HeaderArc90
						| PartitionKit::HeaderArc15 => Box::new(posed_partition_mesh_tier(
							kit_mesh_set(piece.geom),
							transform,
							level,
						)) as Box<dyn Scene>,
						PartitionKit::Joint => Box::new(posed_joint_mesh_tier(
							JOINT_HIGH,
							JOINT_MID,
							transform,
							level,
						)) as Box<dyn Scene>,
						other => Box::new(with_pose(transform, partition_kit_scene(other, lod_ref)))
							as Box<dyn Scene>,
					},
				}
			})
			.collect()
	}
}

pub(crate) fn kit_mesh_set(kit: PartitionKit) -> PartitionMeshSet {
	match kit {
		PartitionKit::Linear => PartitionMeshSet::new(LINEAR_HIGH, LINEAR_MID, LINEAR_LOW),
		PartitionKit::Arc180 => PartitionMeshSet::new(ARC_180_HIGH, ARC_180_MID, ARC_180_LOW),
		PartitionKit::Arc90 => PartitionMeshSet::new(ARC_90_HIGH, ARC_90_MID, ARC_90_LOW),
		PartitionKit::Arc15 => PartitionMeshSet::new(ARC_15_HIGH, ARC_15_MID, ARC_15_LOW),
		PartitionKit::HeaderArc90 => {
			PartitionMeshSet::new(HEADER_90_HIGH, HEADER_90_MID, HEADER_90_LOW)
		}
		PartitionKit::HeaderArc15 => {
			PartitionMeshSet::new(HEADER_15_HIGH, HEADER_15_MID, HEADER_15_LOW)
		}
		PartitionKit::LinearSubsegment
		| PartitionKit::LinearHeaderSubsegment
		| PartitionKit::HeaderArc180
		| PartitionKit::Joint => PartitionMeshSet::uniform(LINEAR),
	}
}

pub(crate) fn partition_kit_scene(kit: PartitionKit, lod_ref: &LodRef) -> Box<dyn Scene> {
	match kit {
		PartitionKit::Linear => Box::new(RoughStoneworkLinear.scene_with_lod(lod_ref)),
		PartitionKit::LinearSubsegment => {
			Box::new(RoughStoneworkLinearSubsegment.scene_with_lod(lod_ref))
		}
		PartitionKit::LinearHeaderSubsegment => {
			Box::new(RoughStoneworkLinearHeaderSubsegment.scene_with_lod(lod_ref))
		}
		PartitionKit::Arc180 => Box::new(RoughStonework180.scene_with_lod(lod_ref)),
		PartitionKit::Arc90 => Box::new(RoughStonework90.scene_with_lod(lod_ref)),
		PartitionKit::Arc15 => Box::new(RoughStonework15.scene_with_lod(lod_ref)),
		PartitionKit::HeaderArc180 => Box::new(RoughStoneworkHeader180.scene_with_lod(lod_ref)),
		PartitionKit::HeaderArc90 => Box::new(RoughStoneworkHeader90.scene_with_lod(lod_ref)),
		PartitionKit::HeaderArc15 => Box::new(RoughStoneworkHeader15.scene_with_lod(lod_ref)),
		PartitionKit::Joint => Box::new(RoughStoneworkJoint.scene_with_lod(lod_ref)),
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
		let level = self.scene_lod_level(lod_ref);
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.placed_kits(self.placement)
			.into_iter()
			.map(|piece| {
				let transform = pose(piece.placement);
				match self.style {
					PartitionStyle::RoughStonework => match piece.geom {
						PartitionKit::Linear
						| PartitionKit::Arc180
						| PartitionKit::Arc90
						| PartitionKit::Arc15
						| PartitionKit::HeaderArc90
						| PartitionKit::HeaderArc15 => Box::new(posed_partition_mesh_lod(
							kit_mesh_set(piece.geom),
							transform,
							level,
							PartitionLodProbe::from_placement(&piece.placement),
						)) as Box<dyn Scene>,
						PartitionKit::Joint => Box::new(posed_joint_mesh_lod(
							JOINT_HIGH,
							JOINT_MID,
							transform,
							level,
							PartitionLodProbe::from_placement(&piece.placement),
						)) as Box<dyn Scene>,
						other => Box::new(with_pose(transform, partition_kit_scene(other, lod_ref)))
							as Box<dyn Scene>,
					},
				}
			})
			.collect();
		confined_scene(self.confines, scene_children(children))
	}
}

macro_rules! impl_partition_mesh_lod_scene {
	($ty:ty, $meshes:expr) => {
		impl ::lod::gen::LodScene for $ty {
			fn scene_lod_level(
				&self,
				lod_ref: &::lod::lod_ref::LodRef,
			) -> ::lod::gen::LodSceneLevel {
				$crate::partitions::lod::leaf_partition_lod_level(lod_ref)
			}

			fn scene_lod_status(
				&self,
				lod_ref: &::lod::lod_ref::LodRef,
			) -> ::lod::gen::LodSceneStatus {
				$crate::partitions::lod::leaf_partition_lod_status(lod_ref)
			}

			fn scene_with_level(
				&self,
				_lod_ref: &::lod::lod_ref::LodRef,
				level: ::lod::gen::LodSceneLevel,
			) -> impl ::bevy::scene::Scene + 'static {
				$crate::partitions::lod::posed_partition_mesh_tier(
					$meshes,
					::bevy::prelude::Transform::IDENTITY,
					level,
				)
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
