//! Maputo on the shared HCSG runtime ([`lod::hcsg::shared`]): furniture cells
//! generate on the worker from the ground's developments and present as
//! [`HcsgNode`] hosts.
//!
//! Maputo derives everything from the ground, so a session only clears it
//! through the shared HCSG session restart.

use std::collections::BTreeSet;
use std::marker::PhantomData;
use std::sync::Arc;

use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::Aabb3d;
use bevy::math::DVec3;
use bevy::prelude::*;
use building_components::{FurnitureNode, FurnitureWireframePlugin};
use furniture_assemblies::FurnitureAssembliesPlugin;
use furniture_shaders::FurnitureShadersPlugin;
use lod::gen::{Id, LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus, OriginalId};
use lod::hcsg::shared::{
	self, GenerationContext, HcsgBounds, HcsgClass, HcsgNode, PresentationPlugin,
};
use lod::lod_ref::LodRef;
use lod::scene::LodSceneRefreshChunkPlugin;
use lod::{LodViewer, SceneChunk};

use crate::cell::{
	intersects_xz, xz_radius_aabb, FurnitureCellExtent, FURNITURE_CELL_SIZE,
	FURNITURE_PRESENT_RADIUS, FURNITURE_REGION_Y_HALF,
};
use crate::host::FurnitureCell;
use crate::slots::FurnitureSlots;

/// The world-space High slots of one of `U`'s developments, under its id.
pub struct DevelopmentSlots<U> {
	pub slots: Vec<FurnitureNode>,
	_ground: PhantomData<fn() -> U>,
}

impl<U: FurnitureSlots> shared::GenerationScheme for DevelopmentSlots<U> {
	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<U::Development>(region)
	}

	/// `None` for a development with nothing to furnish.
	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let development = cx.get_or_generate::<U::Development>(id)?;
		let slots = U::development_slots(&development);
		let bounds = slots
			.iter()
			.map(|slot| slot.scene_bounds())
			.reduce(|a, b| Aabb3d { min: a.min.min(b.min), max: a.max.max(b.max) })?;
		Some((Self { slots, _ground: PhantomData }, bounds))
	}
}

/// One 50 m [`FurnitureCell`] holding the slots of `U`'s developments that
/// fall inside it.
pub struct Furnished<U> {
	pub cell: FurnitureCell,
	_ground: PhantomData<fn() -> U>,
}

impl<U: FurnitureSlots> Furnished<U> {
	/// Empty host value for tests and debug spawns.
	pub fn empty_host(extent: FurnitureCellExtent) -> (Self, Aabb3d) {
		let bounds = extent.aabb();
		(Self { cell: FurnitureCell::new(extent, Vec::new()), _ground: PhantomData }, bounds)
	}

	fn developments(cx: &mut GenerationContext, region: Aabb3d) -> Vec<Arc<DevelopmentSlots<U>>> {
		cx.original_ids_for::<DevelopmentSlots<U>>(region)
			.into_iter()
			.filter_map(|OriginalId(id)| cx.get_or_generate::<DevelopmentSlots<U>>(id))
			.collect()
	}
}

impl<U: FurnitureSlots> shared::GenerationScheme for Furnished<U> {
	/// The cells overlapping `region` that hold a slot.
	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		let mut ids = BTreeSet::new();
		for development in Self::developments(cx, region) {
			for slot in &development.slots {
				let (ix, iz) =
					FurnitureCellExtent::cell_index_containing(slot.placement.translation);
				let extent = FurnitureCellExtent::from_cell_index(ix, iz);
				if intersects_xz(region, extent.aabb()) {
					ids.insert(extent.id());
				}
			}
		}
		ids.into_iter().map(OriginalId).collect()
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let extent = FurnitureCellExtent::from_id(id)?;
		let column = xz_radius_aabb(extent.center(), FURNITURE_CELL_SIZE * 0.5);
		let slots: Vec<FurnitureNode> = Self::developments(cx, column)
			.iter()
			.flat_map(|development| development.slots.iter())
			.filter(|slot| extent.contains_xz(slot.placement.translation))
			.cloned()
			.collect();
		if slots.is_empty() {
			return None;
		}
		let cell = FurnitureCell::new(extent, slots);
		let bounds = cell.bounds();
		Some((Self { cell, _ground: PhantomData }, bounds))
	}
}

impl<U: FurnitureSlots> LodScene for Furnished<U> {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.cell.scene_lod_level(lod_ref)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.cell.scene_lod_status(lod_ref)
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		self.cell.scene_lod_culls(lod_ref, current)
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.cell.scene_with_level(lod_ref, level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		self.cell.scene_chunks_with_level(lod_ref, level)
	}
}

