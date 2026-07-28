//! Roof / cap scene components.
//!
//! IR: [`RoofStyle`] + [`RoofGeometry`] + [`Placement`] → [`RoofNode`] (`LodScene`).

pub mod geometry;
pub mod node;
pub mod rough_stonework;
pub mod style;
pub(crate) mod tessellate;
pub mod wood;

pub use geometry::*;
pub use node::RoofNode;
pub use rough_stonework::*;
pub use style::RoofStyle;
pub use wood::*;
