//! Durham's view of [`HcsgStorage`]: node registration and terrain read helpers.

use crate::terrain::base_noise::BaseTerrainNoise;
use crate::terrain::cell::{
	cell_bounds, universal_bounds, CellTiling, TerrainCellLayout, TERRAIN_CELL_SIZE,
	TERRAIN_CELL_VERTICAL_HALF_EXTENT,
};
use crate::terrain::geography::GeographicFeature;
use crate::terrain::presentation::TerrainPresentationAssets;
use crate::terrain::stamps::{
	CanyonHighPassControllerCell, CanyonHighPassControllerLayout, CanyonHighPassStampCell,
	CanyonLowPassControllerCell, CanyonLowPassControllerLayout, CanyonLowPassStampCell,
	MassifHighPassControllerCell, MassifHighPassControllerLayout, MassifHighPassStampCell,
	MassifLowPassControllerCell, MassifLowPassControllerLayout, MassifLowPassStampCell,
	PlateauHighPassControllerCell, PlateauHighPassControllerLayout, PlateauHighPassStampCell,
	PlateauLowPassControllerCell, PlateauLowPassControllerLayout, PlateauLowPassStampCell,
	PocketWaterHighPassControllerCell, PocketWaterHighPassControllerLayout,
	PocketWaterHighPassStampCell, PocketWaterLowPassControllerCell,
	PocketWaterLowPassControllerLayout, PocketWaterLowPassStampCell, RollingHighPassControllerCell,
	RollingHighPassControllerLayout, RollingHighPassStampCell, RollingLowPassControllerCell,
	RollingLowPassControllerLayout, RollingLowPassStampCell, TerrainStampConfigs,
	ValleyHighPassControllerCell, ValleyHighPassControllerLayout, ValleyHighPassStampCell,
	ValleyLowPassControllerCell, ValleyLowPassControllerLayout, ValleyLowPassStampCell,
};
use crate::terrain::watersheds::{
	HydroComplexCell, PocketHighPassCell, PocketLowPassCell, PocketWatersHighPass,
	PocketWatersLowPass, PrePocketHighPassCell, PrePocketHighPassLayout, PrePocketLowPassCell,
	PrePocketLowPassLayout, WatershedAproningCell, WatershedCarvingCell, WatershedConfigs,
	WatershedRimmingCell,
};
use crate::terrain::{PreWatershedTerrain, Terrain};
use crate::water::{ComposedWater, Water, WaterColumn, WaterPresentationAssets};
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::math::DVec3;
use bevy::prelude::*;
use lod::gen::{Id, OriginalId, Version};
use lod::hcsg::shared::{self, HcsgDemand};
use lod::hcsg::HcsgStorage;
use procedural_common::Bounds2;
use std::collections::HashMap;
use std::sync::Arc;

/// [`HcsgStorage`] group holding every node Durham derives from its seeded
/// roots. A rebuild drops the group; the roots stay seeded.
pub struct DurhamNodes;

/// Tall origin-cell buckets for every Durham node.
const DURHAM_BASE_SCALE: DVec3 = DVec3::new(
	TERRAIN_CELL_SIZE as f64,
	2.0 * TERRAIN_CELL_VERTICAL_HALF_EXTENT as f64,
	TERRAIN_CELL_SIZE as f64,
);

macro_rules! durham_nodes {
	($($T:ty),* $(,)?) => {
		/// Configures Durham's stores and joins them to [`DurhamNodes`].
		pub fn register_durham_nodes(storage: &mut HcsgStorage) {
			$(storage.configure::<$T>(DURHAM_BASE_SCALE).add_to_group::<DurhamNodes, $T>();)*
		}

		impl DurhamNodes {
			/// Configures Durham's stores in the shared storage.
			pub fn configure(storage: &shared::HcsgStorage) {
				$(storage.configure::<$T>(DURHAM_BASE_SCALE);)*
			}

			/// Drops every derived Durham value from the shared storage.
			fn clear(storage: &shared::HcsgStorage) {
				$(storage.clear::<$T>();)*
			}
		}
	};
}

