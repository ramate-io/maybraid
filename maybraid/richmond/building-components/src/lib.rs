//! Reusable Richmond building scene components.
//!
//! Per domain: [`style`](floors::FloorStyle) + geometry + [`Placement`] → node (`LodScene`).

pub mod arc_kit;
pub mod assets;
pub mod doors;
pub mod floors;
pub mod furniture;
pub mod parent_confines;
pub mod partitions;
pub mod placed;
pub mod roofs;
pub mod scene_children;
pub mod stairs;

pub use arc_kit::{decompose_arc_sweep, ArcKit};
pub use assets::AssetPath;
pub use furniture::{
	FurnitureGeometry, FurnitureNode, FurnitureStyle, FurnitureWireframePlugin,
};
pub use parent_confines::{
	apply_parent_confines, confined_scene, distance_to_segment, ParentConfines,
	INTERNAL_REVEAL_FACTOR,
};
pub use partitions::{
	update_partition_host_levels, Partition, PartitionGeometry, PartitionLodBand,
	PartitionLodProbe, PartitionMeshSet, PartitionMeshTier, PartitionNode, PartitionStyle,
	HEADER_KIT_HEIGHT, PARTITION_HIGH_FACTOR, PARTITION_LOW_FACTOR, PARTITION_MEDIUM_FACTOR,
};
pub use placed::{Placed, Placement};
pub use scene_children::{pose, posed_glb, scene_children, with_pose, wireframe_box_with_handles};

use bevy::scene::{ResolveContext, ResolvedScene};

pub(crate) fn empty_scene(_: &mut ResolveContext, _: &mut ResolvedScene) {}

/// Shared empty `LodScene` body for component placeholders.
macro_rules! impl_empty_lod_scene {
	($($ty:ty),+ $(,)?) => {
		$(
			impl ::lod::gen::LodScene for $ty {
				fn scene_lod_status(
					&self,
					_lod_ref: &::lod::lod_ref::LodRef,
				) -> ::lod::gen::LodSceneStatus {
					::lod::gen::LodSceneStatus::Unchanged
				}

				fn scene_with_level(
					&self,
					_lod_ref: &::lod::lod_ref::LodRef,
					_level: ::lod::gen::LodSceneLevel,
				) -> impl ::bevy::scene::Scene + 'static {
					::bevy::scene::SceneFunction($crate::empty_scene)
				}
			}
		)+
	};
}

pub(crate) use impl_empty_lod_scene;

/// `LodScene` that loads a GLB scene root via [`mesh_ref::MeshRef::Glb`].
macro_rules! impl_glb_lod_scene {
	($ty:ty, $asset:expr) => {
		impl ::lod::gen::LodScene for $ty {
			fn scene_lod_status(
				&self,
				_lod_ref: &::lod::lod_ref::LodRef,
			) -> ::lod::gen::LodSceneStatus {
				::lod::gen::LodSceneStatus::Unchanged
			}

			fn scene_with_level(
				&self,
				_lod_ref: &::lod::lod_ref::LodRef,
				_level: ::lod::gen::LodSceneLevel,
			) -> impl ::bevy::scene::Scene + 'static {
				($asset).mesh_ref().scene()
			}
		}
	};
}

pub(crate) use impl_glb_lod_scene;
