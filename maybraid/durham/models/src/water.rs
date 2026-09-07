//! Durham water model: compose stamp-owned lake fills and mesh them on the **terrain** lattice.
//!
//! # Parallel to terrain
//!
//! | Layer | Terrain | Water |
//! | --- | --- | --- |
//! | Origin tiling | [`TerrainCellLayout`] | same (`original_ids_for_origin_cells`) |
//! | Composition | [`ComposedTerrain`] / [`Terrain::compose_sdf`] | [`ComposedWater`] / [`ComposedWater::compose`] |
//! | Cascade chunk | [`cascade_chunk_for_cell`] | **same helper**, same `cell` + `res_2` |
//! | Mesh resolution | [`TerrainPresentationAssets::res_2`](crate::terrain::presentation::TerrainPresentationAssets) via the sibling [`Terrain`] cell | inherited from that [`Terrain::res_2`] — never a separate water grid |
//!
//! Marazion stamps author [`WaterFill`]s backed by [`HydroComplex`] (carve ×
//! half-space below \(W\)). This module collects those fills from an already composed
//! [`Terrain`] cell and presents [`ComposedWater`] on that shared sample space.
//!
//! **Order:** [`Terrain`] must compose **all** Marazion watershed bands before
//! fills are evaluated. [`Water`] reads [`Terrain::marazion_fills`] from that
//! finished cell — never by regenerating leaves mid-compose.

pub mod composed;
pub mod plugin;
pub mod presentation;

use crate::terrain::cell::{original_ids_for_origin_cells, TerrainCellLayout, TerrainCellRing};
use crate::terrain::render::cascade_chunk_for_cell;
use crate::terrain::sdf::TerrainSdf;
use crate::terrain::stream_lod::{stream_banded_level, stream_banded_scene, StreamBandedLod};
use crate::terrain::Terrain;
use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use durham_terrain::shaders::RefractionWater;
use lod::gen::{
	GeneratingSpatialIndex, GenerationScheme, Id, LodScene, LodSceneLevel, LodSceneStatus,
	OriginalId,
};
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

/// Cell-level water collector: same origin cell as [`Terrain`], composed fills + mesh.
#[derive(Debug, Clone, Component)]
pub struct Water {
	/// Origin-cell AABB — identical to the sibling [`Terrain`] cell.
	pub cell: Aabb3d,
	/// Composed terrain heightfield used when evaluating fills.
	pub terrain: TerrainSdf,
	/// Stamp-owned lake fills collected on this cell (wet-volume filtered).
	pub fills: Vec<WaterFill>,
	/// Meshable union of fills against [`Self::terrain`] ([`ComposedWater`]).
	pub sdf: ComposedWater,
	pub material: Handle<RefractionWater>,
	/// Cascade `res_2` copied from the sibling [`Terrain`] cell (shared lattice).
	pub res_2: u8,
	/// Moving stream band copied from the source Durham cell.
	pub stream_ring: Option<TerrainCellRing>,
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

	fn center(&self) -> Vec3 {
		(Vec3::from(self.cell.min) + Vec3::from(self.cell.max)) * 0.5
	}

	/// Visual scene for one cell: **same** cascade chunk as [`Terrain::scene`], then
	/// cached [`ComposedWater`] mesh dispatch.
	pub fn scene(&self) -> impl Scene + 'static {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2);
		let transform = Transform::from_translation(chunk.origin);
		let sdf = self.sdf.clone();
		let material = self.material.clone();
		bsn! {
			template_value(transform)
			template_value(chunk)
			template(move |_ctx| Ok(Cached::new(sdf.clone())))
			MeshMaterial3d::<RefractionWater>({material.clone()})
		}
	}
}

impl StreamBandedLod for Water {
	fn stream_ring(&self) -> Option<TerrainCellRing> {
		self.stream_ring
	}

	fn stream_center(&self) -> Vec3 {
		self.center()
	}
}

impl LodScene for Water {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		stream_banded_level(self, lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let previous = stream_banded_level(self, lod_ref.previous_transform);
		let current = stream_banded_level(self, lod_ref.current_transform);
		if previous == current {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(current)
		}
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		stream_banded_scene(self, level, || self.scene())
	}
}

