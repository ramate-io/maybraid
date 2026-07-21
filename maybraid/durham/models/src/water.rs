//! Durham water model: collect stamp-owned fills against composed terrain and mesh them.
//!
//! Parallel to [`crate::terrain`]: terrain owns the heightfield; this module owns
//! water fill products, evaluation, and presentation. Surface level / softmask
//! boundary remain decisions of **Marazion** lake stamps.

pub mod composed;
pub mod plugin;
pub mod presentation;

use crate::terrain::cell::{original_ids_for_origin_cells, TerrainCellLayout};
use crate::terrain::marazion::{
	original_ids_for_marazion_lake_leaves, MarazionLakeCell, MarazionLakeLayout,
};
use crate::terrain::render::cascade_chunk_for_water_cell;
use crate::terrain::sdf::TerrainSdf;
use crate::terrain::Terrain;
use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use jersey_terrain_stamps::Region2D;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, LodScene, OriginalId};
use lod::lod_ref::LodRef;
use marazion_watersheds::WaterFill;
use render_item::mesh::handle::Cached;
use sdf::Sdf;

pub use composed::ComposedWater;
pub use plugin::{register_water_plugin, WaterPlugin};
pub use presentation::{
	BootstrapWaterPresentationAssets, PresentedWaterScene, WaterPresentationAssets,
	WaterPresenterState, WaterRegionPresenter, WaterStoreView,
};

/// Cell-level water collector over [`Terrain`] plus Marazion stamp fills.
#[derive(Debug, Clone, Component)]
pub struct Water {
	pub cell: Aabb3d,
	/// Composed terrain heightfield used when evaluating fills.
	pub terrain: TerrainSdf,
	/// Stamp-owned lake fills (no re-derivation of W / shore).
	pub fills: Vec<WaterFill>,
	/// Meshable union of fills against [`Self::terrain`].
	pub sdf: ComposedWater,
	pub material: Handle<StandardMaterial>,
	pub res_2: u8,
}

impl Water {
	/// Union of stamp fills at `p` given a terrain height sample.
	pub fn water_distance(&self, p: Vec3, terrain_height: f32) -> f32 {
		water_distance(&self.fills, p, terrain_height)
	}

	/// Union of stamp fills against this collector's composed heightfield.
	pub fn water_distance_at(&self, p: Vec3) -> f32 {
		self.sdf.distance(p)
	}

	/// Visual scene for one cell: cascade chunk + cached SDF mesh dispatch.
	///
	/// Uses the same XZ/`res_2` grid as terrain, with Y fitted to the water slab
	/// so marching cubes can resolve basin depth (terrain cell Y is ~km-scale).
	pub fn scene(&self) -> impl Scene + 'static {
		let (y_min, y_max) = water_mesh_y_span(&self.fills, &self.terrain);
		let chunk = cascade_chunk_for_water_cell(self.cell, self.res_2, y_min, y_max);
		let transform = Transform::from_translation(chunk.origin);
		let sdf = self.sdf.clone();
		let material = self.material.clone();
		bsn! {
			template_value(transform)
			template_value(chunk)
			template(move |_ctx| Ok(Cached::new(sdf.clone())))
			MeshMaterial3d::<StandardMaterial>({material.clone()})
		}
	}
}

impl LodScene for Water {
	fn scene_with_lod(&self, _lod_ref: &LodRef) -> impl Scene + 'static {
		self.scene()
	}
}

/// Min distance over stamp fills (empty → large positive / dry).
pub fn water_distance(fills: &[WaterFill], p: Vec3, terrain_height: f32) -> f32 {
	fills
		.iter()
		.map(|fill| fill.distance(p, terrain_height))
		.fold(f32::INFINITY, f32::min)
}

/// True when the stamp surface sits above terrain at the fill footprint center.
fn fill_has_wet_volume(fill: &WaterFill, terrain: &TerrainSdf) -> bool {
	let center = match &fill.region {
		Region2D::Circle(c) => c.center,
		Region2D::Rect(r) => r.center,
	};
	let h = terrain.height_at_with_all_modulations(center.x, center.y);
	fill.water_level > h + 1.0e-2
}

/// Vertical span for water meshing: cover `[terrain_h, W]` per fill with pad.
fn water_mesh_y_span(fills: &[WaterFill], terrain: &TerrainSdf) -> (f32, f32) {
	let mut y_lo = f32::INFINITY;
	let mut y_hi = f32::NEG_INFINITY;
	for fill in fills {
		let center = match &fill.region {
			Region2D::Circle(c) => c.center,
			Region2D::Rect(r) => r.center,
		};
		let h = terrain.height_at_with_all_modulations(center.x, center.y);
		y_lo = y_lo.min(h).min(fill.water_level);
		y_hi = y_hi.max(h).max(fill.water_level);
	}
	if !y_lo.is_finite() || !y_hi.is_finite() {
		return (-50.0, 50.0);
	}
	let depth = (y_hi - y_lo).abs().max(4.0);
	let pad = (depth * 0.5).max(8.0);
	(y_lo - pad, y_hi + pad)
}

fn collect_marazion_lake_fills<S>(
	spatial_index: &mut S,
	region: Aabb3d,
	lod_ref: &LodRef,
) -> Option<Vec<WaterFill>>
where
	S: GeneratingSpatialIndex<MarazionLakeCell> + GeneratingSpatialIndex<MarazionLakeLayout>,
{
	let mut fills = Vec::new();
	let mut ids = original_ids_for_marazion_lake_leaves(spatial_index, region);
	ids.sort_by(|a, b| a.0.cmp(&b.0));
	for OriginalId(id) in ids {
		fills.extend(
			GeneratingSpatialIndex::<MarazionLakeCell>::get_one_or_generate(
				spatial_index,
				id,
				lod_ref,
			)?
			.fills
			.iter()
			.cloned(),
		);
	}
	Some(fills)
}

impl<S> GenerationScheme<S> for Water
where
	S: GeneratingSpatialIndex<Terrain>
		+ GeneratingSpatialIndex<MarazionLakeCell>
		+ GeneratingSpatialIndex<MarazionLakeLayout>
		+ GeneratingSpatialIndex<TerrainCellLayout>
		+ GeneratingSpatialIndex<WaterPresentationAssets>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_origin_cells(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let terrain = GeneratingSpatialIndex::<Terrain>::get_one_or_generate(
			spatial_index,
			id,
			lod_ref,
		)?;
		let res_2 = terrain.res_2;
		let terrain_sdf = terrain.sdf.terrain.clone();
		let fills = collect_marazion_lake_fills(spatial_index, bounds, lod_ref)?;
		let fills: Vec<_> = fills
			.into_iter()
			.filter(|fill| fill_has_wet_volume(fill, &terrain_sdf))
			.collect();
		if fills.is_empty() {
			return None;
		}
		let assets = GeneratingSpatialIndex::<WaterPresentationAssets>::get_one_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let sdf = ComposedWater::new(terrain_sdf.clone(), fills.clone());
		Some((
			Self {
				cell: bounds,
				terrain: terrain_sdf,
				fills,
				sdf,
				material: assets.material.clone(),
				res_2,
			},
			bounds,
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
