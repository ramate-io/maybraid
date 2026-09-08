//! [`GenerationScheme`] for [`ChicoForest`] (dependency), [`ChicoGrove`] (origins),
//! and [`CanopyBumpOut`](crate::CanopyBumpOut) (160 m canopy-proxy origins).

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use lod::scene::{LodCullRegions, LodCullRegionsStatus, OpenLattice};
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};

use crate::bump_out::{
	blend_selection_neighborhood, bump_out_cell_bounds, bump_out_cells_overlapping,
	bump_out_in_inner_hole, medium_bump_out_in_band, CanopyBumpOut, MediumCanopyBumpOut,
	BUMP_OUT_CELL_XZ, BUMP_OUT_OUTER_RADIUS_M, MEDIUM_BUMP_OUT_CELL_XZ,
};
use crate::grove::{grove_from_id, grove_id};
use crate::index::ForestIndex;
use crate::{
	presenting_recipes, ChicoForest, ChicoGrove, ForestExtent, ForestLayer,
	DEFAULT_FOREST_GROVE_TILE_XZ,
};

/// Forest selection generate ring around the camera (metres). Present is closer;
/// see [`GROVE_PRESENT_RADIUS_M`].
pub const GROVE_GENERATE_RADIUS_M: f32 = 3000.0;

/// Grove geometry present ring around the camera (metres).
pub const GROVE_PRESENT_RADIUS_M: f32 = 1000.0;

impl GenerationScheme<ForestIndex> for ChicoForest {
	fn original_ids_for(_spatial_index: &mut ForestIndex, region: Aabb3d) -> Vec<OriginalId> {
		ForestExtent::cells_overlapping(region)
			.into_iter()
			.map(|extent| OriginalId(extent.id()))
			.collect()
	}

	fn build_with_id(
		spatial_index: &mut ForestIndex,
		id: lod::gen::Id,
		_lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		let extent = ForestExtent::from_id(id)?;
		let layers = spatial_index.selected_layers_for(extent);
		Some((Self { extent, layers }, extent.aabb()))
	}

	fn descendants_with_lod(
		_id: lod::gen::Id,
		_spatial_index: &mut ForestIndex,
		_lod_ref: &LodRef,
	) {
	}
}

impl GenerationScheme<ForestIndex> for ChicoGrove {
	fn original_ids_for(spatial_index: &mut ForestIndex, region: Aabb3d) -> Vec<OriginalId> {
		let mut ids = Vec::new();
		for tile in ForestExtent::grove_tiles_overlapping(region) {
			let center = (tile.min() + tile.max()) * 0.5;
			let forest = ForestExtent::from_cell_index(
				ForestExtent::cell_index_containing(center).0,
				ForestExtent::cell_index_containing(center).1,
			);
			ensure_forest_ring(spatial_index, forest);
			let Some(selected) = SpatialIndex::<ChicoForest>::get(spatial_index, forest.id())
			else {
				continue;
			};
			let layers = selected.layers;
			let neighbors = spatial_index.neighbor_layers(forest);
			for layer in ForestLayer::ALL {
				if layer.kind(layers).is_some() || neighbors.any_kind(layer) {
					ids.push(OriginalId(grove_id(tile, layer)));
				}
			}
		}
		ids
	}

	fn build_with_id(
		spatial_index: &mut ForestIndex,
		id: lod::gen::Id,
		lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		let (extent, layer) = grove_from_id(id)?;
		let center = (extent.min() + extent.max()) * 0.5;
		let forest_extent = ForestExtent::from_cell_index(
			ForestExtent::cell_index_containing(center).0,
			ForestExtent::cell_index_containing(center).1,
		);
		GeneratingSpatialIndex::<ChicoForest>::get_or_generate(
			spatial_index,
			forest_extent.id(),
			lod_ref,
		)?;
		ensure_forest_ring(spatial_index, forest_extent);
		let forest = SpatialIndex::<ChicoForest>::get(spatial_index, forest_extent.id())?;
		let kind = layer.kind(forest.layers);
		let neighbors = spatial_index.neighbor_layers(forest_extent);
		let recipes = presenting_recipes(kind, extent, forest_extent, &neighbors, layer);
		Some((
			Self::selected(extent, layer, recipes),
			grove_id(extent, layer).origin_cell_bounds()?,
		))
	}