durham_nodes!(
	BaseTerrainNoise,
	PreWatershedTerrain,
	Terrain,
	Water,
	PrePocketLowPassLayout,
	PrePocketLowPassCell,
	PocketLowPassCell,
	PocketWatersLowPass,
	PrePocketHighPassLayout,
	PrePocketHighPassCell,
	PocketHighPassCell,
	PocketWatersHighPass,
	HydroComplexCell,
	WatershedCarvingCell,
	WatershedRimmingCell,
	WatershedAproningCell,
	PlateauLowPassControllerLayout,
	PlateauLowPassControllerCell,
	PlateauLowPassStampCell,
	PlateauHighPassControllerLayout,
	PlateauHighPassControllerCell,
	PlateauHighPassStampCell,
	MassifLowPassControllerLayout,
	MassifLowPassControllerCell,
	MassifLowPassStampCell,
	MassifHighPassControllerLayout,
	MassifHighPassControllerCell,
	MassifHighPassStampCell,
	CanyonLowPassControllerLayout,
	CanyonLowPassControllerCell,
	CanyonLowPassStampCell,
	CanyonHighPassControllerLayout,
	CanyonHighPassControllerCell,
	CanyonHighPassStampCell,
	PocketWaterLowPassControllerLayout,
	PocketWaterLowPassControllerCell,
	PocketWaterLowPassStampCell,
	PocketWaterHighPassControllerLayout,
	PocketWaterHighPassControllerCell,
	PocketWaterHighPassStampCell,
	RollingLowPassControllerLayout,
	RollingLowPassControllerCell,
	RollingLowPassStampCell,
	RollingHighPassControllerLayout,
	RollingHighPassControllerCell,
	RollingHighPassStampCell,
	ValleyLowPassControllerLayout,
	ValleyLowPassControllerCell,
	ValleyLowPassStampCell,
	ValleyHighPassControllerLayout,
	ValleyHighPassControllerCell,
	ValleyHighPassStampCell,
);

/// Durham's seeded root inputs as live resources.
///
/// The [`crate::TerrainWindow`] producer and [`lod::hcsg::Seed`] keep these
/// seeded during streaming. Hosts that regenerate synchronously reseed here.
#[derive(SystemParam)]
pub struct DurhamRoots<'w> {
	layout: Res<'w, TerrainCellLayout>,
	stamps: Res<'w, TerrainStampConfigs>,
	watersheds: Res<'w, WatershedConfigs>,
	terrain_assets: Res<'w, TerrainPresentationAssets>,
	water_assets: Res<'w, WaterPresentationAssets>,
}

impl DurhamRoots<'_> {
	pub fn layout(&self) -> &TerrainCellLayout {
		&self.layout
	}

	/// Drops every derived Durham node and seeds the roots from the resources.
	pub fn reseed(&self, storage: &mut HcsgStorage) {
		storage.clear_group::<DurhamNodes>();
		storage.seed(self.layout.clone(), universal_bounds());
		storage.seed(self.stamps.clone(), universal_bounds());
		storage.seed(self.watersheds.clone(), universal_bounds());
		storage.seed(self.terrain_assets.clone(), universal_bounds());
		storage.seed(self.water_assets.clone(), universal_bounds());
	}

	/// Starts a new shared session from the resources: ends the epoch (every
	/// layer's subscriptions, so in-flight values are dropped), clears Durham's
	/// derived values, then seeds the roots.
	pub fn restart(&self, storage: &shared::HcsgStorage, demand: &HcsgDemand) {
		demand.advance_epoch();
		DurhamNodes::clear(storage);
		storage.seed(self.layout.clone(), universal_bounds());
		storage.seed(self.stamps.clone(), universal_bounds());
		storage.seed(self.watersheds.clone(), universal_bounds());
		storage.seed(self.terrain_assets.clone(), universal_bounds());
		storage.seed(self.water_assets.clone(), universal_bounds());
	}
}

/// Cheap owned view of composed height fields for background consumers.
#[derive(Clone, Default)]
pub struct TerrainHeightSnapshot {
	terrain: Arc<HashMap<Id, Arc<crate::terrain::ComposedTerrain>>>,
}

impl TerrainHeightSnapshot {
	pub fn composed_height_at(&self, layout: &TerrainCellLayout, x: f32, z: f32) -> Option<f32> {
		origin_cell_ids_at(layout, x, z)
			.find_map(|id| self.terrain.get(&id))
			.map(|sdf| sdf.terrain().height_at_with_all_modulations(x, z))
	}
}

/// Cheap owned view of composed wet columns for buoyancy and placement.
#[derive(Clone, Default)]
pub struct WaterSurfaceSnapshot {
	water: Arc<HashMap<Id, Arc<ComposedWater>>>,
}

