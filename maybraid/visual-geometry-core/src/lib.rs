//! Shared visual geometry plugins: SceneRef, identified-mesh caches, terrain chunk refs.
//!
//! This crate does not own Durham height, Chico shaders, or character. Hosts that
//! CpuShot terrain (or other [`IdentifiedMesh`] builders) call
//! [`install_enforced_mesh_cache`] and optionally [`share_terrain_chunk_refs`].

use bevy::prelude::*;
use lod_lazy_refs::LodLazyRefsPlugin;
use render_item::mesh::handle::{EnforceCachingPlugin, EnforcedCaches};
use render_item::mesh::{IdentifiedMesh, MeshBuilder};
use scene_ref::SceneRefPlugin;
use terrain_chunk_ref::{TerrainChunkRefCache, TerrainChunkRefPlugin};

/// Ensures [`SceneRefPlugin`] and [`LodLazyRefsPlugin`] are up.
pub struct VisualGeometryCorePlugin;

impl Plugin for VisualGeometryCorePlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SceneRefPlugin>() {
			app.add_plugins(SceneRefPlugin);
		}
		if !app.is_plugin_added::<LodLazyRefsPlugin>() {
			app.add_plugins(LodLazyRefsPlugin);
		}
	}
}

/// Process + optional disk [`Handle<Mesh>`] cache for one CpuShot (or other) builder.
pub fn install_enforced_mesh_cache<T, M>(app: &mut App)
where
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
	M: Material,
{
	if !app.is_plugin_added::<EnforceCachingPlugin<T, M>>() {
		app.add_plugins(EnforceCachingPlugin::<T, M>::default());
	}
}

/// Share fill's [`EnforcedCaches`] mailbox with overlay [`TerrainChunkRef`]s.
///
/// `build_on_miss = false` is the overlay path: wait for fill to publish a handle.
pub fn share_terrain_chunk_refs<T>(app: &mut App, build_on_miss: bool)
where
	T: MeshBuilder + IdentifiedMesh + Clone + Send + Sync + 'static,
{
	if !app.world().contains_resource::<TerrainChunkRefCache<T>>() {
		let caches = app.world().resource::<EnforcedCaches<T>>();
		let (handles, disk) = (caches.handle_map(), caches.disk_cache());
		let mut cache =
			TerrainChunkRefCache::<T>::new().with_handles(handles).with_optional_disk(disk);
		if !build_on_miss {
			cache = cache.without_build_on_miss();
		}
		app.insert_resource(cache);
	}
	if !app.is_plugin_added::<TerrainChunkRefPlugin<T>>() {
		app.add_plugins(TerrainChunkRefPlugin::<T>::default());
	}
}