	fn descendants_with_lod(
		_id: lod::gen::Id,
		_spatial_index: &mut ForestIndex,
		_lod_ref: &LodRef,
	) {
	}
}

fn ensure_forests_for_bounds(index: &mut ForestIndex, bounds: Aabb3d) {
	for extent in ForestExtent::cells_overlapping(bounds) {
		index.ensure_forest_selected(extent);
	}
}

impl GenerationScheme<ForestIndex> for CanopyBumpOut {
	fn original_ids_for(_spatial_index: &mut ForestIndex, region: Aabb3d) -> Vec<OriginalId> {
		bump_out_cells_overlapping(region)
			.filter_map(|(ix, iz)| {
				let bounds = bump_out_cell_bounds(ix, iz);
				if bump_out_in_inner_hole(bounds, region) {
					return None;
				}
				Some(OriginalId(lod::gen::Id::from_cell(bounds)))
			})
			.collect()
	}

	fn build_with_id(
		spatial_index: &mut ForestIndex,
		id: lod::gen::Id,
		_lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let size = (bounds.max.x - bounds.min.x).max(1e-3);
		if (size - BUMP_OUT_CELL_XZ).abs() > 1e-2 {
			return None;
		}
		let neighborhood = Aabb3d::from_min_max(
			Vec3::new(bounds.min.x - size, bounds.min.y, bounds.min.z - size),
			Vec3::new(bounds.max.x + size, bounds.max.y, bounds.max.z + size),
		);
		ensure_forests_for_bounds(spatial_index, neighborhood);
		let samples = blend_selection_neighborhood(spatial_index, bounds);
		let cell = Self { bounds, samples };
		if !cell.has_density() {
			return None;
		}
		Some((cell, bounds))
	}

	fn descendants_with_lod(
		_id: lod::gen::Id,
		_spatial_index: &mut ForestIndex,
		_lod_ref: &LodRef,
	) {
	}
}

impl GenerationScheme<ForestIndex> for MediumCanopyBumpOut {
	fn original_ids_for(_spatial_index: &mut ForestIndex, region: Aabb3d) -> Vec<OriginalId> {
		MediumCanopyBumpOut::cells_overlapping(region)
			.filter_map(|(ix, iz)| {
				let bounds = MediumCanopyBumpOut::cell_bounds(ix, iz);
				if !medium_bump_out_in_band(bounds, region) {
					return None;
				}
				Some(OriginalId(lod::gen::Id::from_cell(bounds)))
			})
			.collect()
	}

	fn build_with_id(
		spatial_index: &mut ForestIndex,
		id: lod::gen::Id,
		_lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let size = (bounds.max.x - bounds.min.x).max(1e-3);
		if (size - MEDIUM_BUMP_OUT_CELL_XZ).abs() > 1e-2 {
			return None;
		}
		let neighborhood = Aabb3d::from_min_max(
			Vec3::new(bounds.min.x - size, bounds.min.y, bounds.min.z - size),
			Vec3::new(bounds.max.x + size, bounds.max.y, bounds.max.z + size),
		);
		ensure_forests_for_bounds(spatial_index, neighborhood);
		let samples = blend_selection_neighborhood(spatial_index, bounds);
		let bump_out = MediumCanopyBumpOut(CanopyBumpOut { bounds, samples });
		Some((bump_out, bounds))
	}
}

fn ensure_forest_ring(index: &mut ForestIndex, forest: ForestExtent) {
	index.ensure_forest_selected(forest);
	let (ix, iz) = ForestExtent::cell_index_containing(forest.center());
	index.ensure_forest_selected(ForestExtent::from_cell_index(ix, iz + 1));
	index.ensure_forest_selected(ForestExtent::from_cell_index(ix + 1, iz));
	index.ensure_forest_selected(ForestExtent::from_cell_index(ix, iz - 1));
	index.ensure_forest_selected(ForestExtent::from_cell_index(ix - 1, iz));
}

