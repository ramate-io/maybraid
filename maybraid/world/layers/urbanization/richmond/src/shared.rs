//! Richmond on the shared HCSG runtime ([`lod::hcsg::shared`]): padded terrain
//! and built developments generate on the worker and present as [`HcsgNode`]
//! hosts.
//!
//! This runs beside the frame-synchronous path in [`crate::layer_stream`] and
//! [`crate::layer_present`] until every Richmond host has moved over. A
//! session starts by advancing the epoch, then [`RichmondRoots::reset`].

use std::marker::PhantomData;

use bevy::ecs::system::{SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::hcsg::shared::{self, HcsgBounds, HcsgNode, PresentationPlugin};
use lod::hcsg::universal_bounds;
use lod::scene::LodSceneRefreshChunkPlugin;
use lod::LodViewer;
use urbanization_cells::{UrbanizationExtent, UrbanizationNodes, UrbanizationSelection};

use crate::built::Built;
use crate::config::DevelopmentConfig;
use crate::developments::site::AuthoredDevelopments;
use crate::ground::RichmondGround;
use crate::layer_config::UrbanizationStreamSpec;
use crate::layer_stream::stream_radii_m;
use crate::padded::PaddedTerrain;
use crate::plugin::register_richmond_plugin;
use crate::storage::RichmondNodes;

/// The urbanization cells within the default stream's present radius of the
/// [`LodViewer`], one box each.
pub struct DevelopmentNeighborhood;

impl HcsgBounds for DevelopmentNeighborhood {
	type Param = Query<'static, 'static, &'static Transform, With<LodViewer>>;

	fn regions(viewers: &SystemParamItem<Self::Param>) -> Vec<Aabb3d> {
		let Some(viewer) = viewers.iter().next() else {
			return Vec::new();
		};
		let (present, _) = stream_radii_m(UrbanizationStreamSpec::default().stream_radius);
		let around = UrbanizationExtent::xz_radius_aabb(viewer.translation, present);
		UrbanizationExtent::cells_overlapping(around)
			.into_iter()
			.map(|cell| cell.aabb())
			.collect()
	}

	fn focus(viewers: &SystemParamItem<Self::Param>) -> Option<Vec3> {
		viewers.iter().next().map(|viewer| viewer.translation)
	}
}

/// Presents built developments over ground `G` within channel `C`'s regions
/// from the shared storage.
pub struct BuiltPresentationPlugin<C, G>(PhantomData<fn() -> (C, G)>);

impl<C, G> Default for BuiltPresentationPlugin<C, G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static, G: RichmondGround> Plugin for BuiltPresentationPlugin<C, G> {
	fn build(&self, app: &mut App) {
		register_richmond_plugin(app);
		let storage = app.world_mut().get_resource_or_init::<shared::HcsgStorage>().clone();
		UrbanizationNodes::configure(&storage);
		RichmondNodes::configure::<G>(&storage);
		app.add_plugins(PresentationPlugin::<C, Built<G>>::default());
		if !app.is_plugin_added::<LodSceneRefreshChunkPlugin<HcsgNode<Built<G>>>>() {
			app.add_plugins(LodSceneRefreshChunkPlugin::<HcsgNode<Built<G>>>::default());
		}
	}
}

/// Presents padded terrain and built developments over ground `G` within
/// channel `C`'s regions from the shared storage.
///
/// Padded terrain is the ground's whole surface, so the ground's own cells
/// are not presented beside it.
pub struct RichmondPresentationPlugin<C, G>(PhantomData<fn() -> (C, G)>);

impl<C, G> Default for RichmondPresentationPlugin<C, G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static, G: RichmondGround> Plugin for RichmondPresentationPlugin<C, G> {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			BuiltPresentationPlugin::<C, G>::default(),
			PresentationPlugin::<C, PaddedTerrain<G>>::default(),
		));
		if !app.is_plugin_added::<LodSceneRefreshChunkPlugin<HcsgNode<PaddedTerrain<G>>>>() {
			app.add_plugins(LodSceneRefreshChunkPlugin::<HcsgNode<PaddedTerrain<G>>>::default());
		}
	}
}

/// Richmond's root resources: the development config, the authored
/// developments and the urbanization selection.
#[derive(SystemParam)]
pub struct RichmondRoots<'w> {
	config: Res<'w, DevelopmentConfig>,
	authored: Res<'w, AuthoredDevelopments>,
	selection: Res<'w, UrbanizationSelection>,
}

