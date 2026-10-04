//! Geneva: tiled languages and place names over a vegetated world.
//!
//! Large tiles compose grammar and lexicon bundles. Guillotine small tiles pick
//! a subset. Names translate English kind labels in the chosen language.
//! [`Geneva<W>`] does not change the ground.

mod bundle;
mod catalog;
mod english;
mod index;
mod model;
mod name;
mod present;
mod sources;
mod stream;
mod tiles;

pub use bundle::{LanguageBundle, LexiconFamily};
pub use catalog::KindConceptUniverse;
pub use english::{
	color_term, development_terms, geographic_terms, grove_kind_terms, layering_terms,
	place_label_terms, urbanization_terms, with_color_name, PLACE_COLORS,
};
pub use index::{LanguageConfig, LanguageIndex, LanguageWorldSeed, NameKey};
pub use model::Geneva;
pub use name::{AssignedName, PlaceName};
pub use present::{LanguageLodChan, LanguageOverlay, NamedOverlay};
pub use sources::{NamedFeature, NamedPlace, NamedWorld};
pub use stream::install_language_stream;
pub use tiles::{
	large_tile_index, large_tile_origin, LargeTile, SmallTile, LARGE_TILE, SMALL_STEP_MAX,
	SMALL_STEP_MIN,
};

#[cfg(test)]
mod tests;