/// Grove-ring bullseye: emit a metric AABB when the driver crosses a 100 m tile.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ForestGenerateBullseye {
	pub radius_m: f32,
	pub enabled: bool,
}

impl Default for ForestGenerateBullseye {
	fn default() -> Self {
		Self { radius_m: GROVE_GENERATE_RADIUS_M, enabled: false }
	}
}

impl LodRefreshRegions for ForestGenerateBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		if !self.enabled {
			return LodRefreshRegionsStatus::Unchanged;
		}
		let previous = grove_tile_index(lod_ref.previous_transform.translation);
		let current = grove_tile_index(lod_ref.current_transform.translation);
		if current == previous {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(ForestExtent::xz_radius_aabb(
			lod_ref.current_transform.translation,
			self.radius_m,
		))
	}
}

/// Present ring — typically 1 km when generate is 3 km.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ForestPresentBullseye {
	pub radius_m: f32,
	pub enabled: bool,
}

impl Default for ForestPresentBullseye {
	fn default() -> Self {
		Self { radius_m: GROVE_PRESENT_RADIUS_M, enabled: false }
	}
}

impl LodRefreshRegions for ForestPresentBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		if !self.enabled {
			return LodRefreshRegionsStatus::Unchanged;
		}
		let previous = grove_tile_index(lod_ref.previous_transform.translation);
		let current = grove_tile_index(lod_ref.current_transform.translation);
		if current == previous {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(ForestExtent::xz_radius_aabb(
			lod_ref.current_transform.translation,
			self.radius_m,
		))
	}
}

fn grove_tile_index(position: Vec3) -> (i32, i32) {
	let s = DEFAULT_FOREST_GROVE_TILE_XZ;
	let origin = -crate::DEFAULT_FOREST_EXTENT_XZ * 0.5;
	(((position.x - origin) / s).floor() as i32, ((position.z - origin) / s).floor() as i32)
}

/// Channel marker for forest generate / present messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct ForestLodChan;

/// Present-layer cull lattice (not the scene [`OpenLattice`] resource).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ForestPresentLattice {
	pub lattice: OpenLattice,
	pub enabled: bool,
}

impl Default for ForestPresentLattice {
	fn default() -> Self {
		Self {
			lattice: OpenLattice::new(
				GROVE_PRESENT_RADIUS_M * 2.0,
				GROVE_GENERATE_RADIUS_M * 2.0 + DEFAULT_FOREST_GROVE_TILE_XZ,
				DEFAULT_FOREST_GROVE_TILE_XZ,
			),
			enabled: false,
		}
	}
}

impl ForestPresentLattice {
	pub fn from_radii_m(present_m: f32, generate_m: f32) -> Self {
		Self {
			lattice: OpenLattice::new(
				present_m.max(1.0) * 2.0,
				generate_m.max(present_m) * 2.0 + DEFAULT_FOREST_GROVE_TILE_XZ,
				DEFAULT_FOREST_GROVE_TILE_XZ,
			),
			enabled: true,
		}
	}
}

impl LodCullRegions for ForestPresentLattice {
	fn lod_cull_regions(
		&self,
		lod_refs: &[LodRef],
		cursor: &mut lod::scene::LodCullRegionCursor,
	) -> LodCullRegionsStatus {
		if !self.enabled || lod_refs.is_empty() {
			return LodCullRegionsStatus::Unchanged;
		}
		self.lattice.lod_cull_regions(lod_refs, cursor)
	}
}

/// Channel marker for canopy bump-out generate / present messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct BumpOutLodChan;

/// Channel marker for medium-terrain canopy bump-out messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct MediumBumpOutLodChan;

fn bump_out_cell_index(position: Vec3) -> (i32, i32) {
	let s = BUMP_OUT_CELL_XZ;
	((position.x / s).floor() as i32, (position.z / s).floor() as i32)
}

/// Bump-out generate bullseye: 5 km disk; [`CanopyBumpOut::original_ids_for`] skips the 1 km hole.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct BumpOutGenerateBullseye {
	pub radius_m: f32,
	pub enabled: bool,
}