/// Min distance over stamp fills (empty → large positive / dry).
pub fn water_distance(fills: &[WaterFill], p: Vec3, terrain_height: f32) -> f32 {
	fills
		.iter()
		.map(|fill| fill.distance(p, terrain_height))
		.fold(f32::INFINITY, f32::min)
}

/// True when the stamp has wet volume at a representative footprint sample.
///
/// Hydro fills probe node interiors (carve samples), so a lake/stream that
/// misses a single cell-center proxy still keeps its water cell.
fn fill_has_wet_volume(fill: &WaterFill, terrain: &TerrainSdf) -> bool {
	for p in fill.wet_volume_probe_points() {
		let h = terrain.height_at_with_all_modulations(p.x, p.y);
		if fill.wet_y_span_at(p.x, p.y, h).is_some() {
			return true;
		}
	}
	false
}

impl<S> GenerationScheme<S> for Water
where
	S: GeneratingSpatialIndex<Terrain>
		+ GeneratingSpatialIndex<TerrainCellLayout>
		+ GeneratingSpatialIndex<WaterPresentationAssets>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		// Same origin-cell tiling controller as terrain.
		original_ids_for_origin_cells(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		// Terrain composes every Marazion band before returning; fills ride along.
		let terrain =
			GeneratingSpatialIndex::<Terrain>::get_one_or_generate(spatial_index, id, lod_ref)?;
		// Lattice resolution comes from the terrain cell — not a water-only knob.
		let res_2 = terrain.res_2;
		let stream_ring = terrain.stream_ring;
		let terrain_sdf = terrain.sdf.terrain().clone();
		let fills: Vec<_> = terrain
			.marazion_fills
			.iter()
			.cloned()
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
		let sdf = ComposedWater::compose(terrain_sdf.clone(), fills.clone());
		Some((
			Self {
				cell: bounds,
				terrain: terrain_sdf,
				fills,
				sdf,
				material: assets.material.clone(),
				res_2,
				stream_ring,
			},
			bounds,
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::terrain::cell::TERRAIN_CELL_SIZE;
	use crate::terrain::stream_lod::{stream_banded_draws, stream_banded_level};

	fn far_ring() -> TerrainCellRing {
		TerrainCellRing {
			cell_size: 2.0 * TERRAIN_CELL_SIZE,
			res_2: 4,
			anchor_step: 4.0 * TERRAIN_CELL_SIZE,
			high_inner_radius: 8.0 * TERRAIN_CELL_SIZE,
			high_outer_radius: 16.0 * TERRAIN_CELL_SIZE,
			cull_margin: 8.0 * TERRAIN_CELL_SIZE,
		}
	}

	fn water_at(center: Vec3, ring: TerrainCellRing) -> Water {
		let half = ring.cell_size * 0.5;
		let cell = Aabb3d::from_min_max(
			Vec3::new(center.x - half, -1.0, center.z - half),
			Vec3::new(center.x + half, 1.0, center.z + half),
		);
		let terrain = TerrainSdf::new(1, 20.0);
		Water {
			cell,
			sdf: ComposedWater::compose(terrain.clone(), Vec::new()),
			terrain,
			fills: Vec::new(),
			material: Handle::default(),
			res_2: ring.res_2,
			stream_ring: Some(ring),
		}
	}

	#[test]
	fn far_water_underfoot_is_empty_high() {
		let water = water_at(Vec3::ZERO, far_ring());
		let viewer = Transform::IDENTITY;
		let level = stream_banded_level(&water, &viewer);
		assert_eq!(level, LodSceneLevel::High);
		assert!(!stream_banded_draws(&water, level));
	}

	#[test]
	fn far_water_in_ring_is_drawn_medium() {
		let ring = far_ring();
		let water = water_at(Vec3::new(ring.high_inner_radius + 80.0, 0.0, 0.0), ring);
		let viewer = Transform::IDENTITY;
		let level = stream_banded_level(&water, &viewer);
		assert_eq!(level, LodSceneLevel::Medium);
		assert!(stream_banded_draws(&water, level));
	}
}
