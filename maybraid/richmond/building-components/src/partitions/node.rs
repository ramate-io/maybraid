//! Partition IR node: style + geometry + placement — fine-phase [`LodScene`] host.
//!
//! Covers both **direct** component mappings (e.g. a single linear / arc kit) and
//! **tessellated** concepts (polyline / continuous arc → many tiles under **one** LOD
//! parent host).

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Component, Transform};
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::Vec3;
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::{LodLazyPending, SceneChunk};
use material_ref::{MaterialRef, MaterialRefRoot, PropagateToDescendants};

use crate::layer::Layers;
use crate::lod_band::{placement_bounds, warm_mesh_lod_culls};
use crate::parent_confines::{confined_scene, ParentConfines};
use crate::partitions::geometry::{JointLod, LinearLod, PartitionGeometry, PartitionTile};
use crate::partitions::style::PartitionStyle;
use crate::placed::Placement;
use crate::scene_children::{pose, scene_children};

/// Authoring IR for a partition feature (primitive — no portals).
///
/// [`Self::style`] picks the kit GLB path. [`Self::material`] is an optional
/// shader look ([`MaterialRef`]) stamped onto that kit after spawn.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct PartitionNode {
	pub style: PartitionStyle,
	pub geometry: PartitionGeometry,
	pub placement: Placement,
	/// External silhouette vs internal detail gating.
	pub confines: ParentConfines,
	pub material: Option<MaterialRef>,
}

impl PartitionNode {
	pub fn new(style: PartitionStyle, geometry: PartitionGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement, confines: ParentConfines::External, material: None }
	}

	pub fn rough_stone(geometry: PartitionGeometry, placement: Placement) -> Self {
		Self::new(PartitionStyle::RoughStonework, geometry, placement)
	}

	pub fn with_confines(mut self, confines: ParentConfines) -> Self {
		self.confines = confines;
		self
	}

	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = Some(material);
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

	fn kit_scenes_for_level(&self, level: LodSceneLevel) -> Vec<Box<dyn Scene>> {
		self.geometry
			.placed_tiles_for_style(self.style, self.placement)
			.into_iter()
			.filter_map(|piece| {
				let transform = pose(piece.placement);
				let scene = match self.style {
					PartitionStyle::RoughStonework => match piece.geom {
						PartitionTile::Joint => {
							if !JointLod::included_at(level) {
								return None;
							}
							Some(Box::new(JointLod::posed_tier(transform, level)) as Box<dyn Scene>)
						}
						PartitionTile::RightTriangle { mirror } => {
							use crate::assets::panels::rough_stonework::{
								RIGHT_TRIANGLE_HIGH, RIGHT_TRIANGLE_LOW, RIGHT_TRIANGLE_MID,
							};
							use crate::partitions::mesh_set::PartitionMeshSet;
							Some(Box::new(LinearLod::posed_mirrored_tier(
								PartitionMeshSet::new(
									RIGHT_TRIANGLE_HIGH,
									RIGHT_TRIANGLE_MID,
									RIGHT_TRIANGLE_LOW,
								),
								transform,
								level,
								mirror,
							)) as Box<dyn Scene>)
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
				}?;
				Some(material_kit_scene(scene, self.material.clone()))
			})
			.collect()
	}
}

impl Layers<PartitionNode> {
	/// Stamp a shader look onto every partition, leaving kit [`PartitionStyle`] unchanged.
	pub fn with_material(self, material: MaterialRef) -> Self {
		self.map(|node| node.with_material(material.clone()))
	}
}

fn material_kit_scene(scene: Box<dyn Scene>, material: Option<MaterialRef>) -> Box<dyn Scene> {
	match material {
		Some(material) => Box::new((
			bsn! {
				template_value(MaterialRefRoot(material))
				PropagateToDescendants
				LodLazyPending
			},
			scene,
		)),
		None => scene,
	}
}

/// Door-frame / empty leaf tiles that lack a mesh set — posed content for `level`.
pub(crate) fn partition_tile_scene(tile: PartitionTile, level: LodSceneLevel) -> Box<dyn Scene> {
	match tile {
		PartitionTile::Joint => Box::new(JointLod::posed_tier(Transform::IDENTITY, level)),
		other => {
			if let Some(meshes) = other.mesh_set() {
				Box::new(LinearLod::posed_tier(meshes, Transform::IDENTITY, level))
			} else {
				Box::new(JointLod::posed_tier(Transform::IDENTITY, LodSceneLevel::Low))
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

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		confined_scene(self.confines, scene_children(self.kit_scenes_for_level(level)))
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		placement_bounds(&self.placement)
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

			fn scene_chunks_with_level(
				&self,
				lod_ref: &::lod::lod_ref::LodRef,
				level: ::lod::gen::LodSceneLevel,
			) -> ::lod::SceneChunk {
				::lod::SceneChunk::primitive(self.scene_with_level(lod_ref, level))
			}

			fn scene_bounds(&self) -> ::bevy::math::bounding::Aabb3d {
				::bevy::math::bounding::Aabb3d::from_min_max(
					::bevy::math::Vec3::new(0.0, 0.0, 0.0),
					::bevy::math::Vec3::new(1.0, 1.0, 1.0),
				)
			}
		}
	};
}

pub(crate) use impl_partition_mesh_lod_scene;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::partitions::geometry::PartitionGeometry;
	use crate::placed::Placement;

	#[test]
	fn with_material_stamps_ref_without_changing_style() {
		let node = PartitionNode::rough_stone(PartitionGeometry::linear(), Placement::default())
			.with_material(MaterialRef::named("stucco"));
		assert_eq!(node.style, PartitionStyle::RoughStonework);
		assert_eq!(
			node.material.as_ref().map(|m| &m.name),
			Some(&material_ref::MaterialId::named("stucco"))
		);
	}
}
