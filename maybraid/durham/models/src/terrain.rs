//! Durham terrain model: SDF composition, LOD generation, Avian index, render.

pub mod base_noise;
pub mod cell;
pub mod cell_noise;
pub mod collider;
pub mod config;
pub mod index;
pub mod jersey_configs;
pub mod jersey_layers;
pub mod jersey_modulation;
pub mod plugin;
pub mod presentation;
pub mod render;
pub mod sdf;
pub mod valley_chain;

use crate::terrain::cell::{original_ids_for_jersey_cells, original_ids_for_origin_cells};
use crate::terrain::render::cascade_chunk_for_cell;
use crate::terrain::valley_chain::{
	original_ids_for_guillotine_leaves, JerseyValleyChainControllerCell,
	JerseyValleyChainControllerLayout, JerseyValleyChainLayerConfig, JerseyValleyChainStampCell,
};
use avian3d::prelude::RigidBody;
use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use durham_terrain::shaders::DurhamTerrainShader;
use jersey_terrain_stamps::JerseyModulation;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, LodScene, OriginalId};
use lod::lod_ref::LodRef;
use render_item::mesh::handle::Cached;

pub use base_noise::BaseTerrainNoise;
pub use cell::{
	JerseyStampCellLayout, MacroCellLayout, TerrainCellLayout, JERSEY_STAMP_CELL_SIZE,
	JERSEY_STAMP_GRID_OFFSET, MACRO_CELL_SIZE, TERRAIN_CELL_SIZE,
};
pub use cell_noise::CellTerrainNoise;
pub use collider::TerrainTrimeshCollider;
pub use config::TerrainConfig;
pub use index::{AvianTerrainIndex, TerrainCellId, TerrainEntryStore};
pub use jersey_configs::JerseyLayerConfigs;
pub use jersey_layers::{
	CanyonLayer, PlateauCapLayer, PocketWaterLayer, RollingGroundLayer, RuggedMassifLayer,
};
pub use plugin::{register_terrain_plugin, TerrainPlugin};
pub use presentation::{
	TerrainPresentationAssets, TerrainPresenterState, TerrainRegionPresenter, TerrainStoreView,
};
pub use render::TerrainRenderItem;
pub use sdf::{ComposedTerrain, ElevationModulation, TerrainSdf};
pub use valley_chain::{
	JerseyValleyChainControllerCell as ValleyChainControllerCell,
	JerseyValleyChainControllerLayout as ValleyChainControllerLayout,
	JerseyValleyChainGuillotineCell as ValleyChainGuillotineCell,
	JerseyValleyChainLayerConfig as ValleyChainLayerConfig,
	JerseyValleyChainStampCell as ValleyChainStampCell, VALLEY_CHAIN_CONTROLLER_CELL_SIZE,
};

/// Top-level terrain cell model.
///
/// Built by pulling intersecting generation deps from the spatial index, cloning
/// them in, and composing a per-cell SDF for sampling / presentation.
#[derive(Debug, Clone, Component)]
pub struct Terrain {
	pub cell: Aabb3d,
	pub base: BaseTerrainNoise,
	/// Flattened jersey + ValleyChain modulations (deterministic source order).
	pub modulations: Vec<JerseyModulation>,
	/// ValleyChain leaf AABBs whose stamps contributed (debug / HUD).
	pub valley_leaves: Vec<Aabb3d>,
	pub sdf: ComposedTerrain,
	pub material: Handle<DurhamTerrainShader>,
	pub res_2: u8,
}

impl Terrain {
	/// Compose an SDF from cloned base noise and flattened modulations.
	pub fn compose_sdf(
		base: &BaseTerrainNoise,
		modulations: &[JerseyModulation],
	) -> ComposedTerrain {
		let mut sdf = base.sdf.clone();
		for modulation in modulations {
			sdf.add_elevation_modulation(Box::new(modulation.clone()));
		}
		ComposedTerrain::from_terrain(sdf)
	}

