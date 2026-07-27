//! Reusable Richmond building scene components.
//!
//! Per domain: `geometry` → `geometry_components` → named scene component modules.

pub mod doors;
pub mod floors;
pub mod partitions;
pub mod placed;
pub mod roofs;
pub mod stairs;

pub use placed::{IntoGeometryComponents, Placed};

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
