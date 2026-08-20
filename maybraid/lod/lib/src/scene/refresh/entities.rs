//! Untyped fold of level messages → write [`LodSceneLevel`] on any host.

mod refresh;
mod update;

pub use refresh::{refresh_lod_host_levels, LodSceneRefreshEntitiesPlugin};
pub use update::{dominant_lod_ref, update_lod_host_levels};
