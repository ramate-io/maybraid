//! Stick domain: style + geometry + placement → [`StickNode`].

pub mod geometry;
pub mod node;
pub mod probe;
pub mod style;

pub use geometry::StickGeometry;
pub use node::StickNode;
pub use probe::{
	update_stick_host_levels, StickLodProbe, STICK_HIGH_FACTOR, STICK_LOW_FACTOR, STICK_MEDIUM_FACTOR,
};
pub use style::StickStyle;

pub use crate::procedural::STICK_KIT_HALF;
