//! Reusable Richmond building scene components.
//!
//! Per domain: [`style`](floors::FloorStyle) + geometry + [`Placement`] → node (`LodScene`).

pub mod arc_kit;
pub mod assets;
pub mod doors;
pub mod floors;
pub mod partitions;
pub mod placed;
pub mod roofs;
pub mod scene_children;
pub mod stairs;

pub use arc_kit::{decompose_arc_sweep, ArcKit};
pub use assets::AssetPath;
pub use placed::{Placed, Placement};
pub use scene_children::{pose, posed_glb, scene_children, with_pose};

use bevy::scene::{ResolveContext, ResolvedScene};

pub(crate) fn empty_scene(_: &mut ResolveContext, _: &mut ResolvedScene) {}

/// Shared empty `LodScene` body for component placeholders.
macro_rules! impl_empty_lod_scene {
	($($ty:ty),+ $(,)?) => {
		$(
			impl ::lod::gen::LodScene for $ty {
				fn scene_with_lod(
					&self,
					_lod_ref: &::lod::lod_ref::LodRef,
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
			fn scene_with_lod(
				&self,
				_lod_ref: &::lod::lod_ref::LodRef,
			) -> impl ::bevy::scene::Scene + 'static {
				($asset).mesh_ref().scene()
			}
		}
	};
}

pub(crate) use impl_glb_lod_scene;
