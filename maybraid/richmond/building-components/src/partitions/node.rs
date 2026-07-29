//! Partition IR node: style + geometry + placement.
//!
//! Covers both **direct** component mappings (e.g. a single linear / arc kit) and
//! **tessellated** concepts (polyline / continuous arc → many tiles under **one** LOD
//! parent host). Leaf style types still expose per-mesh hosts for playground previews.

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::{LodScene, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;

use crate::lod_host::warm_content_host_hsl;
use crate::parent_confines::{confined_scene, ParentConfines};
use crate::partitions::geometry::{JointLod, LinearLod, PartitionGeometry, PartitionTile};
use crate::partitions::probe::PartitionLodProbe;
use crate::partitions::rough_stonework::{
	RoughStoneworkSlice180, RoughStoneworkJoint, RoughStoneworkLinearSliceSubsegment,
	RoughStoneworkLinearSubsegment,
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
						tile => {
							if let Some(meshes) = tile.mesh_set() {
								Some(Box::new(LinearLod::posed_tier(meshes, transform, level))
									as Box<dyn Scene>)
							} else {
								Some(Box::new(with_pose(
									transform,
									placeholder_tile_scene(tile, lod_ref),
								)) as Box<dyn Scene>)
							}
						}
					},
				}
			})
			.collect()
	}
}

fn placeholder_tile_scene(tile: PartitionTile, lod_ref: &LodRef) -> Box<dyn Scene> {
	match tile {
		PartitionTile::LinearSubsegment => {
			Box::new(RoughStoneworkLinearSubsegment.scene_with_lod(lod_ref))
		}
		PartitionTile::LinearSliceSubsegment => {
			Box::new(RoughStoneworkLinearSliceSubsegment.scene_with_lod(lod_ref))
		}
		PartitionTile::SliceArc180 => Box::new(RoughStoneworkSlice180.scene_with_lod(lod_ref)),
		PartitionTile::Joint => Box::new(RoughStoneworkJoint.scene_with_lod(lod_ref)),
		_ => Box::new(RoughStoneworkLinearSubsegment.scene_with_lod(lod_ref)),
	}
}

/// Door-frame / empty leaf tiles that lack a mesh set.
pub(crate) fn partition_tile_scene(tile: PartitionTile, lod_ref: &LodRef) -> Box<dyn Scene> {
	match tile {
		PartitionTile::LinearSubsegment
		| PartitionTile::LinearSliceSubsegment
		| PartitionTile::SliceArc180
		| PartitionTile::Joint => placeholder_tile_scene(tile, lod_ref),
		other => {
			// Asset tiles: leaf style types for doors that still route here.
			if let Some(meshes) = other.mesh_set() {
				Box::new(LinearLod::leaf_host(meshes, lod_ref))
			} else {
				placeholder_tile_scene(PartitionTile::LinearSubsegment, lod_ref)
			}
		}
	}
}

impl LodScene for PartitionNode {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.placement.partition_lod_level(lod_ref)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.placement.partition_lod_status(lod_ref)
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
		let probe = PartitionLodProbe::from_placement(&self.placement);
		warm_content_host_hsl(
			level,
			probe,
			self.scene_with_level(lod_ref, LodSceneLevel::High),
			self.scene_with_level(lod_ref, LodSceneLevel::Medium),
			self.scene_with_level(lod_ref, LodSceneLevel::Low),
		)
	}
}

macro_rules! impl_partition_mesh_lod_scene {
	($ty:ty, $meshes:expr) => {
		impl ::lod::gen::LodScene for $ty {
			fn scene_lod_level(
				&self,
				lod_ref: &::lod::lod_ref::LodRef,
			) -> ::lod::gen::LodSceneLevel {
				$crate::partitions::probe::PartitionLodProbe::from_aabb(lod_ref.bounds)
					.level_for(lod_ref.current_transform)
			}

			fn scene_lod_status(
				&self,
				lod_ref: &::lod::lod_ref::LodRef,
			) -> ::lod::gen::LodSceneStatus {
				$crate::partitions::probe::PartitionLodProbe::from_aabb(lod_ref.bounds)
					.status_for_lod_ref(lod_ref)
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
