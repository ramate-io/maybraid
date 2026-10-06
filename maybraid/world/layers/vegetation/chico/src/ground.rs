//! [`ChicoGround`]: what Chico reads from a concrete lower model.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use lod_cascade::Chunk;
use richmond::Richmond;
use terrain_chunk_ref::TerrainChunkRef;
use terrain_layer_model::{TerrainCell, TerrainModel};
use urbanization_layer_model::Urbanization;

/// Ground Chico generates and presents against.
///
/// Heights and overlays come through [`TerrainModel`]. Durham cell size and
/// cascade-chunk construction live only on the impl for today's urbanized ground.
pub trait ChicoGround: TerrainModel {
	fn fine_overlay_size() -> f32;

	fn cascade_chunk(bounds: Aabb3d, res_2: u8) -> (Vec3, Vec3);

	/// Present groves and bump-outs. Stub grounds leave this empty.
	fn install_presentation(_app: &mut bevy::prelude::App) {}
}

impl<T> ChicoGround for Urbanization<Richmond<T>>
where
	Urbanization<Richmond<T>>: TerrainModel<Cell: TerrainCell<Mesh = durham::TerrainMeshBuilder>>,
{
	fn fine_overlay_size() -> f32 {
		durham::TERRAIN_CELL_SIZE
	}

	fn cascade_chunk(bounds: Aabb3d, res_2: u8) -> (Vec3, Vec3) {
		let cascade = durham::cascade_chunk_for_cell(bounds, res_2);
		let extent = cascade.extent.unwrap_or(Vec3::splat(cascade.size));
		(cascade.origin, extent)
	}

	fn install_presentation(app: &mut bevy::prelude::App) {
		crate::layer_present::install_chico_presentation::<Self>(app);
	}
}

/// Overlay chunk identity for a cell on ground `G`.
pub fn overlay_chunk_ref<G: ChicoGround>(
	cell: &dyn TerrainCell<Mesh = <G::Cell as TerrainCell>::Mesh>,
) -> TerrainChunkRef<<G::Cell as TerrainCell>::Mesh>
where
	<G::Cell as TerrainCell>::Mesh: render_item::mesh::IdentifiedMesh + render_item::NormalizeChunk,
{
	let (origin, extent) = G::cascade_chunk(cell.bounds(), cell.res_2());
	let chunk = Chunk::from_min_max(origin, origin + extent, None);
	TerrainChunkRef::new(cell.mesh_builder(), chunk, cell.res_2())
}

#[cfg(test)]
mod tests {
	use bevy::math::bounding::Aabb3d;
	use bevy::math::Vec3;
	use bevy::prelude::World;
	use durham::{BaseTerrainNoise, HcsgStorage, TerrainCellLayout, TerrainConfig, TerrainStorage};
	use lod_cascade::Chunk;
	use terrain_chunk_ref::TerrainChunkRef;
	use terrain_layer_model::{OnTerrain, TerrainCell};
	use urbanization_layer_model::Urbanization;

	use super::{overlay_chunk_ref, ChicoGround};

	type Urbanized = Urbanization<richmond::Richmond<OnTerrain<durham::Durham>>>;

	#[test]
	fn chunk_ref_matches_cascade_chunk_for_cell() -> anyhow::Result<()> {
		let mut world = World::new();
		let layout = TerrainCellLayout::default();
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(7));
		world.init_resource::<HcsgStorage>();
		world
			.resource_mut::<HcsgStorage>()
			.insert_base_terrain_for_test(&layout, 0, 0, base);
		let store = world.resource::<HcsgStorage>();
		let probe = Aabb3d::from_min_max(Vec3::new(1.0, -10.0, 1.0), Vec3::new(2.0, 10.0, 2.0));
		let id = store
			.terrain_ids_overlapping(probe)
			.into_iter()
			.next()
			.ok_or_else(|| anyhow::anyhow!("stored cell"))?;
		let terrain = store.terrain(id).ok_or_else(|| anyhow::anyhow!("terrain"))?;
		let built = overlay_chunk_ref::<Urbanized>(terrain);
		let cascade = durham::cascade_chunk_for_cell(terrain.bounds(), terrain.res_2());
		let extent = match cascade.extent {
			Some(extent) => extent,
			None => Vec3::splat(cascade.size),
		};
		let chunk = Chunk::from_min_max(cascade.origin, cascade.origin + extent, None);
		let manual = TerrainChunkRef::new(terrain.mesh_builder(), chunk, terrain.res_2());
		assert_eq!(built.key(), manual.key());
		Ok(())
	}

	#[test]
	fn bump_out_cells_match_terrain_rings() {
		assert!((crate::BUMP_OUT_CELL_XZ - Urbanized::fine_overlay_size()).abs() < 1e-3);
		assert!(
			(crate::MEDIUM_BUMP_OUT_CELL_XZ - 2.0 * Urbanized::fine_overlay_size()).abs() < 1e-3
		);
	}
}
