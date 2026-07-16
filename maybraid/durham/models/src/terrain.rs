//! Durham terrain model: SDF composition, LOD generation, Avian index, render.

pub mod cell;
pub mod compose;
pub mod index;
pub mod plugin;
pub mod region;
pub mod render;
pub mod sdf;

use crate::terrain::cell::{cell_bounds, cell_coords_for_region, HasTerrainCellLayout};
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use bevy::scene::{ResolveContext, ResolvedScene, Scene, SceneFunction};
use lod::gen::{GenerationScheme, Id, LodScene, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;

pub use cell::TERRAIN_CELL_SIZE;
pub use compose::{create_terrain, TerrainConfig};
pub use index::{AvianTerrainIndex, TerrainCellId, TerrainEntryStore};
pub use plugin::{register_terrain_plugin, TerrainPlugin};
pub use render::TerrainRenderItem;
pub use sdf::{ComposedTerrain, ElevationModulation, TerrainSdf};

pub use cell::TerrainCellLayout;

/// Top-level terrain cell model.
///
/// Other layers request this when they need elevation / terrain occupancy.
#[derive(Debug, Clone, Component)]
pub struct Terrain {
	pub cell: Aabb3d,
}

fn empty_scene(_: &mut ResolveContext, _: &mut ResolvedScene) {}

impl LodScene for Terrain {
	fn scene_with_lod(&self, _lod_ref: &LodRef) -> impl Scene + 'static {
		SceneFunction(empty_scene)
	}
}

/// Terrain has no generation dependencies; `S` must store terrain and expose cell layout.
///
/// Otherwise we might bound e.g. `S: GeneratingSpatialIndex<HydrologyGraph>`.
impl<S> GenerationScheme<S> for Terrain
where
	S: SpatialIndex<Terrain> + HasTerrainCellLayout,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		let layout = spatial_index.cell_layout().clone();
		cell_coords_for_region(region, layout.cell_size)
			.map(|(ix, iz)| {
				let bounds =
					cell_bounds(ix, iz, layout.cell_size, layout.vertical_half_extent);
				OriginalId(Id::from_cell(bounds))
			})
			.filter(|OriginalId(id)| {
				id.origin_cell_bounds().is_some_and(|b| region.intersects(&b))
			})
			.collect()
	}

	fn build_with_id(_spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		Some((Self { cell: bounds }, bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