impl RichmondRoots<'_> {
	/// Clears what Richmond and urbanization derived over ground `G` and
	/// seeds the roots. Within a restart, after the epoch has advanced.
	pub fn reset<G: RichmondGround>(&self, storage: &shared::HcsgStorage) {
		UrbanizationNodes::clear(storage);
		RichmondNodes::clear::<G>(storage);
		storage.seed(self.config.clone(), universal_bounds());
		storage.seed(self.authored.clone(), universal_bounds());
		storage.seed(self.selection.clone(), universal_bounds());
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use bevy::ecs::entity_disabling::Disabled;
	use bevy::math::bounding::Aabb3d;
	use bevy::scene::ScenePlugin;
	use bevy::state::app::StatesPlugin;
	use building_physics::BuildingWalkCollider;
	use durham::{
		fine_patch_cell_layout, CellTiling, Durham, DurhamRoots, DurhamWindow, Terrain,
		TerrainCellLayout, TerrainColliderMeshSource, TerrainConfig, TerrainMeshLodBand,
		TerrainPresentationAssets, TerrainStampConfigs, WaterPresentationAssets,
		WaterPresentationPlugin, WatershedConfigs,
	};
	use lod::gen::{Id, OriginalId, Version};
	use lod::hcsg::shared::{HcsgDemand, HcsgSystems, HcsgValue};
	use lod::lod_ref::LodNodePose;
	use lod::LodViewer;
	use terrain_layer_model::OnTerrain;
	use urbanization_layer_model::UrbanSetting;

	use super::*;
	use crate::config::DevelopmentSites;
	use crate::developments::site::{AuthoredDevelopment, DevelopmentKind};

	type Ground = OnTerrain<Durham>;

	const IDLE: Duration = Duration::from_secs(120);

	/// Restart the session from the resources on the next update.
	#[derive(Resource)]
	struct Restart(bool);

	fn restart(
		durham: DurhamRoots,
		richmond: RichmondRoots,
		storage: Res<shared::HcsgStorage>,
		demand: Res<HcsgDemand>,
		mut pending: ResMut<Restart>,
	) {
		if std::mem::take(&mut pending.0) {
			demand.advance_epoch();
			durham.reset(&storage);
			richmond.reset::<Ground>(&storage);
		}
	}

	/// One Les Halles authored inside one cell of the patch at `height`; no
	/// pad reaches the other three.
	fn authored(height: f32) -> AuthoredDevelopment {
		AuthoredDevelopment {
			cell: Aabb3d::from_min_max(Vec3::new(10.0, 0.0, 10.0), Vec3::new(150.0, 1.0, 150.0)),
			kinds: vec![DevelopmentKind::LesHalles],
			height,
			config: DevelopmentConfig::default(),
			courtyard: None,
		}
	}

	fn seed_resources(app: &mut App, height: f32) {
		app.insert_resource(TerrainStampConfigs::from_world_seed(1))
			.insert_resource(WatershedConfigs::default().with_seed(1))
			.insert_resource(TerrainPresentationAssets {
				config: TerrainConfig::new(1),
				material: Handle::default(),
				lod_bands: vec![TerrainMeshLodBand { max_radius_cells: 1, res_2: 2 }],
				outer_add_walls: false,
				fine_grid_max_radius: Some(1),
				macro_seam_half_extents: Vec::new(),
				macro_cell_min_size: None,
				macro_res_2: None,
			})
			.insert_resource(DevelopmentConfig {
				sites: DevelopmentSites::Authored,
				..DevelopmentConfig::default()
			})
			.insert_resource(AuthoredDevelopments(vec![authored(height)]))
			.insert_resource(Restart(true));
	}

	/// A 2×2 fine patch at the origin with Richmond presented over it.
	fn app(height: f32) -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, StatesPlugin))
			.add_plugins((AssetPlugin::default(), ScenePlugin))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.init_asset::<bevy::world_serialization::WorldAsset>()
			.insert_resource(fine_patch_cell_layout(1, IVec2::new(-1, -1)))
			.insert_resource(WaterPresentationAssets { material: Handle::default() })
			.init_resource::<UrbanizationSelection>()
			.add_plugins((
				shared::HcsgBoundsPlugin::<DurhamWindow>::default(),
				WaterPresentationPlugin::<DurhamWindow>::default(),
				RichmondPresentationPlugin::<DurhamWindow, Ground>::default(),
			))
			.add_systems(Update, restart.before(HcsgSystems));
		seed_resources(&mut app, height);
		app.finish();
		app.cleanup();
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
		for _ in 0..8 {
			app.update();
		}
		Ok(())
	}

	fn hosts<T: HcsgValue>(app: &mut App) -> Vec<(Id, Version)> {
		let mut hosts: Vec<_> = app
			.world_mut()
			.query::<&HcsgNode<T>>()
			.iter(app.world())
			.map(|node| (node.id, node.version))
			.collect();
		hosts.sort();
		hosts
	}

	fn count<C: Component>(app: &mut App) -> usize {
		app.world_mut()
			.query_filtered::<(), (With<C>, Allow<Disabled>)>()
			.iter(app.world())
			.count()
	}

	/// Padded height at the authored development's center.
	fn pad_height(app: &App) -> anyhow::Result<f32> {
		let cell = authored(0.0).cell;
		let center = (Vec3::from(cell.min) + Vec3::from(cell.max)) * 0.5;
		let storage = app.world().resource::<shared::HcsgStorage>();
		let padded = storage
			.overlapping::<PaddedTerrain<Ground>>(Aabb3d::new(center, Vec3::splat(0.5)))
			.into_iter()
			.find_map(|id| storage.get::<PaddedTerrain<Ground>>(id))
			.ok_or_else(|| anyhow::anyhow!("no padded cell under the development"))?;
		Ok(padded.surface.sdf.terrain().height_at_with_all_modulations(center.x, center.z))
	}

	#[test]
	fn padded_terrain_is_the_whole_ground_surface() -> anyhow::Result<()> {
		let mut app = app(12.0);
		settle(&mut app)?;

		let layout = app.world().resource::<TerrainCellLayout>().clone();
		let mut cells: Vec<Id> = layout
			.cell_ids(layout.request_region())
			.into_iter()
			.map(|OriginalId(id)| id)
			.collect();
		cells.sort();
		let padded: Vec<Id> =
			hosts::<PaddedTerrain<Ground>>(&mut app).into_iter().map(|(id, _)| id).collect();
		assert_eq!(padded, cells, "one padded host per ground cell, padded or not");
		assert!(hosts::<Terrain>(&mut app).is_empty(), "raw ground is never presented");
		assert_eq!(count::<TerrainColliderMeshSource>(&mut app), cells.len());
		assert!((pad_height(&app)? - 12.0).abs() < 1e-3, "the pad flattens to its terrace");
		Ok(())
	}

	#[test]
	fn a_built_development_presents_its_setting_and_buildings() -> anyhow::Result<()> {
		let mut app = app(12.0);
		settle(&mut app)?;

		let id = authored(12.0).id();
		let built: Vec<Id> =
			hosts::<Built<Ground>>(&mut app).into_iter().map(|(id, _)| id).collect();
		assert_eq!(built, vec![id]);
		let settings: Vec<UrbanSetting> = app
			.world_mut()
			.query_filtered::<&UrbanSetting, Allow<Disabled>>()
			.iter(app.world())
			.copied()
			.collect();
		assert_eq!(settings.len(), 1);
		assert_eq!(settings[0].id, id);
		assert!(count::<BuildingWalkCollider>(&mut app) > 0, "buildings bear weight");
		Ok(())
	}

	#[test]
	fn a_restart_replaces_padded_and_built_hosts() -> anyhow::Result<()> {
		let mut app = app(12.0);
		settle(&mut app)?;
		let padded = hosts::<PaddedTerrain<Ground>>(&mut app);
		let built = hosts::<Built<Ground>>(&mut app);

		seed_resources(&mut app, 20.0);
		settle(&mut app)?;

		for (before, after) in [
			(padded, hosts::<PaddedTerrain<Ground>>(&mut app)),
			(built, hosts::<Built<Ground>>(&mut app)),
		] {
			assert_eq!(after.len(), before.len());
			for ((id, old), (new_id, new)) in before.iter().zip(&after) {
				assert_eq!(id, new_id);
				assert!(new > old, "{id:?} still shows the previous session");
			}
		}
		assert!((pad_height(&app)? - 20.0).abs() < 1e-3, "the new terrace height");
		Ok(())
	}
}
