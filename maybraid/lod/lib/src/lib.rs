//! Umbrella crate for generalized LOD in Maybraid ([RFC-154](https://github.com/ramate-io/maybraid/issues/157)).
//!
//! Re-exports [`lod_cascade`] as [`cascade`] and [`lod_cascade_system`] as [`cascade_system`] so dependents can pull in the stack from one dependency when convenient.

pub use lod_cascade as cascade;
pub use lod_cascade_system as cascade_system;
pub mod fine_pass;
pub mod gen;
pub mod lod_level;
pub mod lod_ref;
pub mod lod_scene_host;

pub use fine_pass::{
	add_fine_pass_for, fulfill_lod_level_spawn, track_lod_viewer, update_lod_host_levels,
	LodFinePassPlugin, LodFinePassSystems, LodHostBounds, LodViewer, LodViewerState,
};
pub use lod_level::{LodSceneLevel, QuantizedDistance};
pub use lod_scene_host::{
	lod_host_scene, sync_lod_level_roots, LodLevelRoot, LodLevelRoots, LodLevelSpawnRequest,
	LodSceneHost, LodSceneHostPlugin,
};