/// The furniture cells around the [`LodViewer`]: within
/// [`FURNITURE_PRESENT_RADIUS`] of its cell.
pub struct FurnitureNeighborhood;

impl FurnitureNeighborhood {
	fn around(viewers: &Query<&Transform, With<LodViewer>>, radius: f32) -> Option<Aabb3d> {
		let viewer = viewers.iter().next()?;
		let (ix, iz) = FurnitureCellExtent::cell_index_containing(viewer.translation);
		Some(xz_radius_aabb(FurnitureCellExtent::from_cell_index(ix, iz).center(), radius))
	}
}

impl HcsgBounds for FurnitureNeighborhood {
	const CLASS: HcsgClass = HcsgClass::Near;

	type Param = Query<'static, 'static, &'static Transform, With<LodViewer>>;

	fn regions(viewers: &SystemParamItem<Self::Param>) -> Vec<Aabb3d> {
		Self::around(viewers, FURNITURE_PRESENT_RADIUS).into_iter().collect()
	}

	fn focus(viewers: &SystemParamItem<Self::Param>) -> Option<Vec3> {
		viewers.iter().next().map(|viewer| viewer.translation)
	}
}

/// Every value Maputo derives over ground `U` in the shared storage.
pub struct MaputoNodes;

/// Index scale of furniture values: one furniture cell per bucket, over every
/// elevation a furniture region spans.
const CELL_SCALE: DVec3 = DVec3::new(
	FURNITURE_CELL_SIZE as f64,
	2.0 * FURNITURE_REGION_Y_HALF as f64,
	FURNITURE_CELL_SIZE as f64,
);

impl MaputoNodes {
	pub fn configure<U: FurnitureSlots>(storage: &shared::HcsgStorage) {
		storage
			.configure::<DevelopmentSlots<U>>(CELL_SCALE)
			.configure::<Furnished<U>>(CELL_SCALE);
	}

}

/// Presents furniture over ground `U` within channel `C`'s regions from the
/// shared storage.
pub struct MaputoPresentationPlugin<C, U>(PhantomData<fn() -> (C, U)>);

impl<C, U> Default for MaputoPresentationPlugin<C, U> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static, U: FurnitureSlots> Plugin for MaputoPresentationPlugin<C, U> {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<FurnitureShadersPlugin>() {
			app.add_plugins(FurnitureShadersPlugin);
		}
		if !app.is_plugin_added::<FurnitureAssembliesPlugin>() {
			app.add_plugins(FurnitureAssembliesPlugin);
		}
		if !app.is_plugin_added::<FurnitureWireframePlugin>() {
			app.add_plugins(FurnitureWireframePlugin);
		}
		let storage = app.world_mut().get_resource_or_init::<shared::HcsgStorage>().clone();
		MaputoNodes::configure::<U>(&storage);
		app.add_plugins(PresentationPlugin::<C, Furnished<U>>::default());
		if !app.is_plugin_added::<LodSceneRefreshChunkPlugin<HcsgNode<Furnished<U>>>>() {
			app.add_plugins(LodSceneRefreshChunkPlugin::<HcsgNode<Furnished<U>>>::default());
		}
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use bevy::ecs::entity_disabling::Disabled;
	use bevy::scene::ScenePlugin;
	use bevy::state::app::StatesPlugin;
	use durham::Durham;
	use lod::gen::Version;
	use lod::hcsg::shared::{HcsgDemand, HcsgRestartRequest, HcsgSystems, HcsgValue};
	use lod::lod_ref::LodNodePose;
	use richmond::{
		AuthoredDevelopment, AuthoredDevelopments, Built, DevelopmentConfig, DevelopmentKind,
		DevelopmentSites, Richmond, RichmondRoots,
	};
	use terrain_layer_model::OnTerrain;
	use urbanization_cells::UrbanizationSelection;
	use urbanization_layer_model::Urbanization;

	use super::*;
	use crate::colliders::FurnitureWalkCollider;

	type Ground = OnTerrain<Durham>;
	type Urban = Urbanization<Richmond<Ground>>;

	const IDLE: Duration = Duration::from_secs(120);

