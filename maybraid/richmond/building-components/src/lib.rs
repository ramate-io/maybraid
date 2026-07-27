//! Reusable Richmond building scene components.
//!
//! Per domain: `geometry` → `geometry_components` → named scene component modules.

pub mod assets;
pub mod doors;
pub mod floors;
pub mod partitions;
pub mod placed;
pub mod roofs;
pub mod scene_children;
pub mod stairs;

pub use assets::AssetPath;
pub use placed::{IntoGeometryComponents, Placed};
pub use scene_children::scene_children;

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

/// `LodScene` that loads a GLB scene root from an [`crate::assets::AssetPath`].
macro_rules! impl_glb_lod_scene {
	($ty:ty, $asset:expr) => {
		impl ::lod::gen::LodScene for $ty {
			fn scene_with_lod(
				&self,
				_lod_ref: &::lod::lod_ref::LodRef,
			) -> impl ::bevy::scene::Scene + 'static {
				use ::bevy::scene::prelude::bsn;
				use ::bevy::world_serialization::WorldAssetRoot;
				let path = $asset.gltf_scene_0();
				// Bare `WorldAssetRoot` — a leading `::` is parsed as BSN cache syntax.
				bsn! {
					WorldAssetRoot({path})
				}
			}
		}
	};
}

pub(crate) use impl_glb_lod_scene;
