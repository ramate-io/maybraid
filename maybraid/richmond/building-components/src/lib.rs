//! Reusable Richmond building scene components.
//!
//! Placeholders implement empty [`lod::gen::LodScene`] scenes until geometry
//! authoring lands.

pub mod doors;
pub mod floors;
pub mod partitions;
pub mod roofs;
pub mod stairs;

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