	/// One Les Halles authored at `height`. Authored sites plan on their own
	/// level, so no ground is generated.
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
		app.insert_resource(DevelopmentConfig {
			sites: DevelopmentSites::Authored,
			..DevelopmentConfig::default()
		})
		.insert_resource(AuthoredDevelopments(vec![authored(height)]))
		.insert_resource(HcsgRestartRequest::queued());
	}

	/// Furniture presented around a viewer at `at`.
	fn app(at: Vec3) -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, StatesPlugin))
			.add_plugins((AssetPlugin::default(), ScenePlugin))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.init_asset::<bevy::world_serialization::WorldAsset>()
			.init_resource::<UrbanizationSelection>()
			.add_plugins((
				shared::HcsgBoundsPlugin::<FurnitureNeighborhood>::default(),
				richmond::BuiltPresentationPlugin::<FurnitureNeighborhood, Ground>::default(),
				MaputoPresentationPlugin::<FurnitureNeighborhood, Urban>::default(),
			));
		seed_resources(&mut app, 12.0);
		durham::register_durham_hcsg_session(&mut app);
		app.finish();
		app.cleanup();
		let at = Transform::from_translation(at);
		app.world_mut()
			.spawn((LodViewer, at, LodNodePose { previous: at, current: at }));
		app
	}

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

	fn center() -> Vec3 {
		let cell = authored(0.0).cell;
		(Vec3::from(cell.min) + Vec3::from(cell.max)) * 0.5
	}

	/// Cells holding the development's slots within the viewer's
	/// neighborhood, with how many slots each holds.
	fn expected_cells(app: &mut App) -> anyhow::Result<Vec<(Id, usize)>> {
		let mut viewers = app.world_mut().query_filtered::<&Transform, With<LodViewer>>();
		let viewer = *viewers.single(app.world())?;
		let (ix, iz) = FurnitureCellExtent::cell_index_containing(viewer.translation);
		let inner = xz_radius_aabb(
			FurnitureCellExtent::from_cell_index(ix, iz).center(),
			FURNITURE_PRESENT_RADIUS,
		);
		let storage = app.world().resource::<shared::HcsgStorage>();
		let built = storage
			.get::<Built<Ground>>(authored(0.0).id())
			.ok_or_else(|| anyhow::anyhow!("the development was not built"))?;
		let mut cells = std::collections::BTreeMap::<Id, usize>::new();
		for slot in Urban::development_slots(&built) {
			let (ix, iz) = FurnitureCellExtent::cell_index_containing(slot.placement.translation);
			let extent = FurnitureCellExtent::from_cell_index(ix, iz);
			if intersects_xz(inner, extent.aabb()) {
				*cells.entry(extent.id()).or_default() += 1;
			}
		}
		Ok(cells.into_iter().collect())
	}

	fn furnished(app: &mut App) -> Vec<(Id, usize)> {
		let mut cells: Vec<_> = app
			.world_mut()
			.query::<&HcsgNode<Furnished<Urban>>>()
			.iter(app.world())
			.map(|node| (node.id, node.value.cell.slots.len()))
			.collect();
		cells.sort();
		cells
	}

	#[test]
	fn the_neighborhood_presents_every_furnished_cell_with_walk_colliders() -> anyhow::Result<()> {
		let mut app = app(center());
		settle(&mut app)?;

		let expected = expected_cells(&mut app)?;
		assert!(!expected.is_empty(), "Les Halles furnishes the neighborhood");
		assert_eq!(furnished(&mut app), expected, "one host per furnished cell, with its slots");
		let colliders = app
			.world_mut()
			.query_filtered::<(), (With<FurnitureWalkCollider>, Allow<Disabled>)>()
			.iter(app.world())
			.count();
		assert!(colliders > 0, "furniture bears weight");
		Ok(())
	}

	#[test]
	fn leaving_the_neighborhood_retires_its_hosts() -> anyhow::Result<()> {
		let mut app = app(center());
		settle(&mut app)?;
		assert!(!hosts::<Furnished<Urban>>(&mut app).is_empty());

		let away = Transform::from_xyz(5_000.0, 0.0, 5_000.0);
		let mut viewers = app.world_mut().query_filtered::<&mut Transform, With<LodViewer>>();
		*viewers.single_mut(app.world_mut())? = away;
		settle(&mut app)?;
		assert!(hosts::<Furnished<Urban>>(&mut app).is_empty());
		Ok(())
	}

	#[test]
	fn a_restart_replaces_every_furnished_host() -> anyhow::Result<()> {
		let mut app = app(center());
		settle(&mut app)?;
		let before = hosts::<Furnished<Urban>>(&mut app);

		seed_resources(&mut app, 20.0);
		settle(&mut app)?;
		let after = hosts::<Furnished<Urban>>(&mut app);

		assert_eq!(after.len(), before.len());
		for ((id, old), (new_id, new)) in before.iter().zip(&after) {
			assert_eq!(id, new_id);
			assert!(new > old, "{id:?} still shows the previous session");
		}
		Ok(())
	}
}