impl Default for BumpOutGenerateBullseye {
	fn default() -> Self {
		Self { radius_m: BUMP_OUT_OUTER_RADIUS_M, enabled: false }
	}
}

impl LodRefreshRegions for BumpOutGenerateBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		if !self.enabled {
			return LodRefreshRegionsStatus::Unchanged;
		}
		let previous = bump_out_cell_index(lod_ref.previous_transform.translation);
		let current = bump_out_cell_index(lod_ref.current_transform.translation);
		if current == previous {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(ForestExtent::xz_radius_aabb(
			lod_ref.current_transform.translation,
			self.radius_m,
		))
	}
}

/// Bump-out present bullseye: 5 km keep AABB; tracked ids skip the 1 km grove-fill hole.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct BumpOutPresentBullseye {
	pub radius_m: f32,
	pub enabled: bool,
}

impl Default for BumpOutPresentBullseye {
	fn default() -> Self {
		Self { radius_m: BUMP_OUT_OUTER_RADIUS_M, enabled: false }
	}
}

impl LodRefreshRegions for BumpOutPresentBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		if !self.enabled {
			return LodRefreshRegionsStatus::Unchanged;
		}
		let previous = bump_out_cell_index(lod_ref.previous_transform.translation);
		let current = bump_out_cell_index(lod_ref.current_transform.translation);
		if current == previous {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(ForestExtent::xz_radius_aabb(
			lod_ref.current_transform.translation,
			self.radius_m,
		))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::gen::GeneratingSpatialIndex;
	use lod::lod_ref::LodRef;

	fn test_lod_ref(bounds: Aabb3d) -> (bevy::prelude::Transform, Aabb3d) {
		(bevy::prelude::Transform::IDENTITY, bounds)
	}

	#[test]
	fn forest_original_ids_are_overlapping_forest_cells() -> Result<()> {
		let region = ForestExtent::ring_aabb((0, 0), 1);
		let ids = ChicoForest::original_ids_for(&mut ForestIndex::default(), region);
		assert_eq!(ids.len(), 9);
		Ok(())
	}

	#[test]
	fn forest_build_is_select_only() -> Result<()> {
		let mut index = ForestIndex::default();
		index.layering = Some(crate::LayeringKind::LushJungle);
		let extent = ForestExtent::default_cell();
		let id = extent.id();
		let (identity, bounds) = test_lod_ref(extent.aabb());
		let lod_ref = LodRef {
			entity: bevy::prelude::Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		assert!(GeneratingSpatialIndex::<ChicoForest>::get_or_generate(&mut index, id, &lod_ref)
			.is_some());
		let forest = lod::gen::SpatialIndex::<ChicoForest>::get(&index, id)
			.ok_or_else(|| anyhow::anyhow!("forest"))?;
		assert_eq!(forest.layers.layering, crate::LayeringKind::LushJungle);
		assert!(forest.layers.upper_canopy.is_some());
		Ok(())
	}

	#[test]
	fn grove_origins_are_one_id_per_selected_layer() -> Result<()> {
		let mut index = ForestIndex::default();
		index.layering = Some(crate::LayeringKind::LushJungle);
		let region = ForestExtent::xz_radius_aabb(Vec3::new(50.0, 0.0, 50.0), 10.0);
		let tile = ForestExtent::grove_tiles_overlapping(region);
		assert_eq!(tile.len(), 1);
		let ids = ChicoGrove::original_ids_for(&mut index, region);
		let layers = crate::LayeringKind::LushJungle.layering().typical_layers();
		let expected = ForestLayer::ALL.iter().filter(|layer| layer.kind(layers).is_some()).count();
		assert_eq!(ids.len(), expected);
		assert!(expected > 1, "lush jungle should select more than one layer");
		Ok(())
	}

	#[test]
	fn grove_build_depends_on_forest_and_does_not_grow() -> Result<()> {
		let mut index = ForestIndex::default();
		index.layering = Some(crate::LayeringKind::LushJungle);
		let region = ForestExtent::xz_radius_aabb(Vec3::ZERO, 50.0);
		let ids = ChicoGrove::original_ids_for(&mut index, region);
		let id = ids.first().ok_or_else(|| anyhow::anyhow!("grove id"))?.0;
		let (identity, bounds) = test_lod_ref(region);
		let lod_ref = LodRef {
			entity: bevy::prelude::Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		assert!(GeneratingSpatialIndex::<ChicoGrove>::get_or_generate(&mut index, id, &lod_ref)
			.is_some());
		let grove = lod::gen::SpatialIndex::<ChicoGrove>::get(&index, id)
			.ok_or_else(|| anyhow::anyhow!("grove"))?;
		assert!(!grove.recipes.is_empty());
		assert!(grove.grown_tiles().is_none());
		let forest_id = ForestExtent::default_cell().id();
		assert!(lod::gen::SpatialIndex::<ChicoForest>::get(&index, forest_id).is_some());
		Ok(())
	}

	#[derive(bevy::prelude::Resource, Default)]
	struct GrovePresentLog {
		presented: std::collections::HashMap<lod::gen::Id, lod::gen::Version>,
	}

	#[derive(bevy::ecs::system::SystemParam)]
	struct GrovePresentParam<'w> {
		log: bevy::prelude::ResMut<'w, GrovePresentLog>,
	}

	impl lod::presentation::RegionPresenter<ChicoGrove, ForestIndex> for GrovePresentParam<'_> {
		fn presented_version(&self, id: lod::gen::Id) -> Option<lod::gen::Version> {
			self.log.presented.get(&id).copied()
		}

		fn handle(
			&mut self,
			id: lod::gen::Id,
			version: lod::gen::Version,
			grove: &ChicoGrove,
			_lod_ref: &LodRef,
		) {
			let Some(_tiles) = grove.tiles_ready_to_present(&crate::index::forest_world_sample())
			else {
				return;
			};
			self.log.presented.insert(id, version);
		}

		fn presented_ids(&self) -> Vec<lod::gen::Id> {
			self.log.presented.keys().copied().collect()
		}

		fn remove_stale(&mut self, wanted: &std::collections::HashSet<lod::gen::Id>) {
			self.log.presented.retain(|id, _| wanted.contains(id));
		}
	}

	#[test]
	fn drain_present_grows_then_spawns_on_the_next_slot() -> Result<()> {
		use bevy::prelude::*;
		use lod::gen::{SpatialIndex, Version};
		use lod::lod_ref::{LodNode, LodNodePose};
		use lod::presentation::{LodPresentBudget, LodPresentKeepRegion, LodPresentPlugin};

		use crate::index::forest_world_sample;
		use crate::{ForestGroveKind, ForestGroveRecipe};

		let extent = chico_groves::GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let grove = ChicoGrove::selected(
			extent,
			ForestLayer::UpperCanopy,
			vec![ForestGroveRecipe::uniform(ForestGroveKind::Orchard, extent)],
		);
		let id = grove.id();
		let bounds = grove.aabb();
		let (identity, lod_bounds) = test_lod_ref(bounds);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &lod_bounds,
		};
		let mut index = ForestIndex::default();
		SpatialIndex::<ChicoGrove>::insert(&mut index, id, grove, bounds, &lod_ref);

		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(index)
			.insert_resource(GrovePresentLog::default())
			.insert_resource(LodPresentBudget { ids_per_frame: 1 })
			.insert_resource({
				let mut keep = LodPresentKeepRegion::<ForestLodChan>::default();
				keep.region = Some(bounds);
				keep
			})
			.add_plugins(LodPresentPlugin::<
				ChicoGrove,
				ForestIndex,
				GrovePresentParam,
				ForestLodChan,
			>::default());
		app.world_mut().spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));
		app.update();

		{
			let log = app.world().resource::<GrovePresentLog>();
			assert!(log.presented.is_empty(), "first slot grows and does not present");
			let index = app.world().resource::<ForestIndex>();
			let grove = SpatialIndex::<ChicoGrove>::get(index, id)
				.ok_or_else(|| anyhow::anyhow!("grove after grow"))?;
			assert!(grove.grown_tiles().is_some());
			assert!(grove.tiles_ready_to_present(&forest_world_sample()).is_some());
		}

		app.update();
		let log = app.world().resource::<GrovePresentLog>();
		assert_eq!(log.presented.get(&id).copied(), Some(Version(1)));
		Ok(())
	}

	#[test]
	fn bump_out_radii_keep_the_grove_fill_hole() {
		assert!((crate::BUMP_OUT_INNER_RADIUS_M - GROVE_PRESENT_RADIUS_M).abs() < 1e-3);
		assert!((BUMP_OUT_OUTER_RADIUS_M - 5_000.0).abs() < 1e-3);
	}

	#[test]
	fn bump_out_original_ids_skip_the_inner_kilometre() -> Result<()> {
		let region = ForestExtent::xz_radius_aabb(Vec3::ZERO, BUMP_OUT_OUTER_RADIUS_M);
		let ids = CanopyBumpOut::original_ids_for(&mut ForestIndex::default(), region);
		assert!(!ids.is_empty());
		for OriginalId(id) in &ids {
			let bounds = id.origin_cell_bounds().ok_or_else(|| anyhow::anyhow!("cell"))?;
			assert!(
				!bump_out_in_inner_hole(bounds, region),
				"origin {:?} is inside the grove-fill hole",
				bounds
			);
			let size = bounds.max.x - bounds.min.x;
			assert!((size - BUMP_OUT_CELL_XZ).abs() < 1e-2);
		}
		let inner = ForestExtent::xz_radius_aabb(Vec3::ZERO, crate::BUMP_OUT_INNER_RADIUS_M * 0.5);
		let inner_ids = CanopyBumpOut::original_ids_for(&mut ForestIndex::default(), inner);
		assert!(inner_ids.is_empty());
		Ok(())
	}

	#[test]
	fn bump_out_build_is_select_only() -> Result<()> {
		let mut index = ForestIndex::default();
		index.layering = Some(crate::LayeringKind::LushJungle);
		let bounds = bump_out_cell_bounds(8, 0);
		assert!(
			crate::bump_out_chebyshev_xz(bounds, Vec3::ZERO) > crate::BUMP_OUT_INNER_RADIUS_M,
			"fixture cell should sit outside the inner hole"
		);
		let id = lod::gen::Id::from_cell(bounds);
		let (identity, lod_bounds) = test_lod_ref(bounds);
		let lod_ref = LodRef {
			entity: bevy::prelude::Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &lod_bounds,
		};
		assert!(GeneratingSpatialIndex::<CanopyBumpOut>::get_or_generate(&mut index, id, &lod_ref)
			.is_some());
		let cell = lod::gen::SpatialIndex::<CanopyBumpOut>::get(&index, id)
			.ok_or_else(|| anyhow::anyhow!("bump-out"))?;
		assert!(cell.has_density());
		let neighborhood = Aabb3d::from_min_max(
			Vec3::new(bounds.min.x - BUMP_OUT_CELL_XZ, 0.0, bounds.min.z - BUMP_OUT_CELL_XZ),
			Vec3::new(bounds.max.x + BUMP_OUT_CELL_XZ, 1.0, bounds.max.z + BUMP_OUT_CELL_XZ),
		);
		assert!(!lod::gen::SpatialIndex::<ChicoForest>::tracked_ids_for(&index, neighborhood)
			.is_empty());
		Ok(())
	}

	#[test]
	fn medium_bump_outs_use_the_medium_terrain_grid() -> Result<()> {
		let region =
			ForestExtent::xz_radius_aabb(Vec3::ZERO, crate::MEDIUM_BUMP_OUT_OUTER_RADIUS_M);
		let ids = MediumCanopyBumpOut::original_ids_for(&mut ForestIndex::default(), region);
		assert!(!ids.is_empty());
		for OriginalId(id) in ids {
			let bounds = id.origin_cell_bounds().ok_or_else(|| anyhow::anyhow!("cell"))?;
			assert!((bounds.max.x - bounds.min.x - MEDIUM_BUMP_OUT_CELL_XZ).abs() < 1e-2);
			assert!(medium_bump_out_in_band(bounds, region));
		}
		Ok(())
	}
}
