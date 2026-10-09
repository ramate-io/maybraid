//! Geneva: tiled languages and place names over a vegetated world.
//!
//! Large tiles compose grammar and lexicon bundles from seed and coordinates.
//! Guillotine small tiles pick a subset. Nearby features and POIs are named
//! inside a local window by translating English kind labels in the chosen
//! language. Geneva does not change the ground; [`GenevaPlugin`] runs it on
//! the shared HCSG runtime.

mod bundle;
mod catalog;
mod english;
mod index;
mod name;
mod present;
pub mod shared;
mod sources;
mod tiles;

pub use bundle::{LanguageBundle, LexiconFamily};
pub use catalog::KindConceptUniverse;
pub use english::{
	color_term, compose_english, development_terms, geographic_terms, grove_kind_terms,
	layering_terms, named_forest_english, named_geographic_english, named_grove_english,
	named_place_english, named_region_english, named_urban_english, place_label_terms,
	urbanization_terms, with_color_name, PLACE_COLORS,
};
pub use index::{
	AssignResult, LanguageConfig, LanguageIndex, LanguageSourceDeps, LanguageWorldSeed, NameKey,
};
pub use name::{AssignedName, PlaceName};
pub use present::{LanguageOverlay, LargeTileOverlay, NamedOverlay};
pub use shared::{GenevaNodes, GenevaPlugin, GenevaRoots, LanguageNeighborhood};
pub use sources::{
	FeatureSnapshot, NameSources, NamedFeature, NamedPlace, NamingRegion, PlaceSnapshot,
	PoseUpdate, SourceRevisions, NAME_WINDOW_QUANT_M,
};
pub use tiles::{
	large_tile_aabb, large_tile_index, large_tile_origin, LargeTile, SmallTile, LARGE_TILE,
	SMALL_STEP_MAX, SMALL_STEP_MIN,
};

#[cfg(test)]
mod tests;
