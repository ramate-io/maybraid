//! Durham on the shared HCSG runtime ([`lod::hcsg::shared`]): terrain and
//! water generate on the worker and present as [`HcsgNode`] hosts.
//!
//! This runs beside the frame-synchronous path in [`crate::terrain::host`]
//! until every Durham host has moved over. Start a session with
//! [`crate::DurhamRoots::restart`].

use std::marker::PhantomData;
use std::sync::Arc;

use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::hcsg::shared::{Busy, HcsgBounds, HcsgNode, HcsgStorage, PresentationPlugin};
use lod::scene::LodSceneRefreshChunkPlugin;

use crate::terrain::index::{origin_cell_ids_at, DurhamNodes};
use crate::terrain::{Terrain, TerrainCellLayout};
use crate::water::Water;

/// The [`TerrainCellLayout`] resource's window: its request region is
/// generated and presented, and hosts are kept within its presentation region.
pub struct DurhamWindow;

impl HcsgBounds for DurhamWindow {
	type Param = Res<'static, TerrainCellLayout>;

	fn inner(layout: &SystemParamItem<Self::Param>) -> Option<Aabb3d> {
		Some(layout.request_region())
	}

	fn outer(layout: &SystemParamItem<Self::Param>) -> Option<Aabb3d> {
		Some(layout.presentation_region())
	}

	fn focus(layout: &SystemParamItem<Self::Param>) -> Option<Vec3> {
		Some(layout.region_center_xz())
	}
}

/// Presents Durham terrain and water within `B` from the shared storage.
///
/// Scenes come from [`Terrain`] and [`Water`]'s own `LodScene` impls; cells
/// that seed collision carry the trimesh source.
pub struct DurhamPresentationPlugin<B>(PhantomData<fn() -> B>);

impl<B> Default for DurhamPresentationPlugin<B> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<B: HcsgBounds> Plugin for DurhamPresentationPlugin<B> {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<WaterPresentationPlugin<B>>() {
			app.add_plugins(WaterPresentationPlugin::<B>::default());
		}
		app.add_plugins(PresentationPlugin::<B, Terrain>::default());
		if !app.is_plugin_added::<LodSceneRefreshChunkPlugin<HcsgNode<Terrain>>>() {
			app.add_plugins(LodSceneRefreshChunkPlugin::<HcsgNode<Terrain>>::default());
		}
	}
}

/// Presents Durham water within `B` from the shared storage, for a layer
/// that presents its own surface over Durham terrain.
pub struct WaterPresentationPlugin<B>(PhantomData<fn() -> B>);

impl<B> Default for WaterPresentationPlugin<B> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<B: HcsgBounds> Plugin for WaterPresentationPlugin<B> {
	fn build(&self, app: &mut App) {
		let storage = app.world_mut().get_resource_or_init::<HcsgStorage>().clone();
		DurhamNodes::configure(&storage);
		app.add_plugins(PresentationPlugin::<B, Water>::default());
		if !app.is_plugin_added::<LodSceneRefreshChunkPlugin<HcsgNode<Water>>>() {
			app.add_plugins(LodSceneRefreshChunkPlugin::<HcsgNode<Water>>::default());
		}
	}
}

/// Frame-side terrain reads over the shared storage. [`Busy`] while the
/// worker holds a store; try again next frame.
pub trait SharedTerrainStorage {
	/// The published origin cell covering `(x, z)`: fine grid first, then
	/// outer and stream rings.
	fn try_terrain_at(
		&self,
		layout: &TerrainCellLayout,
		x: f32,
		z: f32,
	) -> Result<Option<Arc<Terrain>>, Busy>;

	/// Composed terrain height (stamps and watersheds) at `(x, z)`, once
	/// that cell is published.
	fn try_composed_height_at(
		&self,
		layout: &TerrainCellLayout,
		x: f32,
		z: f32,
	) -> Result<Option<f32>, Busy> {
		let terrain = self.try_terrain_at(layout, x, z)?;
		Ok(terrain.map(|terrain| terrain.sdf.terrain().height_at_with_all_modulations(x, z)))
	}
}

impl SharedTerrainStorage for HcsgStorage {
	fn try_terrain_at(
		&self,
		layout: &TerrainCellLayout,
		x: f32,
		z: f32,
	) -> Result<Option<Arc<Terrain>>, Busy> {
		for id in origin_cell_ids_at(layout, x, z) {
			if let Some(entry) = self.try_entry::<Terrain>(id)? {
				return Ok(Some(entry.value));
			}
		}
		Ok(None)
	}
}

