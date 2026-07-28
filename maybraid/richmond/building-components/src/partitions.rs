//! Wall / partition scene components (linear, angular, and header variants).
//!
//! IR: [`WallStyle`] + [`WallGeometry`] + [`Placement`] → [`WallNode`] (`LodScene`).

pub mod geometry;
pub mod node;
pub mod rough_stonework;
pub mod style;
pub(crate) mod tessellate;

pub use geometry::*;
pub use node::WallNode;
pub use rough_stonework::*;
pub use style::WallStyle;