impl WaterSurfaceSnapshot {
	pub fn column(&self, layout: &TerrainCellLayout, x: f32, z: f32) -> Option<WaterColumn> {
		origin_cell_ids_at(layout, x, z)
			.find_map(|id| self.water.get(&id))
			.and_then(|sdf| sdf.column_at(x, z))
	}
}

/// Origin ids covering `(x, z)`: fine grid first, then outer and stream rings.
pub(crate) fn origin_cell_ids_at(
	layout: &TerrainCellLayout,
	x: f32,
	z: f32,
) -> impl Iterator<Item = Id> + '_ {
	std::iter::once(layout.cell_size)
		.chain(layout.outer_rings.iter().map(|outer| outer.cell_size))
		.chain(layout.stream_rings.iter().map(|ring| ring.cell_size))
		.map(move |size| {
			let size = size.max(1e-3);
			Id::from_cell(cell_bounds(
				(x / size).floor() as i32,
				(z / size).floor() as i32,
				size,
				layout.vertical_half_extent,
			))
		})
}

/// Terrain reads over [`HcsgStorage`].
pub trait TerrainStorage {
	fn terrain(&self, id: Id) -> Option<&Terrain>;

	/// Number of stored terrain origin cells.
	fn terrain_count(&self) -> usize;

	/// Advances whenever a terrain cell is stored, moved, or dropped.
	fn terrain_revision(&self) -> u64;

	/// Advances whenever a store read by [`Self::geographic_features_overlapping`]
	/// changes membership. Other writes, including other layers', leave it alone.
	fn geography_revision(&self) -> u64;

	/// Origin ids already stored in `region` (GET; does not admit missing cells).
	fn terrain_ids_overlapping(&self, region: Aabb3d) -> Vec<Id>;

	/// Every origin cell of `layout`'s request window is stored. Generation
	/// admits a few cells per frame, so a stamp read earlier misses the rest.
	fn fills_layout(&self, layout: &TerrainCellLayout) -> bool;

	fn water(&self, id: Id) -> Option<&Water>;

	fn water_version(&self, id: Id) -> Option<Version>;

	fn base_noise(&self) -> Option<&BaseTerrainNoise>;

	fn height_snapshot(&self) -> TerrainHeightSnapshot;

	fn water_snapshot(&self) -> WaterSurfaceSnapshot;

	/// Wet column at `(x, z)` when a covering water cell is stored.
	fn water_column_at(&self, layout: &TerrainCellLayout, x: f32, z: f32) -> Option<WaterColumn>;

	/// Composed terrain height (jersey + Watershed) at `(x, z)`, if that cell is stored.
	fn composed_height_at(&self, layout: &TerrainCellLayout, x: f32, z: f32) -> Option<f32>;

	/// Stored geographic sources whose leaf bounds overlap `region`.
	///
	/// Reads family-specific stamp records (skipping empty modulation lists) and
	/// authored high/low-pass watershed leaves (skipping
	/// [`crate::terrain::watersheds::PocketWater::Empty`]). Does not generate
	/// cells or walk [`HydroComplexCell`].
	fn geographic_features_overlapping(
		&self,
		region: Bounds2,
	) -> impl Iterator<Item = GeographicFeature> + '_;

	/// Store one origin cell whose SDF is `base` with no jersey / hydro ops.
	///
	/// Bounds match [`Self::composed_height_at`]'s lookup for `(ix, iz)` on `layout`.
	/// Only for composed-surface parity tests — not a production insert path.
	#[doc(hidden)]
	fn insert_base_terrain_for_test(
		&mut self,
		layout: &TerrainCellLayout,
		ix: i32,
		iz: i32,
		base: BaseTerrainNoise,
	);
}

impl TerrainStorage for HcsgStorage {
	fn terrain(&self, id: Id) -> Option<&Terrain> {
		self.get::<Terrain>(id)
	}

	fn terrain_count(&self) -> usize {
		self.store::<Terrain>().map_or(0, |store| store.len())
	}

	fn terrain_revision(&self) -> u64 {
		lod::gen::SpatialIndex::<Terrain>::membership_revision(self)
	}

	fn geography_revision(&self) -> u64 {
		crate::terrain::geography::geography_revision(self)
	}

	fn terrain_ids_overlapping(&self, region: Aabb3d) -> Vec<Id> {
		self.overlapping::<Terrain>(region)
	}