#[cfg(test)]
mod equivalence;

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use bevy::scene::ScenePlugin;
	use lod::gen::{Id, OriginalId, Version};
	use lod::hcsg::shared::{HcsgDemand, HcsgSystems};
	use lod::lod_ref::LodNodePose;
	use lod::LodViewer;

	use super::*;
	use crate::terrain::{
		fine_patch_cell_layout, CellTiling, TerrainColliderMeshSource, TerrainConfig,
		TerrainMeshLodBand, TerrainPresentationAssets, TerrainStampConfigs, WatershedConfigs,
	};
	use crate::water::WaterPresentationAssets;
	use crate::DurhamRoots;

	const IDLE: Duration = Duration::from_secs(120);

	/// Restart the session from the resources on the next update.
	#[derive(Resource)]
	struct Restart(bool);

	fn restart(
		roots: DurhamRoots,
		storage: Res<HcsgStorage>,
		demand: Res<HcsgDemand>,
		mut pending: ResMut<Restart>,
	) {
		if std::mem::take(&mut pending.0) {
			roots.restart(&storage, &demand);
		}
	}

	fn seed_resources(app: &mut App, seed: u32) {
		app.insert_resource(TerrainStampConfigs::from_world_seed(seed))
			.insert_resource(WatershedConfigs::default().with_seed(seed))
			.insert_resource(TerrainPresentationAssets {
				config: TerrainConfig::new(seed),
				material: Handle::default(),
				lod_bands: vec![TerrainMeshLodBand { max_radius_cells: 1, res_2: 2 }],
				outer_add_walls: false,
				fine_grid_max_radius: Some(1),
				macro_seam_half_extents: Vec::new(),
				macro_cell_min_size: None,
				macro_res_2: None,
			})
			.insert_resource(Restart(true));
	}

	/// A 2×2 fine patch at the origin, presented from the shared runtime.
	fn app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_plugins((AssetPlugin::default(), ScenePlugin))
			.insert_resource(fine_patch_cell_layout(1, IVec2::new(-1, -1)))
			.insert_resource(WaterPresentationAssets { material: Handle::default() })
			.add_plugins(DurhamPresentationPlugin::<DurhamWindow>::default())
			.add_systems(Update, restart.before(HcsgSystems));
		seed_resources(&mut app, 1);
		let at = Transform::IDENTITY;
		app.world_mut()
			.spawn((LodViewer, at, LodNodePose { previous: at, current: at }));
		app
	}

	/// Restart and subscribe, let the worker finish, then spawn and fulfill hosts.
	fn settle(app: &mut App) -> anyhow::Result<()> {
		for _ in 0..2 {
			app.update();
			let demand = app.world().resource::<HcsgDemand>().clone();
			anyhow::ensure!(demand.wait_idle(IDLE), "worker did not go idle");
		}
		for _ in 0..4 {
			app.update();
		}
		Ok(())
	}

	fn hosts<T: lod::hcsg::shared::HcsgValue>(app: &mut App) -> Vec<(Id, Version)> {
		let mut hosts: Vec<_> = app
			.world_mut()
			.query::<&HcsgNode<T>>()
			.iter(app.world())
			.map(|node| (node.id, node.version))
			.collect();
		hosts.sort();
		hosts
	}

	/// Height inside the patch, off the noise lattice (noise is zero on it).
	fn patch_height(app: &App) -> anyhow::Result<Option<f32>> {
		let layout = app.world().resource::<TerrainCellLayout>();
		let at = layout.region_center_xz() + Vec3::new(0.37, 0.0, 0.61) * layout.cell_size;
		let storage = app.world().resource::<HcsgStorage>();
		storage
			.try_composed_height_at(layout, at.x, at.z)
			.map_err(|_| anyhow::anyhow!("storage busy after idle"))
	}

	#[test]
	fn the_window_presents_terrain_and_water_with_collision() -> anyhow::Result<()> {
		let mut app = app();
		settle(&mut app)?;

		let layout = app.world().resource::<TerrainCellLayout>().clone();
		let mut cells: Vec<Id> = layout
			.cell_ids(layout.request_region())
			.into_iter()
			.map(|OriginalId(id)| id)
			.collect();
		cells.sort();
		let terrain: Vec<Id> = hosts::<Terrain>(&mut app).into_iter().map(|(id, _)| id).collect();
		assert_eq!(terrain, cells, "one terrain host per layout cell");
		let water: Vec<Id> = hosts::<Water>(&mut app).into_iter().map(|(id, _)| id).collect();
		let mut wet = app
			.world()
			.resource::<HcsgStorage>()
			.overlapping::<Water>(layout.request_region());
		wet.sort();
		assert_eq!(water, wet, "one water host per wet cell");
		assert!(patch_height(&app)?.is_some());

		let colliders = app
			.world_mut()
			.query_filtered::<(), With<TerrainColliderMeshSource>>()
			.iter(app.world())
			.count();
		assert_eq!(colliders, cells.len(), "fine patch cells seed collision");
		Ok(())
	}

	#[test]
	fn a_restart_replaces_every_host_with_the_new_session() -> anyhow::Result<()> {
		let mut app = app();
		settle(&mut app)?;
		let before = hosts::<Terrain>(&mut app);
		let height = patch_height(&app)?;

		seed_resources(&mut app, 2);
		settle(&mut app)?;
		let after = hosts::<Terrain>(&mut app);

		assert_eq!(after.len(), before.len());
		for ((id, old), (new_id, new)) in before.iter().zip(&after) {
			assert_eq!(id, new_id);
			assert!(new > old, "{id:?} still shows the previous session");
		}
		assert_ne!(patch_height(&app)?, height, "the new seed reshapes the terrain");
		Ok(())
	}
}
