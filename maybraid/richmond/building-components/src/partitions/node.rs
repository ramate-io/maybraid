//! Partition IR node: style + geometry + placement.
//!
//! Covers both **direct** component mappings (e.g. a single linear / arc kit) and
//! **tessellated** concepts (polyline / continuous arc → many tiles under **one** LOD
//! parent host). Leaf style types still expose per-mesh hosts for playground previews.
//!
//! [`PartitionNode::scene_with_lod`] builds a **lazy** [`lod::LodSceneHost`]: one level
//! root at spawn; fine-pass update / eager fulfill / cull bring other bands as the
//! viewer moves (see playground `add_fine_pass_for::<PartitionNode>`).

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Children, Component, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::Vec3;
use lod::fine_pass::LodHostBounds;
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::lod_scene_host::{LodLevelRoot, LodLevelRoots, LodSceneHost};

use crate::assets::panels::rough_stonework::{
	RIGHT_TRIANGLE_HIGH, RIGHT_TRIANGLE_LOW, RIGHT_TRIANGLE_MID,
};
use crate::lod_band::{characteristic_extent_abs, placement_center, warm_mesh_lod_culls};
use crate::parent_confines::{confined_scene, ParentConfines};
use crate::partitions::geometry::{JointLod, LinearLod, PartitionGeometry, PartitionTile};
use crate::partitions::rough_stonework::{
	RoughStoneworkJoint, RoughStoneworkLinearSliceSubsegment, RoughStoneworkLinearSubsegment,
	RoughStoneworkSlice180,
};
use crate::partitions::style::PartitionStyle;
use crate::placed::Placement;
use crate::scene_children::{pose, scene_children, with_pose};

/// Authoring IR for a partition feature (primitive — no portals).
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct PartitionNode {
	pub style: PartitionStyle,
	pub geometry: PartitionGeometry,
	pub placement: Placement,
	/// External silhouette vs internal detail gating.
	pub confines: ParentConfines,
}

impl PartitionNode {
	pub fn new(style: PartitionStyle, geometry: PartitionGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement, confines: ParentConfines::External }
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

	/// AABB for [`LodHostBounds`] from placement center / characteristic extent.
	pub fn host_bounds(&self) -> Aabb3d {
		let center = placement_center(&self.placement);
		let half = Vec3::splat(characteristic_extent_abs(&self.placement) * 0.5);
		Aabb3d::from_min_max(center - half, center + half)
	}

	fn kit_scenes_for_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> Vec<Box<dyn Scene>> {
		let _ = lod_ref;
		self.geometry
			.placed_tiles_for_style(self.style, self.placement)
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
						PartitionTile::RightTriangle { mirror } => {
							Some(Box::new(posed_right_triangle_tier(transform, level, mirror))
								as Box<dyn Scene>)
						}
						tile => {
							if let Some(meshes) = tile.mesh_set() {
								Some(Box::new(LinearLod::posed_tier(meshes, transform, level))
									as Box<dyn Scene>)
							} else {
								None
							}
						}
					},
				}
			})
			.collect()
	}
}

fn posed_right_triangle_tier(
	transform: Transform,
	level: LodSceneLevel,
	mirror: Option<scene_ref::MirrorAxis>,
) -> impl Scene + 'static {
	let path = match level {
		LodSceneLevel::High => RIGHT_TRIANGLE_HIGH,
		LodSceneLevel::Medium => RIGHT_TRIANGLE_MID,
		LodSceneLevel::Low
		| LodSceneLevel::UltraLow
		| LodSceneLevel::Distance(_)
		| LodSceneLevel::Resolution(_) => RIGHT_TRIANGLE_LOW,
	};
	with_pose(transform, path.scene_ref().with_mirror(mirror).scene())
}

/// Lazy host: [`LodSceneHost`] + [`PartitionNode`] + one active level root.
fn lazy_partition_host(
	node: PartitionNode,
	level: LodSceneLevel,
	bounds: Aabb3d,
	content: impl Scene + 'static,
) -> impl Scene + 'static {
	let content_children: Vec<Box<dyn Scene>> = vec![Box::new(content)];
	let level_root: Box<dyn Scene> = Box::new(bsn! {
		template_value(LodLevelRoot(level))
		Transform::default()
		Visibility::Inherited
		Children [ {content_children} ]
	});
	let level_roots_children: Vec<Box<dyn Scene>> = vec![level_root];
	let roots: Box<dyn Scene> = Box::new(bsn! {
		LodLevelRoots
		Transform::default()
		Visibility::Inherited
		Children [ {level_roots_children} ]
	});
	let host_children: Vec<Box<dyn Scene>> = vec![roots];
	let host_bounds = LodHostBounds(bounds);
	bsn! {
		LodSceneHost
		template_value(node)
		template_value(level)
		template_value(host_bounds)
		Transform::default()
		Visibility::Inherited
		Children [ {host_children} ]
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
		_ => Box::new(RoughStoneworkLinearSubsegment.scene_with_lod(lod_ref))
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

	fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		let _ = lod_ref;
		warm_mesh_lod_culls(current)
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		confined_scene(self.confines, scene_children(self.kit_scenes_for_level(lod_ref, level)))
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let level = self.scene_lod_level(lod_ref);
		lazy_partition_host(
			self.clone(),
			level,
			self.host_bounds(),
			self.scene_with_level(lod_ref, level),
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

			fn scene_lod_culls(
				&self,
				_lod_ref: &::lod::lod_ref::LodRef,
				current: ::lod::gen::LodSceneLevel,
			) -> ::lod::gen::LodSceneCulls {
				$crate::lod_band::warm_mesh_lod_culls(current)
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
