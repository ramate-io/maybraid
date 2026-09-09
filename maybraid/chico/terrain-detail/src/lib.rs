//! Chico terrain-detail stack for rocks ([#785](https://github.com/ramate-io/maybraid/issues/785)).
//!
//! Same shape as forests and groves: unit mesh **components**, 40 m **outcropping**
//! recipes, 400 m **formation** throws, and a select-only [`TerrainDetail`] parent.
//! Grow on the outcropping origin. Do not remarch Durham. Do not implement
//! `LodScene` on [`TerrainDetail`].

mod component;
mod extent;
mod formation;
mod generation;
mod index;
mod outcropping;
mod plugin;
mod present;
mod sample;
mod stream;
mod terrain_detail;

pub use component::assets;
pub use component::{rock_material, RockComponent};
pub use extent::{
	FormationExtent, OutcroppingExtent, DEFAULT_FORMATION_EXTENT_XZ, DEFAULT_OUTCROPPING_EXTENT_XZ,
};
pub use formation::FormationKind;
pub use generation::{
	TerrainDetailGenerateBullseye, TerrainDetailLodChan, TerrainDetailPresentBullseye,
	TERRAIN_DETAIL_GENERATE_RADIUS_M, TERRAIN_DETAIL_PRESENT_RADIUS_M,
};
pub use index::TerrainDetailIndex;
pub use outcropping::{OutcroppingKind, RockPlacement, TerrainOutcropping};
pub use plugin::{register_terrain_detail_lod, TerrainDetailPlugin};
pub use present::{
	init_rock_mesh_cache, spawn_rock, FlatTerrainDetailPresenter, RockMeshCache,
	TerrainDetailPresenterState,
};
pub use sample::{FlatTerrainDetailSample, TerrainDetailWorldSample};
pub use stream::{
	parse_formation_kind, stream_radii_m, TerrainDetailStreamLod, TerrainDetailStreamSpec,
	DEFAULT_TERRAIN_DETAIL_NOISE, DEFAULT_TERRAIN_DETAIL_STREAM_RADIUS,
};
pub use terrain_detail::TerrainDetail;
