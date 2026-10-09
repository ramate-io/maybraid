//! Cellular forests for Chico vegetation ([RFC-183 §3.5]).
//!
//! A forest cell is 1600 m. Hopscotch picks a well-known layering. Each layer
//! Bucket-Throws a grove or `None`. Generate queries [`ChicoGrove`] (100 m ×
//! layer); [`ChicoForest`] is select-only. Ground cover is omitted. Missing
//! groves (Conifer Lower Massives) are dropped, not aliased.
//! `ForestGroveBiases` stay default — forests do not bias construction seeds.

mod assemble;
mod blend;
mod bump_out;
mod chico;
mod config;
mod extent;
mod forest;
mod generation;
mod ground;
mod grove;
pub(crate) mod hopscotch;
mod host;
mod index;
mod kind;
mod layer;
mod layer_present;
mod layer_stream;
pub mod layerings;
mod material;
mod model;
mod packed;
mod plugin;
mod present;
mod recipe;
mod stick_physics;
mod stream;
mod view;

pub use assemble::{
	assemble, assemble_isolated, grow_tile, presenting_recipes, AssembledForest, ForestGroveTile,
	NeighborLayers,
};
pub use blend::{
	GROVE_BLEND_INFLUENCE, GROVE_BLEND_NOISE, GROVE_BLEND_RADIUS, GROVE_BLEND_TEMPERATURE,
};
pub use bump_out::{
	blend_selection_neighborhood, blend_selection_on_bounds, bump_out_cell_bounds,
	bump_out_cells_overlapping, bump_out_chebyshev_xz, bump_out_in_inner_hole,
	medium_bump_out_in_band, selection_sample_at, BumpOutSelection, BumpOutSelectionSample,
	CanopyBumpOut, MediumCanopyBumpOut, BUMP_OUT_CELL_XZ, BUMP_OUT_INNER_RADIUS_M,
	BUMP_OUT_OUTER_RADIUS_M, MEDIUM_BUMP_OUT_ANCHOR_STEP_M, MEDIUM_BUMP_OUT_CELL_XZ,
	MEDIUM_BUMP_OUT_INNER_RADIUS_M, MEDIUM_BUMP_OUT_OUTER_RADIUS_M,
};
pub use chico::{chico_hopscotch, select_cell, select_layering, DEFAULT_HOP_BUDGET};
pub use config::{ChicoConfig, ForestStreamSpec};
pub use extent::{ForestExtent, DEFAULT_FOREST_EXTENT_XZ, DEFAULT_FOREST_GROVE_TILE_XZ};
pub use forest::{neighbor_layers, ChicoForest};
pub use generation::{
	BumpOutGenerateBullseye, BumpOutLodChan, BumpOutPresentBullseye, ForestGenerateBullseye,
	ForestLodChan, ForestPresentBullseye, ForestPresentLattice, MediumBumpOutLodChan,
	GROVE_GENERATE_RADIUS_M, GROVE_PRESENT_RADIUS_M,
};
pub use ground::{overlay_chunk_ref, ChicoGround};
pub use grove::{grove_from_id, grove_id, ChicoGrove};
pub use hopscotch::{select as hopscotch_select, HopscotchNode};
pub use host::ChicoGroveHost;
pub use index::{forest_world_sample, ForestIndex};
pub use kind::{
	ForestGroveKind, ForestLayer, ForestLayering, LayerDropOut, LayeringKind, SelectedLayers,
	WeightedGrove, TUFT_DROP_MIN_HEIGHT_M,
};
pub use layer::{select_layers, throw_layer};
pub use layer_present::{
	bump_out_from_cell, bump_out_noise, GroundCanopyBumpOutPresenter, GroundForestPresenter,
	GroundGroveSample, GroundMediumCanopyBumpOutPresenter,
};
pub use layer_stream::{
	install_vegetation_stream, stream_radii_m, stream_vegetation, VegetationStreamKey,
	DEFAULT_FOREST_NOISE, DEFAULT_FOREST_STREAM_RADIUS,
};
pub use material::{VegetationOnTerrainMaterialLib, VegetationOnTerrainMaterialRefPlugin};
pub use model::Chico;
pub use plugin::{register_vegetation_view, VegetationViewPlugin};
pub use present::ForestPresenterState;
pub use recipe::{ForestGroveRecipe, WORLD_FOREST_TREE_VARIANTS};
pub use stream::parse_layering_kind;

#[cfg(test)]
mod layer_tests;
#[cfg(test)]
mod present_tests;
pub use view::{
	VegetationBullseye, VegetationCull, VegetationLodRefreshPlugin, VegetationRefresh,
	VegetationSpotlight,
};
