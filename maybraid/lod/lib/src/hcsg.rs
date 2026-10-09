//! Shared HCSG runtime ([#1005](https://github.com/ramate-io/maybraid/issues/1005),
//! [#1007](https://github.com/ramate-io/maybraid/issues/1007)). See [`shared`] and
//! `hcsg/README.md`.

pub mod shared;

pub use shared::node_store::{NodeStore, StoredEntry, DEFAULT_BASE_SCALE};
pub use shared::{
	bounds::{Gated, HcsgBounds, HcsgBoundsPlugin, HcsgClass, HcsgGate, HcsgRegions},
	context::{GenerationContext, GenerationScheme},
	demand::{HcsgDemand, Published, SubscriptionId},
	generation::GenerationPlugin,
	node::HcsgNode,
	presentation::{PresentationPlugin, RetiredHost},
	runtime::HcsgSystems,
	session::{
		request_hcsg_session_restart, HcsgRestartRequest, HcsgSessionPlugin, HcsgSessionSeed,
	},
	storage::{Busy, HcsgStorage, HcsgValue},
	worker::HcsgWorker,
};

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;

/// Bounds for universal entries: everything a region query could ask about.
pub fn universal_bounds() -> Aabb3d {
	Aabb3d::from_min_max(Vec3::splat(-1.0e9), Vec3::splat(1.0e9))
}

/// Declares `T` a seeded session root at `Id::Universal`. It is never built,
/// only [`HcsgStorage::seed`].
#[macro_export]
macro_rules! seeded_root {
	($T:ty) => {
		impl $crate::hcsg::shared::GenerationScheme for $T {
			fn original_ids_for(
				_cx: &mut $crate::hcsg::shared::GenerationContext,
				_region: bevy::math::bounding::Aabb3d,
			) -> Vec<$crate::gen::OriginalId> {
				vec![$crate::gen::OriginalId::universal()]
			}

			fn build_with_id(
				_cx: &mut $crate::hcsg::shared::GenerationContext,
				_id: $crate::gen::Id,
			) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
				None
			}
		}
	};
}
