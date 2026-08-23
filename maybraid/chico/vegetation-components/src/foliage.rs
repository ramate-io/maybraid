//! Foliage domain: style + geometry + placement → [`FoliageNode`].

pub mod ball_collection;
pub mod collection;
pub mod geometry;
pub mod node;
pub mod probe;
pub mod style;

pub use ball_collection::{
	CheapBallCollection, CHEAP_BALL_COLLECTION_HIGH_METERS, CHEAP_BALL_COLLECTION_LOW_METERS,
	CHEAP_BALL_COLLECTION_MEDIUM_METERS,
};
pub use collection::{
	FrondCollection, FrondKit, FrondMember, FrondRun, FROND_COLLECTION_HIGH_FACTOR,
	FROND_COLLECTION_HIGH_METERS, FROND_COLLECTION_LOW_FACTOR, FROND_COLLECTION_LOW_METERS,
	FROND_COLLECTION_MEDIUM_FACTOR, FROND_COLLECTION_MEDIUM_METERS,
};
pub use geometry::FoliageGeometry;
pub use node::FoliageNode;
pub use probe::{
	update_foliage_host_levels, FoliageLodProbe, FOLIAGE_HIGH_FACTOR, FOLIAGE_LOW_FACTOR,
	FOLIAGE_MEDIUM_FACTOR,
};
pub use style::FoliageStyle;
