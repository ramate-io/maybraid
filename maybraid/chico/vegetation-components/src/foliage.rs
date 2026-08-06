//! Foliage domain: style + geometry + placement → [`FoliageNode`].

pub mod collection;
pub mod geometry;
pub mod node;
pub mod probe;
pub mod style;

pub use collection::{
	FrondCollection, FrondKit, FrondMember, FROND_COLLECTION_HIGH_FACTOR,
	FROND_COLLECTION_LOW_FACTOR, FROND_COLLECTION_MEDIUM_FACTOR,
};
pub use geometry::FoliageGeometry;
pub use node::FoliageNode;
pub use probe::{
	update_foliage_host_levels, FoliageLodProbe, FOLIAGE_HIGH_FACTOR, FOLIAGE_LOW_FACTOR,
	FOLIAGE_MEDIUM_FACTOR,
};
pub use style::FoliageStyle;
