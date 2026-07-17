//! Uniform controller cells that own guillotine cuts for ValleyChain leaves.

use crate::terrain::valley_chain::config::JerseyValleyChainLayerConfig;
use crate::terrain::valley_chain::layout::{
	original_ids_for_controller_cells, JerseyValleyChainControllerLayout,
};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::*;
use comproc::guillotine::{Bounds2, Guillotine, GuillotineCuts};
use comproc::noise::config::NoiseConfig;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId};
use lod::lod_ref::LodRef;
use noise::Perlin;

fn cell_salt(cell: Aabb3d) -> u32 {
	let ix = cell.min.x.to_bits();
	let iz = cell.min.z.to_bits();
	ix.wrapping_mul(73856093) ^ iz.wrapping_mul(19349663)
}

fn root_bounds2(cell: Aabb3d) -> Bounds2 {
	Bounds2::from_vec2(
		Vec2::new(cell.min.x, cell.min.z),
		Vec2::new(cell.max.x, cell.max.z),
	)
}

/// One controller cell: uniform grid tile with cached guillotine cuts.
#[derive(Debug, Clone, Component)]
pub struct JerseyValleyChainControllerCell {
	pub cell: Aabb3d,
	pub cuts: GuillotineCuts<2>,
}

impl JerseyValleyChainControllerCell {
	pub fn from_config(cell: Aabb3d, config: &JerseyValleyChainLayerConfig) -> Self {
		let seed = config.seed.wrapping_add(cell_salt(cell));
		let noise = NoiseConfig::new(Perlin::default())
			.with_seed(seed)
			.with_frequency(config.noise_frequency)
			.with_amplitude(1.0)
			.with_octaves(1);
		let cutter = Guillotine::new(noise, config.guillotine, config.depth);
		let root = root_bounds2(cell);
		let cuts = cutter.cut(root);
		Self { cell, cuts }
	}

	/// Leaf AABBs using this controller's vertical extent.
	pub fn leaf_aabbs(&self) -> Vec<Aabb3d> {
		let vy_min = self.cell.min.y;
		let vy_max = self.cell.max.y;
		self.cuts
			.regions()
			.map(|leaf| {
				Aabb3d::from_min_max(
					Vec3::new(leaf.min[0], vy_min, leaf.min[1]),
					Vec3::new(leaf.max[0], vy_max, leaf.max[1]),
				)
			})
			.collect()
	}
}

impl<S> GenerationScheme<S> for JerseyValleyChainControllerCell
where
	S: GeneratingSpatialIndex<JerseyValleyChainLayerConfig>
		+ GeneratingSpatialIndex<JerseyValleyChainControllerLayout>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_controller_cells(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let config = GeneratingSpatialIndex::<JerseyValleyChainLayerConfig>::get_one_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?
		.clone();
		Some((Self::from_config(bounds, &config), bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