	/// Visual scene for one cell: cascade chunk + cached SDF mesh dispatch.
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

/// Pull coexist jersey stamp families at `id` (ValleyBasin replaced by ValleyChain).
fn append_jersey_families_at_cell<S>(
	spatial_index: &mut S,
	id: Id,
	lod_ref: &LodRef,
	out: &mut Vec<JerseyModulation>,
) -> Option<()>
where
	S: GeneratingSpatialIndex<PlateauCapLayer>
		+ GeneratingSpatialIndex<RuggedMassifLayer>
		+ GeneratingSpatialIndex<CanyonLayer>
		+ GeneratingSpatialIndex<PocketWaterLayer>
		+ GeneratingSpatialIndex<RollingGroundLayer>,
{
	out.extend(
		GeneratingSpatialIndex::<PlateauCapLayer>::get_one_or_generate(spatial_index, id, lod_ref)?
			.modulations
			.iter()
			.cloned(),
	);
	out.extend(
		GeneratingSpatialIndex::<RuggedMassifLayer>::get_one_or_generate(
			spatial_index,
			id,
			lod_ref,
		)?
		.modulations
		.iter()
		.cloned(),
	);
	out.extend(
		GeneratingSpatialIndex::<CanyonLayer>::get_one_or_generate(spatial_index, id, lod_ref)?
			.modulations
			.iter()
			.cloned(),
	);
	out.extend(
		GeneratingSpatialIndex::<PocketWaterLayer>::get_one_or_generate(
			spatial_index,
			id,
			lod_ref,
		)?
		.modulations
		.iter()
		.cloned(),
	);
	out.extend(
		GeneratingSpatialIndex::<RollingGroundLayer>::get_one_or_generate(
			spatial_index,
			id,
			lod_ref,
		)?
		.modulations
		.iter()
		.cloned(),
	);

	Some(())
}

/// Terrain loads base noise, jersey stamp families, and ValleyChain leaf stamps directly.
impl<S> GenerationScheme<S> for Terrain
where
	S: GeneratingSpatialIndex<BaseTerrainNoise>
		+ GeneratingSpatialIndex<PlateauCapLayer>
		+ GeneratingSpatialIndex<RuggedMassifLayer>
		+ GeneratingSpatialIndex<CanyonLayer>
		+ GeneratingSpatialIndex<PocketWaterLayer>
		+ GeneratingSpatialIndex<RollingGroundLayer>
		+ GeneratingSpatialIndex<JerseyValleyChainStampCell>
		+ GeneratingSpatialIndex<JerseyValleyChainControllerCell>
		+ GeneratingSpatialIndex<JerseyValleyChainLayerConfig>
		+ GeneratingSpatialIndex<JerseyValleyChainControllerLayout>
		+ GeneratingSpatialIndex<JerseyStampCellLayout>
		+ GeneratingSpatialIndex<TerrainCellLayout>
		+ GeneratingSpatialIndex<TerrainPresentationAssets>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_origin_cells(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;

		let base = GeneratingSpatialIndex::<BaseTerrainNoise>::get_one_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?
		.clone();

		let mut modulations = Vec::new();

		// Jersey stamp grid families first (sorted cell Id), then ValleyChain leaves.
		// Keep Id order when composing so neighboring Terrain cells apply
		// non-commutative jersey ops identically.
		let mut jersey_ids = original_ids_for_jersey_cells(spatial_index, bounds);
		jersey_ids.sort_by(|a, b| a.0.cmp(&b.0));
		for OriginalId(jid) in jersey_ids {
			append_jersey_families_at_cell(spatial_index, jid, lod_ref, &mut modulations)?;
		}

		let mut leaf_ids = original_ids_for_guillotine_leaves(spatial_index, bounds);
		leaf_ids.sort_by(|a, b| a.0.cmp(&b.0));
		let mut valley_leaves = Vec::new();
		for OriginalId(lid) in leaf_ids {
			let stamp = GeneratingSpatialIndex::<JerseyValleyChainStampCell>::get_one_or_generate(
				spatial_index,
				lid,
				lod_ref,
			)?;
			valley_leaves.push(stamp.cell);
			modulations.extend(stamp.modulations.iter().cloned());
		}

		let sdf = Self::compose_sdf(&base, &modulations);
		let assets = GeneratingSpatialIndex::<TerrainPresentationAssets>::get_one_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let material = assets.material.clone();
		let res_2 = assets.res_2;

		Some((
			Self {
				cell: bounds,
				base,
				modulations,
				valley_leaves,
				sdf,
				material,
				res_2,
			},
			bounds,
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
