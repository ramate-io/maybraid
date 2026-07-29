//! Reusable Richmond building scene components.
//!
//! Per domain: [`style`](floors::FloorStyle) + geometry + [`Placement`] → node (`LodScene`).

pub mod arc_kit;
pub mod assets;
pub mod doors;
pub mod floors;
pub mod furniture;
pub mod lod_host;
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
pub use lod_host::{posed_asset_tier, warm_content_host, warm_content_host_hsl, warm_mesh_level_host};
pub use parent_confines::{
	apply_parent_confines, confined_scene, distance_to_segment, InternalShape, ParentConfines,
	INTERNAL_REVEAL_FACTOR,
};
pub use partitions::{
	update_partition_host_levels, Partition, PartitionGeometry, PartitionLodBand,
	PartitionLodProbe, PartitionMeshSet, PartitionMeshTier, PartitionNode, PartitionStyle,
	HEADER_KIT_HEIGHT, LINEAR_HIGH_FACTOR, LINEAR_LOW_FACTOR, LINEAR_MEDIUM_FACTOR,
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

/// `LodScene` that loads a GLB scene root via [`scene_ref::SceneRef`].
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
				($asset).scene_ref().scene()
			}
		}
	};
}

pub(crate) use impl_glb_lod_scene;
