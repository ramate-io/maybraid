//! Durham terrain model: SDF composition, LOD generation, Avian index, render.

pub mod cell;
pub mod collider;
pub mod compose;
pub mod index;
pub mod plugin;
pub mod presentation;
pub mod region;
pub mod render;
pub mod sdf;

use crate::terrain::cell::{cell_bounds, cell_coords_for_region, HasTerrainCellLayout};
use crate::terrain::presentation::HasTerrainPresentationAssets;
use crate::terrain::render::cascade_chunk_for_cell;
use avian3d::prelude::RigidBody;
use bevy::ecs::template::template;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use durham_terrain::shaders::DurhamTerrainShader;
use lod::gen::{GenerationScheme, Id, LodScene, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use render_item::mesh::handle::Cached;

pub use cell::TERRAIN_CELL_SIZE;
pub use collider::TerrainTrimeshCollider;
pub use compose::{create_terrain, TerrainConfig};
pub use index::{AvianTerrainIndex, TerrainCellId, TerrainEntryStore};
pub use plugin::{register_terrain_plugin, TerrainPlugin};
pub use presentation::{
	TerrainPresentationAssets, TerrainPresenterState, TerrainRegionPresenter, TerrainStoreView,
};
pub use render::TerrainRenderItem;
pub use sdf::{ComposedTerrain, ElevationModulation, TerrainSdf};

pub use cell::TerrainCellLayout;

/// Top-level terrain cell model.
///
/// Other layers request this when they need elevation / terrain occupancy.
/// Presentation fields are snapshotted at generation time so [`LodScene`] can
/// build a self-contained `bsn!` scene without looking up world resources.
#[derive(Debug, Clone, Component)]
pub struct Terrain {
	pub cell: Aabb3d,
	pub sdf: ComposedTerrain,
	pub material: Handle<DurhamTerrainShader>,
	pub res_2: u8,
}

impl Terrain {
	/// Visual scene for one cell: cascade chunk + cached SDF mesh dispatch.
	///
	/// Bare locals (`transform`, `chunk`) are not valid `bsn!` entries — those
	/// positions are type/patch syntax. Pre-built `Template` values go through
	/// [`template_value`]; [`Cached`] is not `Default`/`FromTemplate` yet, so it
	/// uses [`template`].
	pub fn scene(&self) -> impl Scene + 'static {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2);
		let transform = Transform::from_translation(chunk.origin);
		let sdf = self.sdf.clone();
		let material = self.material.clone();
		bsn! {
			template_value(transform)
			template_value(chunk)
			template(move |_ctx| Ok(Cached::new(sdf.clone())))
			MeshMaterial3d::<DurhamTerrainShader>({material.clone()})
			// Static body + marker; trimesh colliders are queued on mesh children
			// once `fetch_meshes` spawns them (see `collider::queue_terrain_trimesh_colliders`).
			// `RigidBody` lacks VariantDefaults, so use `template` for Static.
			template(move |_ctx| Ok(RigidBody::Static))
			TerrainTrimeshCollider
		}
	}
}

impl LodScene for Terrain {
	fn scene_with_lod(&self, _lod_ref: &LodRef) -> impl Scene + 'static {
		self.scene()
	}
}

/// Terrain has no generation dependencies; `S` must store terrain, expose cell
/// layout, and supply presentation assets for scene snapshots.
///
/// Otherwise we might bound e.g. `S: GeneratingSpatialIndex<HydrologyGraph>`.
impl<S> GenerationScheme<S> for Terrain
where
	S: SpatialIndex<Terrain> + HasTerrainCellLayout + HasTerrainPresentationAssets,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		let layout = spatial_index.cell_layout().clone();
		cell_coords_for_region(region, layout.cell_size)
			.map(|(ix, iz)| {
				let bounds = cell_bounds(ix, iz, layout.cell_size, layout.vertical_half_extent);
				OriginalId(Id::from_cell(bounds))
			})
			.filter(|OriginalId(id)| id.origin_cell_bounds().is_some_and(|b| region.intersects(&b)))
			.collect()
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let assets = spatial_index.presentation_assets();
		Some((
			Self {
				cell: bounds,
				sdf: assets.sdf.clone(),
				material: assets.material.clone(),
				res_2: assets.res_2,
			},
			bounds,
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