	fn fills_layout(&self, layout: &TerrainCellLayout) -> bool {
		layout
			.cell_ids(layout.request_region())
			.into_iter()
			.all(|OriginalId(id)| self.contains::<Terrain>(id))
	}

	fn water(&self, id: Id) -> Option<&Water> {
		self.get::<Water>(id)
	}

	fn water_version(&self, id: Id) -> Option<Version> {
		self.entry::<Water>(id).map(|entry| entry.version)
	}

	fn base_noise(&self) -> Option<&BaseTerrainNoise> {
		self.get::<BaseTerrainNoise>(Id::Universal)
	}

	fn height_snapshot(&self) -> TerrainHeightSnapshot {
		let terrain = self
			.store::<Terrain>()
			.into_iter()
			.flat_map(|store| store.iter())
			.map(|(id, entry)| (id, Arc::clone(&entry.value.sdf)))
			.collect();
		TerrainHeightSnapshot { terrain: Arc::new(terrain) }
	}

	fn water_snapshot(&self) -> WaterSurfaceSnapshot {
		let water = self
			.store::<Water>()
			.into_iter()
			.flat_map(|store| store.iter())
			.map(|(id, entry)| (id, Arc::new(entry.value.sdf.clone())))
			.collect();
		WaterSurfaceSnapshot { water: Arc::new(water) }
	}

	fn water_column_at(&self, layout: &TerrainCellLayout, x: f32, z: f32) -> Option<WaterColumn> {
		origin_cell_ids_at(layout, x, z)
			.find_map(|id| self.get::<Water>(id))
			.and_then(|water| water.column_at(x, z))
	}

	fn composed_height_at(&self, layout: &TerrainCellLayout, x: f32, z: f32) -> Option<f32> {
		origin_cell_ids_at(layout, x, z)
			.find_map(|id| self.get::<Terrain>(id))
			.map(|terrain| terrain.sdf.terrain().height_at_with_all_modulations(x, z))
	}

	fn geographic_features_overlapping(
		&self,
		region: Bounds2,
	) -> impl Iterator<Item = GeographicFeature> + '_ {
		crate::terrain::geography::geographic_features_overlapping(self, region).into_iter()
	}

	fn insert_base_terrain_for_test(
		&mut self,
		layout: &TerrainCellLayout,
		ix: i32,
		iz: i32,
		base: BaseTerrainNoise,
	) {
		let cell = cell_bounds(ix, iz, layout.cell_size, layout.vertical_half_extent);
		let sdf = Arc::new(Terrain::compose_sdf(&base, &[]));
		let terrain = Terrain {
			cell,
			base,
			modulations: Vec::new(),
			jersey_leaves: Vec::new(),
			marazion_leaves: Vec::new(),
			marazion_fills: Vec::new(),
			sdf,
			material: Handle::default(),
			res_2: 0,
			stream_ring: None,
			wall_faces: render_item::sdf::cpu_shot::WallFaces::NONE,
		};
		self.insert(Id::from_cell(cell), terrain, cell);
	}
}

#[cfg(test)]
pub(crate) fn insert_water_for_test(storage: &mut HcsgStorage, water: Water) {
	let bounds = water.cell;
	storage.insert(Id::from_cell(bounds), water, bounds);
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::terrain::config::TerrainConfig;

	#[test]
	fn clearing_durham_nodes_advances_versions_past_the_previous_epoch() -> anyhow::Result<()> {
		let mut storage = HcsgStorage::default();
		register_durham_nodes(&mut storage);
		let layout = TerrainCellLayout::default();
		storage.insert_base_terrain_for_test(
			&layout,
			0,
			0,
			BaseTerrainNoise::from_config(&TerrainConfig::new(7)),
		);
		let id = storage.terrain_ids_overlapping(layout.request_region())[0];
		let previous =
			storage.entry::<Terrain>(id).ok_or_else(|| anyhow::anyhow!("inserted"))?.version;
		storage.clear_group::<DurhamNodes>();
		assert_eq!(storage.terrain_count(), 0);
		storage.insert_base_terrain_for_test(
			&layout,
			0,
			0,
			BaseTerrainNoise::from_config(&TerrainConfig::new(7)),
		);
		let rebuilt = storage
			.entry::<Terrain>(id)
			.ok_or_else(|| anyhow::anyhow!("reinserted"))?
			.version;
		assert!(rebuilt > previous, "restamp must not reuse a presented version");
		Ok(())
	}
}
