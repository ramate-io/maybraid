//! Gimme-backed LOD scene-host index and refresh / cull plugins.
//!
//! [`gimme_core`] stores hosts as AABB cells. Physics layers stay in `lod-avian`.
//! Call sites register refresh with `gimme_host!` (`GimmeLodSceneRefreshPlugin` /
//! `GimmeLodSceneCullPlugin`).

mod host;
mod refresh;

pub use host::{
	GimmeLodHostIndex, GimmeLodHostMarshaller, GimmeLodHostPlugin, GimmeLodHostVolume,
	GimmeLodSceneHostIndex,
};
pub use refresh::{GimmeLodSceneCullPlugin, GimmeLodSceneRefreshPlugin};
