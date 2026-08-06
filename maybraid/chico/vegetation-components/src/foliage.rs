//! Foliage domain: style + geometry + placement → [`FoliageNode`].

pub mod geometry;
pub mod node;
pub mod probe;
pub mod style;

pub use geometry::FoliageGeometry;
pub use node::FoliageNode;
pub use probe::{
	update_foliage_host_levels, FoliageLodProbe, FOLIAGE_HIGH_FACTOR, FOLIAGE_LOW_FACTOR,
	FOLIAGE_MEDIUM_FACTOR,
};
pub use style::FoliageStyle;
