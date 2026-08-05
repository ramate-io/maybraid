//! Foliage domain: style + geometry + placement → [`FoliageNode`].

mod geometry;
mod node;
mod probe;
mod style;

pub use geometry::FoliageGeometry;
pub use node::FoliageNode;
pub use probe::{
	update_foliage_host_levels, FoliageLodProbe, FOLIAGE_HIGH_FACTOR, FOLIAGE_LOW_FACTOR,
	FOLIAGE_MEDIUM_FACTOR,
};
pub use style::FoliageStyle;
