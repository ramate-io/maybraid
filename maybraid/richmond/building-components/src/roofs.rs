//! Roof / cap scene components.
//!
//! IR: [`RoofStyle`] + [`RoofGeometry`] + [`Placement`] → [`RoofNode`] (`LodScene`).

pub mod geometry;
pub mod node;
pub mod shepherds_thatch;
pub mod style;
pub(crate) mod tessellate;

pub use geometry::*;
pub use node::RoofNode;
pub use shepherds_thatch::*;
pub use style::RoofStyle;
